use serde::Serialize;

/// Emitted on `email:status` — IMAP connection lifecycle.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailStatusEvent {
  pub status: EmailStatus,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EmailStatus {
  Connected,
  Reconnecting,
  Disconnected,
}

/// Emitted on `email:message` — a new chat message arrived via email.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailMessageEvent {
  /// Sender email address.
  pub from: String,
  /// Plain-text body (already decrypted if it was encrypted).
  pub text: String,
  /// Value of X-TgAir-Telegram-Ref if present (`chatId:localRef`).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub telegram_ref: Option<String>,
  /// Value of Message-ID header.
  pub message_id: String,
  /// Value of Chat-Group-Name header.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub chat_group_name: Option<String>,
  /// Value of In-Reply-To header.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub in_reply_to: Option<String>,
  /// Autocrypt header value, if present (used for peer keyring update).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub autocrypt: Option<String>,
}

/// Emitted on `email:error` — a recoverable operational error.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailErrorEvent {
  pub message: String,
}

/// Emitted on `email:handshake` — a contact request or acceptance arrived.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailHandshakeEvent {
  /// "request" or "accept"
  pub kind: String,
  /// Sender email address.
  pub from: String,
  /// Display name from X-Telegramail-Display-Name header.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub display_name: Option<String>,
}
