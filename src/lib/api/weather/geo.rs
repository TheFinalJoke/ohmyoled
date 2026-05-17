//! Geolocation via the `ipinfo` crate.
//!
//! Mirrors Python's `ipinfo.getHandler(...).getDetails()` calls in
//! `openweather/weather.py` and `weathergov/nws.py`. The result is cached for
//! the lifetime of the collector (location doesn't change while the panel runs).

use crate::api::error::ApiError;
use ipinfo::{IpInfo, IpInfoConfig};
use tokio::sync::{Mutex, OnceCell};

/// What we extract from an ipinfo lookup.
#[derive(Debug, Clone)]
pub struct GeoLocation {
    pub lat: f64,
    pub lon: f64,
    pub city: String,
}

/// Look up the *current host's* IP location.
///
/// `token` is required by the upstream API on most plans; pass an empty
/// `""` to attempt the free anonymous endpoint (rate-limited).
pub async fn lookup_ipinfo(token: &str) -> Result<GeoLocation, ApiError> {
    static CLIENT: OnceCell<Mutex<IpInfo>> = OnceCell::const_new();

    let client_mu = CLIENT
        .get_or_try_init(|| async {
            let cfg = IpInfoConfig {
                token: if token.is_empty() { None } else { Some(token.to_string()) },
                ..Default::default()
            };
            IpInfo::new(cfg)
                .map(Mutex::new)
                .map_err(|e| ApiError::Provider {
                    provider: "ipinfo",
                    msg: e.to_string(),
                })
        })
        .await?;

    // `lookup("")` queries the calling host's own IP.
    let mut guard = client_mu.lock().await;
    let details = guard.lookup("").await.map_err(|e| ApiError::Provider {
        provider: "ipinfo",
        msg: e.to_string(),
    })?;
    drop(guard);

    let (lat, lon) = parse_loc(&details.loc).ok_or(ApiError::Provider {
        provider: "ipinfo",
        msg: format!("unparseable loc: {:?}", details.loc),
    })?;

    Ok(GeoLocation {
        lat,
        lon,
        city: details.city,
    })
}

/// Parse an ipinfo `loc` field — `"lat,lon"`.
fn parse_loc(s: &str) -> Option<(f64, f64)> {
    let (lat, lon) = s.split_once(',')?;
    Some((lat.trim().parse().ok()?, lon.trim().parse().ok()?))
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
