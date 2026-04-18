use lettre::message::{Mailbox, SinglePart, header};
use lettre::Message;

use super::pgp;

/// Parameters for building an outgoing chat email.
pub struct ChatMailParams<'a> {
  pub from_email: &'a str,
  pub to_email: &'a str,
  pub chat_name: &'a str,
  pub text: &'a str,
  /// Telegram chat id — used in Message-ID and X-TgAir-Telegram-Ref.
  pub chat_id: &'a str,
  /// Client-side local reference (timestamp or local message id).
  pub local_ref: &'a str,
  /// If Some, this email is a reply to the given email Message-ID.
  pub in_reply_to: Option<&'a str>,
  /// Armored public key of the sender — added as Autocrypt header.
  pub own_public_key: Option<&'a str>,
  /// Armored public key of the recipient — if present, body is encrypted.
  pub recipient_public_key: Option<&'a str>,
}

/// Builds a Delta Chat-compatible outgoing message.
/// Headers:
///   Chat-Version: 1.0
///   Message-ID: <tgair-{chatId}-{localRef}@telegramail>
///   X-TgAir-Telegram-Ref: {chatId}:{localRef}
///   Chat-Group-Name: {chatName}
///   Autocrypt: addr={from}; keydata=... (if own key provided)
///   In-Reply-To / References (if replying)
/// Body: PGP-encrypted if recipient key present, else plain text.
pub fn build_chat_message(p: &ChatMailParams<'_>) -> Result<Message, String> {
  let from: Mailbox = p.from_email.parse().map_err(|e| format!("Invalid from address: {e}"))?;
  let to: Mailbox = p.to_email.parse().map_err(|e| format!("Invalid to address: {e}"))?;

  let message_id = format!("<tgair-{}-{}@telegramail>", p.chat_id, p.local_ref);
  let tg_ref = format!("{}:{}", p.chat_id, p.local_ref);

  // Subject is intentionally opaque (Delta Chat convention).
  let subject = "Chat Message";

  let mut builder = Message::builder()
    .from(from)
    .to(to)
    .subject(subject)
    .raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("Message-ID"),
      message_id,
    ))
    .raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("Chat-Version"),
      "1.0".to_owned(),
    ))
    .raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("X-TgAir-Telegram-Ref"),
      tg_ref,
    ))
    .raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("Chat-Group-Name"),
      p.chat_name.to_owned(),
    ));

  if let Some(pub_key) = p.own_public_key {
    let autocrypt = pgp::build_autocrypt_header(p.from_email, pub_key)?;
    builder = builder.raw_header(header::HeaderValue::new(
      header::HeaderName::new_from_ascii_str("Autocrypt"),
      autocrypt,
    ));
  }

  if let Some(reply_to) = p.in_reply_to {
    builder = builder
      .raw_header(header::HeaderValue::new(
        header::HeaderName::new_from_ascii_str("In-Reply-To"),
        reply_to.to_owned(),
      ))
      .raw_header(header::HeaderValue::new(
        header::HeaderName::new_from_ascii_str("References"),
        reply_to.to_owned(),
      ));
  }

  let body = match p.recipient_public_key {
    Some(rec_key) => pgp::encrypt(p.text, rec_key)?,
    None => p.text.to_owned(),
  };

  builder
    .singlepart(SinglePart::plain(body))
    .map_err(|e| format!("Failed to build message: {e}"))
}
