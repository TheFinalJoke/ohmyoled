//! Geolocation via the [`ipinfo.io`](https://ipinfo.io) HTTP API.
//!
//! Previously used the `ipinfo` crate, but it depends on `reqwest 0.13`
//! which transitively pins `rustls 0.23` with the `aws-lc-rs` provider —
//! that pulls `aws-lc-sys`, a C library that needs cmake/clang to
//! cross-compile and which broke the aarch64/armv7 release build. We
//! only consume one endpoint, so a direct call via our shared
//! `get_json` helper is both smaller and TLS-stack-agnostic.
//!
//! Endpoint: `https://ipinfo.io/json[?token=...]` — returns the calling
//! host's geolocation. With no token the request hits the rate-limited
//! anonymous endpoint; on paid plans pass the token.

use crate::api::error::ApiError;
use crate::api::http::get_json;
use serde::Deserialize;
use tokio::sync::OnceCell;

/// What we extract from an ipinfo lookup.
#[derive(Debug, Clone)]
pub struct GeoLocation {
    pub lat: f64,
    pub lon: f64,
    pub city: String,
}

/// Look up the *current host's* IP location. Cached for the lifetime of
/// the process — location doesn't change while the panel runs.
///
/// `token` is required by the upstream API on most plans; pass an empty
/// `""` to attempt the free anonymous endpoint (rate-limited).
pub async fn lookup_ipinfo(token: &str) -> Result<GeoLocation, ApiError> {
    static CACHE: OnceCell<GeoLocation> = OnceCell::const_new();

    CACHE
        .get_or_try_init(|| async {
            let url = if token.is_empty() {
                "https://ipinfo.io/json".to_string()
            } else {
                format!("https://ipinfo.io/json?token={token}")
            };
            let raw: IpInfoRaw = get_json(&url, &[]).await.map_err(|e| ApiError::Provider {
                provider: "ipinfo",
                msg: e.to_string(),
            })?;

            let (lat, lon) = parse_loc(&raw.loc).ok_or(ApiError::Provider {
                provider: "ipinfo",
                msg: format!("unparseable loc: {:?}", raw.loc),
            })?;

            Ok(GeoLocation {
                lat,
                lon,
                city: raw.city,
            })
        })
        .await
        .cloned()
}

/// Parse an ipinfo `loc` field — `"lat,lon"`.
fn parse_loc(s: &str) -> Option<(f64, f64)> {
    let (lat, lon) = s.split_once(',')?;
    Some((lat.trim().parse().ok()?, lon.trim().parse().ok()?))
}

// The full ipinfo response carries ip, org, postal, timezone, etc. —
// permissive struct ignores fields we don't consume.
#[derive(Debug, Deserialize)]
struct IpInfoRaw {
    #[serde(default)]
    loc: String,
    #[serde(default)]
    city: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_loc_happy_path() {
        assert_eq!(parse_loc("37.7749,-122.4194"), Some((37.7749, -122.4194)));
    }

    #[test]
    fn parse_loc_with_spaces() {
        assert_eq!(parse_loc("37.7749, -122.4194"), Some((37.7749, -122.4194)));
    }

    #[test]
    fn parse_loc_rejects_garbage() {
        assert_eq!(parse_loc("not a location"), None);
        assert_eq!(parse_loc(""), None);
    }
}
