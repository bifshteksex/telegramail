use std::time::Duration;

use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use tokio::time::timeout;

use tauri::{AppHandle, Manager};

use super::{db, mime, storage, SmtpResult};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(serde::Serialize)]
pub struct SmtpCheckResult {
  pub ok: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub port: Option<u16>,
  // True when every port timed out — signals VPN blocking at network level
  #[serde(skip_serializing_if = "std::ops::Not::not")]
  pub all_ports_timed_out: bool,
}

impl SmtpCheckResult {
  fn ok(port: u16) -> Self {
    Self { ok: true, error: None, port: Some(port), all_ports_timed_out: false }
  }
  fn err(msg: impl Into<String>) -> Self {
    Self { ok: false, error: Some(msg.into()), port: None, all_ports_timed_out: false }
  }
  fn all_timed_out() -> Self {
    Self {
      ok: false,
      error: Some("All ports timed out".into()),
      port: None,
      all_ports_timed_out: true,
    }
  }
}

fn build_transport(
  host: &str,
  port: u16,
  email: &str,
  password: &str,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
  let creds = Credentials::new(email.to_owned(), password.to_owned());

  // native-tls uses the OS certificate store (Windows SChannel / macOS SecureTransport).
  // This is required for VPN clients that inject their own CA into the system store —
  // rustls uses a bundled webpki root list and never sees those CAs.
  let tls_params = TlsParameters::builder(host.to_owned())
    .build_native()
    .map_err(|e| format!("TLS error: {e}"))?;

  let transport = if port == 465 {
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
      .port(port)
      .tls(Tls::Wrapper(tls_params))
      .credentials(creds)
      .timeout(Some(CONNECT_TIMEOUT))
      .build()
  } else {
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
      .port(port)
      .tls(Tls::Required(tls_params))
      .credentials(creds)
      .timeout(Some(CONNECT_TIMEOUT))
      .build()
  };

  Ok(transport)
}

// Try each port in sequence, return the first that succeeds.
// This lets the app work even when a VPN blocks specific ports (e.g. 465/SMTPS).
#[tauri::command]
pub async fn smtp_check(
  host: String,
  ports: Vec<u16>,
  email: String,
  password: String,
) -> SmtpCheckResult {
  if ports.is_empty() {
    return SmtpCheckResult::err("No ports provided");
  }

  let mut last_error = String::new();
  let mut timed_out_count = 0usize;
  let total_ports = ports.len();

  for port in ports {
    let transport = match build_transport(&host, port, &email, &password) {
      Ok(t) => t,
      Err(e) => {
        last_error = e;
        continue;
      }
    };

    let result = timeout(CONNECT_TIMEOUT, transport.test_connection()).await;
    log::info!("[smtp_check] host={host} port={port} result={result:?}");

    match result {
      Ok(Ok(true)) => {
        if let Err(e) = storage::save_smtp_port(&email, port) {
          return SmtpCheckResult::err(format!("Connected on port {port} but failed to save port: {e}"));
        }
        return SmtpCheckResult::ok(port);
      }
      Ok(Ok(false)) => {
        last_error = format!("Port {port}: server did not accept the connection");
      }
      Ok(Err(e)) => {
        last_error = format!("Port {port}: {e}");
      }
      Err(_) => {
        timed_out_count += 1;
        last_error = format!("Port {port}: connection timed out after {}s", CONNECT_TIMEOUT.as_secs());
      }
    }
  }

  if timed_out_count == total_ports {
    return SmtpCheckResult::all_timed_out();
  }

  SmtpCheckResult::err(last_error)
}

#[tauri::command]
pub async fn smtp_send(
  app: AppHandle,
  host: String,
  email: String,
  to_email: String,
  chat_name: String,
  chat_id: String,
  local_ref: String,
  text: String,
  in_reply_to: Option<String>,
) -> SmtpResult {
  let password = match storage::load_smtp_password(&email) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(format!("Keychain error (password): {e}")),
  };

  let port = match storage::load_smtp_port(&email) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(format!("Keychain error (port): {e}")),
  };

  let transport = match build_transport(&host, port, &email, &password) {
    Ok(t) => t,
    Err(e) => return SmtpResult::err(e),
  };

  // Load own public key (for Autocrypt header).
  let db_path = app
    .path()
    .app_data_dir()
    .ok()
    .map(|d| d.join("telegramail.db").to_string_lossy().into_owned());

  let own_public_key = db_path
    .as_deref()
    .and_then(|p| db::load_peer_key(p, &format!("self:{email}")).ok().flatten());

  // Load recipient public key (for encryption), if different from self.
  let recipient_public_key = if to_email != email {
    db_path
      .as_deref()
      .and_then(|p| db::load_peer_key(p, &to_email).ok().flatten())
  } else {
    None
  };

  // Warn in log when sending unencrypted to a non-self address.
  if to_email != email && recipient_public_key.is_none() {
    log::warn!("[smtp_send] no PGP key for {to_email} — sending unencrypted");
  }

  let message = match mime::build_chat_message(&mime::ChatMailParams {
    from_email: &email,
    to_email: &to_email,
    chat_name: &chat_name,
    text: &text,
    chat_id: &chat_id,
    local_ref: &local_ref,
    in_reply_to: in_reply_to.as_deref(),
    own_public_key: own_public_key.as_deref(),
    recipient_public_key: recipient_public_key.as_deref(),
  }) {
    Ok(m) => m,
    Err(e) => return SmtpResult::err(e),
  };

  match timeout(CONNECT_TIMEOUT, transport.send(message)).await {
    Err(_) => SmtpResult::err("Send timed out (10s)"),
    Ok(Ok(_)) => SmtpResult::ok(),
    Ok(Err(e)) => SmtpResult::err(format!("{e}")),
  }
}
