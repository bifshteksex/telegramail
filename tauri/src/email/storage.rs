use keyring::Entry;

// All entries share one service name — macOS grants access per (service, app) pair.
const SERVICE: &str = "org.telegramail.Telegramail";
// Legacy service names used before this refactor — checked during migration only.
const LEGACY_SERVICE: &str = "telegram-air";
const LEGACY_PGP_SERVICE: &str = "telegramail-pgp";

fn migrate_entry(new_service: &str, account: &str, legacy_service: &str, legacy_account: &str) -> Option<String> {
  let legacy = Entry::new(legacy_service, legacy_account).ok()?;
  let value = legacy.get_password().ok()?;
  // Write to new location, then delete old entry.
  if let Ok(new_entry) = Entry::new(new_service, account) {
    let _ = new_entry.set_password(&value);
    let _ = legacy.delete_credential();
  }
  Some(value)
}

pub fn save_smtp_password(email: &str, password: &str) -> Result<(), String> {
  Entry::new(SERVICE, &format!("smtp-password:{email}"))
    .map_err(|e| e.to_string())?
    .set_password(password)
    .map_err(|e| e.to_string())
}

pub fn load_smtp_password(email: &str) -> Result<String, String> {
  let account = format!("smtp-password:{email}");
  let entry = Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
  match entry.get_password() {
    Ok(v) => Ok(v),
    Err(keyring::Error::NoEntry) => {
      migrate_entry(SERVICE, &account, LEGACY_SERVICE, &account)
        .ok_or_else(|| "No SMTP password stored".to_owned())
    }
    Err(e) => Err(e.to_string()),
  }
}

pub fn delete_smtp_password(email: &str) -> Result<(), String> {
  Entry::new(SERVICE, &format!("smtp-password:{email}"))
    .map_err(|e| e.to_string())?
    .delete_credential()
    .map_err(|e| e.to_string())
}

pub fn save_smtp_port(email: &str, port: u16) -> Result<(), String> {
  Entry::new(SERVICE, &format!("smtp-port:{email}"))
    .map_err(|e| e.to_string())?
    .set_password(&port.to_string())
    .map_err(|e| e.to_string())
}

pub fn load_smtp_port(email: &str) -> Result<u16, String> {
  let account = format!("smtp-port:{email}");
  let raw = match Entry::new(SERVICE, &account).map_err(|e| e.to_string())?.get_password() {
    Ok(v) => v,
    Err(keyring::Error::NoEntry) => {
      migrate_entry(SERVICE, &account, LEGACY_SERVICE, &account)
        .ok_or_else(|| "No SMTP port stored".to_owned())?
    }
    Err(e) => return Err(e.to_string()),
  };
  raw.parse::<u16>().map_err(|e| e.to_string())
}

pub fn save_pgp_secret(email: &str, armored: &str) -> Result<(), String> {
  Entry::new(SERVICE, &format!("pgp-secret:{email}"))
    .map_err(|e| e.to_string())?
    .set_password(armored)
    .map_err(|e| e.to_string())
}

pub fn load_pgp_secret(email: &str) -> Result<String, String> {
  let account = format!("pgp-secret:{email}");
  let entry = Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
  match entry.get_password() {
    Ok(v) => Ok(v),
    Err(keyring::Error::NoEntry) => {
      migrate_entry(SERVICE, &account, LEGACY_PGP_SERVICE, &account)
        .ok_or_else(|| "No PGP secret key stored".to_owned())
    }
    Err(e) => Err(e.to_string()),
  }
}

pub fn delete_pgp_secret(email: &str) -> Result<(), String> {
  Entry::new(SERVICE, &format!("pgp-secret:{email}"))
    .map_err(|e| e.to_string())?
    .delete_credential()
    .map_err(|e| e.to_string())
}
