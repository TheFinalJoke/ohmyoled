//! Normalized aurora forecast — what the SWPC collector hands to the
//! renderer.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AuroraReading {
    /// Rounded planetary Kp index (0..=9). Drives the headline digit.
    pub kp: u8,
    /// Raw float value from SWPC (e.g. `4.67`) — kept around for callers
    /// that want to render a precise reading or split bar segments.
    pub kp_index: f32,
    /// Quadrant-form Kp from SWPC: `"4-"`, `"3+"`, `"0Z"`, etc. `Z` means
    /// "zero" (the absolute floor). Useful for a precise badge.
    pub kp_text: String,
    /// True when `kp >= alert_threshold` — i.e. the configured threshold
    /// for "aurora is likely visible".
    pub alert: bool,
    /// Observation time from SWPC (UTC).
    pub sampled_at: DateTime<Utc>,
}
