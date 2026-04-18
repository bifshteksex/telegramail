use std::time::Duration;

use lettre::message::{Mailbox, SinglePart, header};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tauri::{AppHandle, Manager};
use tokio::time::timeout;

use super::{db, pgp, storage, SmtpResult};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

fn build_transport(
  host: &str,
  port: u16,
  email: &str,
  password: &str,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
  let creds = Credentials::new(email.to_owned(), password.to_owned());
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

fn build_handshake_message(
  from_email: &str,
  to_email: &str,
  display_name: &str,
  kind: &str,
  own_public_key: Option<&str>,
) -> Result<Message, String> {
  let from: Mailbox = from_email.parse().map_err(|e| format!("Invalid from: {e}"))?;
  let to: Mailbox = to_email.parse().map_err(|e| format!("Invalid to: {e}"))?;

  let mut builder = Message::builder()
    .from(from)
    .to(to)
    .subject("Telegramail Contact Request")
    .raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("Chat-Version"),
      "1.0".to_owned(),
    ))
    .raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("X-Telegramail-Handshake"),
      kind.to_owned(),
    ))
    .raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("X-Telegramail-Display-Name"),
      display_name.to_owned(),
    ));

  if let Some(pub_key) = own_public_key {
    if let Ok(autocrypt) = pgp::build_autocrypt_header(from_email, pub_key) {
      builder = builder.raw_header(header::HeaderValue::new(
        header::HeaderName::new_from_ascii_str("Autocrypt"),
        autocrypt,
      ));
    }
  }

  let body = format!(
    "{display_name} would like to connect with you via Telegramail.\n\
     \n\
     To accept, open Telegramail and confirm this contact request.\n\
     \n\
     This message was sent automatically by the Telegramail app.",
  );

  builder
    .singlepart(SinglePart::plain(body))
    .map_err(|e| format!("Failed to build message: {e}"))
}

async fn send_handshake(
  app: &AppHandle,
  own_email: &str,
  smtp_host: &str,
  display_name: &str,
  to_email: &str,
  kind: &str,
) -> SmtpResult {
  let password = match storage::load_smtp_password(own_email) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(format!("Keychain error: {e}")),
  };

  let port = match storage::load_smtp_port(own_email) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(format!("Port not saved: {e}")),
  };

  let db_path = app
    .path()
    .app_data_dir()
    .ok()
    .map(|d| d.join("telegramail.db").to_string_lossy().into_owned());

  let own_public_key = db_path
    .as_deref()
    .and_then(|p| db::load_peer_key(p, &format!("self:{own_email}")).ok().flatten());

  let message = match build_handshake_message(
    own_email,
    to_email,
    display_name,
    kind,
    own_public_key.as_deref(),
  ) {
    Ok(m) => m,
    Err(e) => return SmtpResult::err(e),
  };

  let transport = match build_transport(smtp_host, port, own_email, &password) {
    Ok(t) => t,
    Err(e) => return SmtpResult::err(e),
  };

  match timeout(CONNECT_TIMEOUT, transport.send(message)).await {
    Err(_) => SmtpResult::err("Send timed out"),
    Ok(Ok(_)) => SmtpResult::ok(),
    Ok(Err(e)) => SmtpResult::err(format!("{e}")),
  }
}

/// Send a contact request to a new email address.
#[tauri::command]
pub async fn handshake_send_request(
  app: AppHandle,
  own_email: String,
  smtp_host: String,
  display_name: String,
  to_email: String,
) -> SmtpResult {
  send_handshake(&app, &own_email, &smtp_host, &display_name, &to_email, "request").await
}

/// Accept an incoming contact request — sends a reply handshake.
#[tauri::command]
pub async fn handshake_accept(
  app: AppHandle,
  own_email: String,
  smtp_host: String,
  display_name: String,
  to_email: String,
) -> SmtpResult {
  send_handshake(&app, &own_email, &smtp_host, &display_name, &to_email, "accept").await
}
