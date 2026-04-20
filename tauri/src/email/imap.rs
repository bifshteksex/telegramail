use std::time::Duration;

use async_imap::extensions::idle::IdleResponse;
use async_native_tls::{TlsConnector, TlsStream};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio::time::sleep;

use super::events::{EmailErrorEvent, EmailHandshakeEvent, EmailMessageEvent, EmailStatus, EmailStatusEvent};
use super::{db, pgp};

type ImapSession = async_imap::Session<TlsStream<TcpStream>>;

const INITIAL_BACKOFF: Duration = Duration::from_secs(5);
const MAX_BACKOFF: Duration = Duration::from_secs(300);
// RFC 2177 recommends re-issuing IDLE at least every 29 minutes.
const IDLE_TIMEOUT: Duration = Duration::from_secs(25 * 60);

pub type StopTx = watch::Sender<bool>;

pub static STOP_TX: std::sync::Mutex<Option<StopTx>> = std::sync::Mutex::new(None);

pub fn start(app: AppHandle, email: String, password: String, host: String, port: u16) {
  let mut guard = STOP_TX.lock().unwrap();
  if guard.is_some() {
    return; // already running
  }
  let (stop_tx, stop_rx) = watch::channel(false);
  *guard = Some(stop_tx);
  drop(guard);

  tokio::spawn(async move {
    run_watcher(app, email, password, host, port, stop_rx).await;
  });
}

pub fn stop() {
  if let Some(tx) = STOP_TX.lock().unwrap().take() {
    let _ = tx.send(true);
  }
}

// ── Watcher loop ──────────────────────────────────────────────────────────────

async fn run_watcher(
  app: AppHandle,
  email: String,
  password: String,
  host: String,
  port: u16,
  mut stop_rx: watch::Receiver<bool>,
) {
  let mut backoff = INITIAL_BACKOFF;

  loop {
    if *stop_rx.borrow() {
      break;
    }

    match watch_inbox(&app, &email, &password, &host, port, &mut stop_rx).await {
      Ok(()) => {
        break; // clean stop
      }
      Err(e) => {
        log::warn!("[imap] error: {e} — reconnecting in {backoff:?}");
        emit_status(&app, EmailStatus::Reconnecting, Some(e.clone()));
        emit_error(&app, e);

        tokio::select! {
          _ = sleep(backoff) => {}
          _ = stop_rx.changed() => { break; }
        }

        backoff = (backoff * 2).min(MAX_BACKOFF);
      }
    }
  }

  emit_status(&app, EmailStatus::Disconnected, None);
  STOP_TX.lock().unwrap().take();
}

async fn connect_session(
  email: &str,
  password: &str,
  host: &str,
  port: u16,
) -> Result<ImapSession, String> {
  let tcp = TcpStream::connect((host, port))
    .await
    .map_err(|e| format!("TCP connect {host}:{port}: {e}"))?;

  let tls = TlsConnector::new();
  let tls_stream = tls
    .connect(host, tcp)
    .await
    .map_err(|e| format!("TLS handshake: {e}"))?;

  let client = async_imap::Client::new(tls_stream);
  let session = client
    .login(email, password)
    .await
    .map_err(|(e, _)| format!("IMAP login: {e}"))?;

  Ok(session)
}

async fn watch_inbox(
  app: &AppHandle,
  email: &str,
  password: &str,
  host: &str,
  port: u16,
  stop_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
  let mut session = connect_session(email, password, host, port).await?;

  emit_status(app, EmailStatus::Connected, None);

  session
    .select("INBOX")
    .await
    .map_err(|e| format!("SELECT INBOX: {e}"))?;

  // Fetch any unseen messages that arrived before we connected.
  fetch_unseen(app, &mut session, email).await?;

  // IDLE loop.
  loop {
    if *stop_rx.borrow() {
      let _ = session.logout().await;
      return Ok(());
    }

    let mut idle_handle = session.idle();
    idle_handle
      .init()
      .await
      .map_err(|e| format!("IDLE init: {e}"))?;

    // wait_with_timeout returns (future, stop_source).
    // Dropping stop_source signals ManualInterrupt to the future.
    // We must let idle_future fully complete before calling idle_handle.done(),
    // because the future holds a mutable borrow on idle_handle.
    let should_stop;
    {
      let (idle_future, stop_source) = idle_handle.wait_with_timeout(IDLE_TIMEOUT);
      tokio::pin!(idle_future);

      // Race the IDLE future against the stop signal.
      // If stop fires first, drop stop_source (injects ManualInterrupt) and
      // still drive idle_future to completion to release the borrow.
      let idle_response: Result<IdleResponse, _> = tokio::select! {
        r = &mut idle_future => r,
        _ = stop_rx.changed() => {
          // Inject interrupt by dropping the stop token, then drain the future.
          drop(stop_source);
          idle_future.await // drives to ManualInterrupt completion
        }
      };
      // stop_source is dropped here if not already dropped above.
      should_stop = *stop_rx.borrow()
        || idle_response.ok() == Some(IdleResponse::ManualInterrupt);
    } // idle_future dropped, borrow on idle_handle released

    let mut resumed = idle_handle
      .done()
      .await
      .map_err(|e| format!("IDLE done: {e}"))?;

    if should_stop {
      return Ok(());
    }

    fetch_unseen(app, &mut resumed, email).await?;
    session = resumed;
  }
}

async fn fetch_unseen(app: &AppHandle, session: &mut ImapSession, email: &str) -> Result<(), String> {
  let uids = session
    .uid_search("UNSEEN")
    .await
    .map_err(|e| format!("UID SEARCH: {e}"))?;

  if uids.is_empty() {
    return Ok(());
  }

  // Limit to the 50 highest (most recent) UIDs to avoid downloading gigabytes on first connect.
  let mut uid_vec: Vec<u32> = uids.into_iter().collect();
  uid_vec.sort_unstable();
  let uids_to_fetch: Vec<u32> = uid_vec.into_iter().rev().take(50).collect();
  let uid_set = uids_to_fetch.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");

  let db_path = app
    .path()
    .app_data_dir()
    .ok()
    .map(|d| d.join("telegramail.db").to_string_lossy().into_owned());

  use futures_util::TryStreamExt as _;

  // Single-pass: fetch full RFC822 and filter by Chat-Version locally.
  // Collect into Vec so the stream borrow on session is released before uid_store.
  let fetched: Vec<(u32, Vec<u8>)> = {
    let mut msgs = session
      .uid_fetch(&uid_set, "RFC822")
      .await
      .map_err(|e| format!("UID FETCH RFC822: {e}"))?;

    let mut acc = Vec::new();
    while let Some(msg) = msgs.try_next().await.map_err(|e| format!("Fetch body: {e}"))? {
      if let (Some(uid), Some(raw)) = (msg.uid, msg.body()) {
        acc.push((uid, raw.to_vec()));
      }
    }
    acc
  }; // stream dropped here, session borrow released

  for (_, raw) in &fetched {
    parse_and_emit(app, raw, email, db_path.as_deref()).await;
  }

  // Mark fetched messages as \Seen so they are not re-delivered on reconnect.
  if !fetched.is_empty() {
    let seen_set = fetched.iter().map(|(uid, _)| uid.to_string()).collect::<Vec<_>>().join(",");
    let _ = session.uid_store(&seen_set, "+FLAGS.SILENT (\\Seen)").await;
  }

  Ok(())
}

/// Unfold RFC 2822 header continuations (CRLF + whitespace → single space).
fn unfold_headers(raw: &str) -> String {
  raw.replace("\r\n ", " ").replace("\r\n\t", " ")
    .replace("\n ", " ").replace("\n\t", " ")
}

async fn parse_and_emit(app: &AppHandle, raw: &[u8], own_email: &str, db_path: Option<&str>) {
  let raw_str = match std::str::from_utf8(raw) {
    Ok(s) => s,
    Err(_) => return,
  };

  // Split headers / body at first blank line (before unfolding, to preserve body).
  let (headers_section, body_raw) = if let Some(pos) = raw_str.find("\r\n\r\n") {
    (&raw_str[..pos], raw_str[pos + 4..].trim())
  } else if let Some(pos) = raw_str.find("\n\n") {
    (&raw_str[..pos], raw_str[pos + 2..].trim())
  } else {
    return;
  };

  // Unfold MIME header continuations before parsing.
  let headers_raw = unfold_headers(headers_section);

  if !headers_raw.contains("Chat-Version:") {
    return;
  }

  let get_header = |name: &str| -> Option<String> {
    let needle = format!("{name}:");
    for line in headers_raw.lines() {
      if line.to_ascii_lowercase().starts_with(&needle.to_ascii_lowercase()) {
        return Some(line[needle.len()..].trim().to_owned());
      }
    }
    None
  };

  // Extract just the email address from "Display Name <email@host>" format.
  let extract_email = |raw_from: &str| -> String {
    if let (Some(start), Some(end)) = (raw_from.find('<'), raw_from.find('>')) {
      raw_from[start + 1..end].trim().to_owned()
    } else {
      raw_from.trim().to_owned()
    }
  };

  let from = match get_header("From") {
    Some(f) => extract_email(&f),
    None => return,
  };
  let autocrypt = get_header("Autocrypt");

  // Auto-import sender's key from Autocrypt header.
  if let (Some(ac_value), Some(path)) = (autocrypt.as_deref(), db_path) {
    if let Ok((addr, armored_pub)) = pgp::parse_autocrypt_header(ac_value) {
      let now = chrono::Utc::now().timestamp();
      let _ = db::upsert_peer_key(path, &addr, &armored_pub, now);
    }
  }

  // Handshake letters are not regular messages — emit email:handshake and stop.
  if let Some(kind) = get_header("X-Telegramail-Handshake") {
    let display_name = get_header("X-Telegramail-Display-Name");
    let _ = app.emit("email:handshake", EmailHandshakeEvent { kind, from, display_name });
    return;
  }

  let message_id = get_header("Message-ID").unwrap_or_default();
  let chat_group_name = get_header("Chat-Group-Name");
  let in_reply_to = get_header("In-Reply-To");
  let telegram_ref = get_header("X-TgAir-Telegram-Ref");

  // Decrypt body if PGP-armored.
  let text = if body_raw.contains("-----BEGIN PGP MESSAGE-----") {
    pgp::decrypt(body_raw, own_email).unwrap_or_else(|e| {
      log::warn!("[imap] PGP decrypt failed: {e}");
      body_raw.to_owned()
    })
  } else {
    body_raw.to_owned()
  };

  let _ = app.emit("email:message", EmailMessageEvent {
    from,
    text,
    telegram_ref,
    message_id,
    chat_group_name,
    in_reply_to,
    autocrypt,
  });
}

fn emit_status(app: &AppHandle, status: EmailStatus, error: Option<String>) {
  let _ = app.emit("email:status", EmailStatusEvent { status, error });
}

fn emit_error(app: &AppHandle, message: String) {
  let _ = app.emit("email:error", EmailErrorEvent { message });
}

// ── Tauri commands ────────────────────────────────────────────────────────────

use super::SmtpResult;

#[tauri::command]
pub async fn imap_start_watch(
  app: AppHandle,
  email: String,
  imap_host: String,
  imap_port: u16,
) -> SmtpResult {
  let password = match super::storage::load_smtp_password(&email) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(format!("Keychain error: {e}")),
  };
  start(app, email, password, imap_host, imap_port);
  SmtpResult::ok()
}

#[tauri::command]
pub async fn imap_stop_watch() -> SmtpResult {
  stop();
  SmtpResult::ok()
}
