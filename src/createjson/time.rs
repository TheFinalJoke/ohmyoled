use crate::createjson::ui;
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeOptions {
    pub run: bool,
    pub color: (i32, i32, i32),
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub time_format: Option<String>,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub timezone: Option<String>,
}

impl Default for TimeOptions {
    fn default() -> Self {
        Self {
            run: true,
            color: (255, 255, 255),
            time_format: None,
            timezone: None,
        }
    }
}

fn parse_color(s: &str) -> Option<(i32, i32, i32)> {
    let parts: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == ',' || c == '/' || c == ';')
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].parse::<i32>().ok()?;
    let g = parts[1].parse::<i32>().ok()?;
    let b = parts[2].parse::<i32>().ok()?;
    if [r, g, b].iter().any(|c| !(0..=255).contains(c)) {
        return None;
    }
    Some((r, g, b))
}

pub fn configure() -> TimeOptions {
    ui::section("Time");
    ui::hint("System clock with optional timezone override.");

    if ui::read_yes_no("Use defaults (white, 12h, system timezone)?", true) {
        ui::success("Time — defaults");
        return TimeOptions::default();
    }

    let color = loop {
        let raw = ui::read_line_default("Display color (R G B, 0–255)", "255 255 255");
        match parse_color(&raw) {
            Some(c) => break c,
            None => ui::warn("Expected three integers in 0–255, e.g. '255 255 255'"),
        }
    };

    let fmt_slug = ui::choose(
        "Time format",
        &[
            ("system", "Use the system locale"),
            ("12h", "12-hour with AM/PM"),
            ("24h", "24-hour"),
        ],
        "system",
    );
    let time_format = if fmt_slug == "system" { None } else { Some(fmt_slug) };

    let tz_raw = ui::read_line_default(
        "Timezone (IANA name, blank = system)",
        "",
    );
    let timezone = if tz_raw.trim().is_empty() { None } else { Some(tz_raw) };

    ui::success("Time configured");
    TimeOptions {
        run: true,
        color,
        time_format,
        timezone,
    }
}

pub fn summary_line(opts: &TimeOptions) -> String {
    let fmt = opts.time_format.as_deref().unwrap_or("system");
    let tz = opts.timezone.as_deref().unwrap_or("system");
    format!("time ({fmt}, tz={tz})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy() {
        let t = TimeOptions::default();
        assert!(t.run);
        assert_eq!(t.color, (255, 255, 255));
        assert!(t.time_format.is_none());
        assert!(t.timezone.is_none());
    }

    #[test]
    fn null_string_is_none() {
        let json = r#"{"run":true,"color":[255,255,255],"time_format":"null","timezone":"null"}"#;
        let parsed: TimeOptions = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.time_format, None);
        assert_eq!(parsed.timezone, None);
    }

    #[test]
    fn real_string_kept() {
        let json = r#"{"run":true,"color":[255,255,255],"time_format":"24h","timezone":"UTC"}"#;
        let parsed: TimeOptions = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.time_format.as_deref(), Some("24h"));
        assert_eq!(parsed.timezone.as_deref(), Some("UTC"));
    }

    #[test]
    fn parse_color_accepts_various_separators() {
        assert_eq!(parse_color("255 0 128"), Some((255, 0, 128)));
        assert_eq!(parse_color("10,20,30"), Some((10, 20, 30)));
        assert_eq!(parse_color("10/20/30"), Some((10, 20, 30)));
    }

    #[test]
    fn parse_color_rejects_oob() {
        assert!(parse_color("256 0 0").is_none());
        assert!(parse_color("-1 0 0").is_none());
        assert!(parse_color("a b c").is_none());
        assert!(parse_color("1 2").is_none());
    }
}
