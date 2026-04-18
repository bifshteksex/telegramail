use tauri::{AppHandle, Manager};

use super::{db, pgp, storage, SmtpResult};

fn db_path(app: &AppHandle) -> Result<String, String> {
  let dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("app_data_dir: {e}"))?;
  std::fs::create_dir_all(&dir).map_err(|e| format!("create_dir_all: {e}"))?;
  Ok(dir.join("telegramail.db").to_string_lossy().into_owned())
}

/// Returns the armored public key for the given email, or empty string if none.
#[tauri::command]
pub async fn smtp_get_public_key(app: AppHandle, email: String) -> SmtpResult {
  let path = match db_path(&app) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(e),
  };
  match db::load_peer_key(&path, &format!("self:{email}")) {
    Ok(Some(key)) => SmtpResult { ok: true, error: None, data: Some(key) },
    Ok(None) => SmtpResult { ok: true, error: None, data: None },
    Err(e) => SmtpResult::err(e),
  }
}

#[tauri::command]
pub async fn smtp_save_credentials(app: AppHandle, email: String, password: String) -> SmtpResult {
  // Save SMTP password to keychain.
  if let Err(e) = storage::save_smtp_password(&email, &password) {
    return SmtpResult::err(e);
  }

  // Generate PGP keypair if one doesn't exist yet.
  let path = match db_path(&app) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(e),
  };

  let self_key = format!("self:{email}");
  let has_key = db::load_peer_key(&path, &self_key)
    .unwrap_or(None)
    .is_some();

  if !has_key {
    let (secret_armored, public_armored) = match pgp::generate_keypair(&email) {
      Ok(pair) => pair,
      Err(e) => return SmtpResult::err(format!("PGP keygen: {e}")),
    };
    if let Err(e) = pgp::save_secret_key(&email, &secret_armored) {
      return SmtpResult::err(format!("Save PGP secret key: {e}"));
    }
    let now = chrono::Utc::now().timestamp();
    if let Err(e) = db::upsert_peer_key(&path, &self_key, &public_armored, now) {
      return SmtpResult::err(format!("Save PGP public key: {e}"));
    }
  }

  SmtpResult::ok()
}

#[tauri::command]
pub async fn smtp_load_credentials(email: String) -> SmtpResult {
  match storage::load_smtp_password(&email) {
    Ok(_) => SmtpResult::ok(),
    Err(e) => SmtpResult::err(e),
  }
}

#[tauri::command]
pub async fn smtp_delete_credentials(app: AppHandle, email: String) -> SmtpResult {
  if let Err(e) = storage::delete_smtp_password(&email) {
    return SmtpResult::err(e);
  }
  let _ = pgp::delete_secret_key(&email);
  if let Ok(path) = db_path(&app) {
    let _ = db::delete_peer_key(&path, &format!("self:{email}"));
  }
  SmtpResult::ok()
}

/// Import a peer's public key from an Autocrypt header value.
#[tauri::command]
pub async fn smtp_import_autocrypt(app: AppHandle, autocrypt_value: String) -> SmtpResult {
  let (addr, armored_pub) = match pgp::parse_autocrypt_header(&autocrypt_value) {
    Ok(v) => v,
    Err(e) => return SmtpResult::err(format!("Parse Autocrypt: {e}")),
  };
  let path = match db_path(&app) {
    Ok(p) => p,
    Err(e) => return SmtpResult::err(e),
  };
  let now = chrono::Utc::now().timestamp();
  match db::upsert_peer_key(&path, &addr, &armored_pub, now) {
    Ok(()) => SmtpResult::ok(),
    Err(e) => SmtpResult::err(e),
  }
}
