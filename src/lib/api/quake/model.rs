//! Normalized earthquake data — what the collector hands to the renderer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Which USGS feed window this collector subscribes to.
///
/// USGS publishes parallel `summary` feeds covering the past day, week, and
/// month at different magnitude thresholds. We only expose the day-window
/// feeds here — the others are too noisy or too quiet for a single-tile
/// rotation display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QuakeFeed {
    /// "Significant" events in the last 24h. Quietest default — fits the
    /// "alert when something interesting happens" use case.
    #[default]
    SignificantDay,
    /// M ≥ 4.5 in the last 24h.
    M45Day,
    /// M ≥ 2.5 in the last 24h.
    M25Day,
    /// Everything USGS recorded in the last 24h.
    AllDay,
}

impl QuakeFeed {
    pub fn slug(self) -> &'static str {
        match self {
            Self::SignificantDay => "significant_day",
            Self::M45Day => "4.5_day",
            Self::M25Day => "2.5_day",
            Self::AllDay => "all_day",
        }
    }
}

#[derive(Debug, Clone)]
pub enum QuakeStatus {
    /// No events in the selected feed window.
    Quiet,
    /// Top-magnitude event from the feed.
    Event(QuakeEvent),
}

#[derive(Debug, Clone)]
pub struct QuakeEvent {
    /// Numeric magnitude — kept around purely for color-band selection in
    /// the renderer. The textual `"M 6.2"` form is already inside `title`.
    pub magnitude: f32,
    /// USGS-curated title, e.g. `"M 6.2 - OFF EAST COAST OF HONSHU, JAPAN"`.
    /// Prefer this over assembling `M {mag} - {place}` ourselves; USGS
    /// occasionally tweaks formatting per event.
    pub title: String,
    /// Origin time of the event.
    pub origin: DateTime<Utc>,
    /// Depth below the surface, km.
    pub depth_km: f32,
    /// Number of "Did You Feel It?" community reports submitted for this
    /// event. `None` when USGS has no reports yet (common for offshore /
    /// remote / small events).
    pub felt: Option<u32>,
}

impl QuakeEvent {
    pub fn age_minutes(&self, now: DateTime<Utc>) -> u32 {
        let secs = (now - self.origin).num_seconds().max(0);
        (secs / 60) as u32
    }
}
