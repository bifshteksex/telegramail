use rusqlite::{Connection, params};

/// Open (and migrate) the SQLite database at the given path.
fn open(path: &str) -> Result<Connection, String> {
  let conn = Connection::open(path).map_err(|e| format!("DB open: {e}"))?;
  conn
    .execute_batch(
      "CREATE TABLE IF NOT EXISTS peer_keys (
         email      TEXT PRIMARY KEY,
         public_key TEXT NOT NULL,
         updated_at INTEGER NOT NULL
       );",
    )
    .map_err(|e| format!("DB migrate: {e}"))?;
  Ok(conn)
}

/// Upsert a peer's public key. `updated_at` is a Unix timestamp (seconds).
pub fn upsert_peer_key(db_path: &str, email: &str, public_key: &str, updated_at: i64) -> Result<(), String> {
  let conn = open(db_path)?;
  conn
    .execute(
      "INSERT INTO peer_keys (email, public_key, updated_at)
       VALUES (?1, ?2, ?3)
       ON CONFLICT(email) DO UPDATE SET
         public_key = CASE WHEN excluded.updated_at > peer_keys.updated_at
                          THEN excluded.public_key ELSE peer_keys.public_key END,
         updated_at = MAX(excluded.updated_at, peer_keys.updated_at)",
      params![email, public_key, updated_at],
    )
    .map_err(|e| format!("DB upsert peer key: {e}"))?;
  Ok(())
}

/// Load a peer's public key. Returns None if not found.
pub fn load_peer_key(db_path: &str, email: &str) -> Result<Option<String>, String> {
  let conn = open(db_path)?;
  let mut stmt = conn
    .prepare("SELECT public_key FROM peer_keys WHERE email = ?1")
    .map_err(|e| format!("DB prepare: {e}"))?;
  let mut rows = stmt
    .query(params![email])
    .map_err(|e| format!("DB query: {e}"))?;
  if let Some(row) = rows.next().map_err(|e| format!("DB row: {e}"))? {
    Ok(Some(row.get(0).map_err(|e| format!("DB get: {e}"))?))
  } else {
    Ok(None)
  }
}

/// Delete a peer's key (e.g. on key revocation or manual reset).
pub fn delete_peer_key(db_path: &str, email: &str) -> Result<(), String> {
  let conn = open(db_path)?;
  conn
    .execute("DELETE FROM peer_keys WHERE email = ?1", params![email])
    .map_err(|e| format!("DB delete: {e}"))?;
  Ok(())
}
