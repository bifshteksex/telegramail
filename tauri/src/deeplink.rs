use log::info;
use tauri::{Emitter, Manager};
#[cfg(desktop)]
use tauri::UserAttentionType;
use tauri_plugin_deep_link::DeepLinkExt;
use url::Url;

use crate::email::events::EmailHandshakeEvent;

pub struct Deeplink;

impl Deeplink {
  pub fn init() -> Self {
    Self {}
  }

  pub fn setup(&self, app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.clone();

    app.deep_link().on_open_url(move |event| {
      let urls = event.urls();
      info!("Deep link received: {:?}", urls);

      for url in &urls {
        if url.scheme() == "telegramail" {
          handle_telegramail_url(&app_handle, url);
        }
      }

      // Bring the window to the front for any deep-link.
      if let Some(window) = app_handle.get_webview_window("main") {
        // For tg:// links keep original behaviour — forward raw URL strings to frontend.
        let has_non_telegramail = urls.iter().any(|u| u.scheme() != "telegramail");
        if has_non_telegramail {
          let url_strings: Vec<String> = urls.iter().map(|u| u.to_string()).collect();
          if let Err(err) = window.emit("deeplink", &url_strings) {
            info!("Error emitting deeplink event: {:?}", err);
          }
        }

        #[cfg(desktop)]
        let _ = window.request_user_attention(Some(UserAttentionType::Informational));
        #[cfg(desktop)]
        let _ = window.show();
        #[cfg(desktop)]
        let _ = window.unminimize();
        #[cfg(desktop)]
        let _ = window.set_focus();
      }
    });

    Ok(())
  }
}

fn handle_telegramail_url(app: &tauri::AppHandle, url: &Url) {
  let path = url.path().trim_matches('/');

  if path == "handshake" {
    let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let from = match params.get("email") {
      Some(e) => e.to_string(),
      None => {
        info!("[deeplink] telegramail://handshake missing `email` param");
        return;
      }
    };
    let display_name = params.get("name").map(|n| n.to_string());
    let pub_key = params.get("pgp").map(|k| k.to_string());

    // If a public key is present, import it into the peer keyring before emitting.
    if let Some(ref armored_key) = pub_key {
      if let Ok(app_data) = app.path().app_data_dir() {
        let db_path = app_data.join("telegramail.db").to_string_lossy().into_owned();
        let now = chrono::Utc::now().timestamp();
        if let Err(e) = crate::email::db::upsert_peer_key(&db_path, &from, armored_key, now) {
          info!("[deeplink] failed to import pgp key from handshake link: {e}");
        }
      }
    }

    let event = EmailHandshakeEvent {
      kind: "request".to_owned(),
      from,
      display_name,
    };
    if let Err(e) = app.emit("email:handshake", event) {
      info!("[deeplink] failed to emit email:handshake: {e}");
    }
  } else {
    info!("[deeplink] unknown telegramail path: {path}");
  }
}
