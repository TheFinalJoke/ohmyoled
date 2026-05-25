use crate::createjson::ui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuroraOptions {
    pub run: bool,
    /// Kp value at which the "AURORA LIKELY" banner appears. 5 matches
    /// NOAA's G1 minor-storm threshold and the registry default.
    pub alert_threshold: u8,
}

impl Default for AuroraOptions {
    fn default() -> Self {
        Self {
            run: true,
            alert_threshold: 5,
        }
    }
}

pub fn configure() -> Result<AuroraOptions, String> {
    ui::section("Aurora (Kp index)");
    ui::hint("NOAA planetary K-index, 0-9. The alert banner trips at or above your threshold.");

    let alert_threshold = loop {
        let raw = ui::read_line_default("Alert threshold (Kp 0-9)", "5");
        match raw.trim().parse::<u8>() {
            Ok(v) if v <= 9 => break v,
            _ => ui::warn("Expected an integer in 0..=9"),
        }
    };

    ui::success(&format!("Aurora — alert at Kp ≥ {alert_threshold}"));
    Ok(AuroraOptions {
        run: true,
        alert_threshold,
    })
}

pub fn summary_line(opts: &AuroraOptions) -> String {
    format!("aurora (alert ≥ Kp {})", opts.alert_threshold)
}
