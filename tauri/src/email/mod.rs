use serde::{Deserialize, Serialize};

pub mod credentials;
pub mod db;
pub mod events;
pub mod handshake;
pub mod imap;
pub mod mime;
pub mod pgp;
pub mod smtp;
pub mod storage;

#[derive(Debug, Serialize, Deserialize)]
pub struct SmtpResult {
  pub ok: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  /// Optional payload (e.g. armored public key).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<String>,
}

impl SmtpResult {
  pub fn ok() -> Self {
    Self { ok: true, error: None, data: None }
  }

  pub fn err(msg: impl Into<String>) -> Self {
    Self { ok: false, error: Some(msg.into()), data: None }
  }
}
