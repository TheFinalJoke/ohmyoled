//! Process-wide shared `reqwest::Client` + a typed `get_json` helper.

use crate::api::error::ApiError;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use std::time::Duration;

static CLIENT: OnceLock<Client> = OnceLock::new();

/// Get the shared HTTP client, building it on first use.
///
/// Configured for the small JSON payloads ohmyoled fetches: gzip enabled,
/// 15-second timeout, a real user-agent so APIs don't 403 us.
pub fn shared_client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent(concat!("ohmyoled/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(15))
            .build()
            .expect("reqwest::Client build failed — invalid TLS or system config")
    })
}

/// GET `url` with optional headers, decode the JSON body as `T`.
pub async fn get_json<T: DeserializeOwned>(
    url: &str,
    headers: &[(&str, &str)],
) -> Result<T, ApiError> {
    let mut req = shared_client().get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await?.error_for_status()?;
    Ok(resp.json::<T>().await?)
}
