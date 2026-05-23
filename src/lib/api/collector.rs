//! `Collector` — the trait every API module implements.
//!
//! A collector knows how to talk to one external service (or two, behind a
//! provider enum). Pair it with a `Renderer` whose `Data` type matches
//! `Self::Output`, and the resulting `Module` is automatically driven by the
//! scheduler.

use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait Collector: Send + Sync {
    /// The normalized data this collector produces.
    type Output: Send + 'static;

    /// Stable identifier for logging and config lookup (e.g. `"weather"`).
    fn id(&self) -> &'static str;

    /// How often the collector should be re-polled. The scheduler uses this to
    /// avoid spamming APIs whose data changes slowly.
    fn refresh_interval(&self) -> Duration;

    /// Fetch one fresh value. Errors should be propagated; the scheduler logs
    /// and continues without tearing down the panel.
    async fn poll(&self) -> Result<Self::Output, ApiError>;
}
