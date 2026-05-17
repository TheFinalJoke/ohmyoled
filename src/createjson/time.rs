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

pub fn configure() -> TimeOptions {
    println!("Time Configuration");
    println!("No Configuration for Time");
    TimeOptions {
        run: true,
        color: (255, 255, 255),
        time_format: None,
        timezone: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_configure() {
        let tested = configure();
        assert!(tested.run);
        assert_eq!(tested.color, (255, 255, 255));
        assert_eq!(tested.time_format, None);
        assert_eq!(tested.timezone, None);
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
}
