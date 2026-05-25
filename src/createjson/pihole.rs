use crate::createjson::ui;
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PiholeOptions {
    pub run: bool,
    pub base_url: String,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub token: Option<String>,
}

impl Default for PiholeOptions {
    fn default() -> Self {
        Self {
            run: true,
            base_url: "http://pi.hole".to_owned(),
            token: None,
        }
    }
}

pub fn configure() -> Result<PiholeOptions, String> {
    ui::section("Pi-hole");
    ui::hint("DNS-block summary tile. Token only needed if your admin UI requires auth.");

    let base_url = ui::read_line_default("Base URL (e.g. http://pi.hole)", "http://pi.hole");
    let token_raw = ui::read_line_default(
        "Admin API token (blank = unauth — many home installs allow this)",
        "",
    );
    let token = if token_raw.trim().is_empty() {
        None
    } else {
        Some(token_raw.trim().to_string())
    };

    ui::success(&format!("Pi-hole — {}", base_url));
    Ok(PiholeOptions {
        run: true,
        base_url: base_url.trim().to_string(),
        token,
    })
}

pub fn summary_line(opts: &PiholeOptions) -> String {
    let auth = if opts.token.is_some() { "auth" } else { "unauth" };
    format!("pihole ({}, {auth})", opts.base_url)
}
