use keyring::Entry;

const SERVICE: &str = "telegram-air";
const SMTP_PASSWORD_ACCOUNT: &str = "smtp-password";
const SMTP_PORT_ACCOUNT: &str = "smtp-port";
pub fn save_smtp_password(email: &str, password: &str) -> Result<(), String> {
  let account = format!("{}:{}", SMTP_PASSWORD_ACCOUNT, email);
  let entry = Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
  entry.set_password(password).map_err(|e| e.to_string())
}

pub fn load_smtp_password(email: &str) -> Result<String, String> {
  let account = format!("{}:{}", SMTP_PASSWORD_ACCOUNT, email);
  let entry = Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
  entry.get_password().map_err(|e| e.to_string())
}

pub fn delete_smtp_password(email: &str) -> Result<(), String> {
  let account = format!("{}:{}", SMTP_PASSWORD_ACCOUNT, email);
  let entry = Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
  entry.delete_credential().map_err(|e| e.to_string())
}

pub fn save_smtp_port(email: &str, port: u16) -> Result<(), String> {
  let account = format!("{}:{}", SMTP_PORT_ACCOUNT, email);
  let entry = Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
  entry.set_password(&port.to_string()).map_err(|e| e.to_string())
}

pub fn load_smtp_port(email: &str) -> Result<u16, String> {
  let account = format!("{}:{}", SMTP_PORT_ACCOUNT, email);
  let entry = Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
  let raw = entry.get_password().map_err(|e| e.to_string())?;
  raw.parse::<u16>().map_err(|e| e.to_string())
}

