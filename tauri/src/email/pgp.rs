use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use pgp::composed::key::{KeyType, SecretKeyParamsBuilder, SubkeyParamsBuilder};
use pgp::composed::message::Message;
use pgp::composed::{Deserializable, SignedSecretKey};
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::ser::Serialize;
use pgp::types::SecretKeyTrait;
use smallvec::smallvec;

const KEYCHAIN_SERVICE: &str = "telegramail-pgp";

// ── Key generation ────────────────────────────────────────────────────────────

/// Generate an Ed25519 signing key + X25519 encryption subkey.
/// Returns (armored_secret_key, armored_public_key).
pub fn generate_keypair(email: &str) -> Result<(String, String), String> {
  let params = SecretKeyParamsBuilder::default()
    .key_type(KeyType::Ed25519)
    .can_sign(true)
    .can_certify(true)
    .can_encrypt(false)
    .primary_user_id(email.to_owned())
    .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256])
    .passphrase(None)
    .subkey(
      SubkeyParamsBuilder::default()
        .key_type(KeyType::X25519)
        .can_encrypt(true)
        .passphrase(None)
        .build()
        .map_err(|e| format!("Subkey build error: {e}"))?,
    )
    .build()
    .map_err(|e| format!("Key params build error: {e}"))?;

  let secret_key = params
    .generate(rand::thread_rng())
    .map_err(|e| format!("Key generation error: {e}"))?;

  let signed_secret_key = secret_key
    .sign(rand::thread_rng(), || String::new())
    .map_err(|e| format!("Key signing error: {e}"))?;

  let public_key = signed_secret_key.public_key();
  let signed_public_key = public_key
    .sign(rand::thread_rng(), &signed_secret_key, || String::new())
    .map_err(|e| format!("Public key signing error: {e}"))?;

  let mut secret_buf = Vec::new();
  signed_secret_key
    .to_armored_writer(&mut secret_buf, Default::default())
    .map_err(|e| format!("Armor secret key error: {e}"))?;

  let mut public_buf = Vec::new();
  signed_public_key
    .to_armored_writer(&mut public_buf, Default::default())
    .map_err(|e| format!("Armor public key error: {e}"))?;

  Ok((
    String::from_utf8(secret_buf).map_err(|e| e.to_string())?,
    String::from_utf8(public_buf).map_err(|e| e.to_string())?,
  ))
}

// ── Keychain storage for own secret key ──────────────────────────────────────

pub fn save_secret_key(email: &str, armored: &str) -> Result<(), String> {
  keyring::Entry::new(KEYCHAIN_SERVICE, &format!("pgp-secret:{email}"))
    .map_err(|e| e.to_string())?
    .set_password(armored)
    .map_err(|e| e.to_string())
}

pub fn load_secret_key(email: &str) -> Result<String, String> {
  keyring::Entry::new(KEYCHAIN_SERVICE, &format!("pgp-secret:{email}"))
    .map_err(|e| e.to_string())?
    .get_password()
    .map_err(|e| e.to_string())
}

pub fn delete_secret_key(email: &str) -> Result<(), String> {
  keyring::Entry::new(KEYCHAIN_SERVICE, &format!("pgp-secret:{email}"))
    .map_err(|e| e.to_string())?
    .delete_credential()
    .map_err(|e| e.to_string())
}

// ── Autocrypt header ──────────────────────────────────────────────────────────

/// Build the value of the `Autocrypt:` header for outgoing mail.
/// Format: `addr=<email>; keydata=<base64 DER public key>`
pub fn build_autocrypt_header(email: &str, armored_public_key: &str) -> Result<String, String> {
  let (signed_pub, _) = pgp::composed::SignedPublicKey::from_string(armored_public_key)
    .map_err(|e| format!("Parse public key: {e}"))?;

  let mut buf = Vec::new();
  signed_pub
    .to_writer(&mut buf)
    .map_err(|e| format!("Serialize public key: {e}"))?;

  let keydata = B64.encode(&buf);
  Ok(format!("addr={email}; keydata={keydata}"))
}

/// Parse an `Autocrypt:` header value.
/// Returns (addr, armored_public_key) or an error.
pub fn parse_autocrypt_header(value: &str) -> Result<(String, String), String> {
  let mut addr = String::new();
  let mut keydata_b64 = String::new();

  for part in value.split(';') {
    let part = part.trim();
    if let Some(v) = part.strip_prefix("addr=") {
      addr = v.trim().to_owned();
    } else if let Some(v) = part.strip_prefix("keydata=") {
      keydata_b64 = v.split_whitespace().collect::<String>();
    }
  }

  if addr.is_empty() || keydata_b64.is_empty() {
    return Err("Missing addr or keydata in Autocrypt header".into());
  }

  let raw = B64.decode(&keydata_b64).map_err(|e| format!("Base64 decode: {e}"))?;

  let signed_pub = pgp::composed::SignedPublicKey::from_bytes(Cursor::new(&raw))
    .map_err(|e| format!("Parse keydata: {e}"))?;

  let mut armor_buf = Vec::new();
  signed_pub
    .to_armored_writer(&mut armor_buf, Default::default())
    .map_err(|e| format!("Re-armor: {e}"))?;

  let armored = String::from_utf8(armor_buf).map_err(|e| e.to_string())?;
  Ok((addr, armored))
}

// ── Encryption / Decryption ───────────────────────────────────────────────────

/// Encrypt `plaintext` to `recipient_armored_pubkey`.
/// Returns PGP/ASCII-armored ciphertext.
pub fn encrypt(plaintext: &str, recipient_armored_pubkey: &str) -> Result<String, String> {
  let (signed_pub, _) = pgp::composed::SignedPublicKey::from_string(recipient_armored_pubkey)
    .map_err(|e| format!("Parse recipient key: {e}"))?;

  let lit = Message::new_literal("msg.txt", plaintext);

  let mut rng = rand::thread_rng();
  let encrypted = lit
    .encrypt_to_keys_seipdv1(&mut rng, SymmetricKeyAlgorithm::AES256, &[&signed_pub])
    .map_err(|e| format!("Encrypt error: {e}"))?;

  encrypted
    .to_armored_string(Default::default())
    .map_err(|e| format!("Armor error: {e}"))
}

/// Decrypt an armored PGP message using the secret key from keychain.
pub fn decrypt(ciphertext: &str, email: &str) -> Result<String, String> {
  let armored_secret = load_secret_key(email)?;

  let (signed_secret, _) = SignedSecretKey::from_string(&armored_secret)
    .map_err(|e| format!("Parse secret key: {e}"))?;

  let (message, _) = Message::from_string(ciphertext)
    .map_err(|e| format!("Parse ciphertext: {e}"))?;

  let (decrypted, _) = message
    .decrypt(|| String::new(), &[&signed_secret])
    .map_err(|e| format!("Decrypt error: {e}"))?;

  let content = decrypted
    .get_content()
    .map_err(|e| format!("Get content: {e}"))?
    .ok_or_else(|| "Empty decrypted message".to_owned())?;

  String::from_utf8(content).map_err(|e| e.to_string())
}
