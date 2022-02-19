use json;

pub struct TimeOptions {
    pub run: bool,
    pub color: (i32, i32, i32),
    pub time_format: Option<String>,
    pub timezone: Option<String>,
}
impl TimeOptions {
    pub fn convert(&self) -> json::JsonValue {
        json::object!{
            "run": self.run,
            "color": format!("({}, {}, {})", self.color.0, self.color.1, self.color.2),
            "time_format": match &self.time_format {
                Some(format) => format,
                None => "null"
            },
            "timezone": match &self.timezone {
                Some(timezone) => timezone,
                None => "null"
            }
        }
    }
}
pub fn configure() -> TimeOptions {
    println!("Time Configuration");
    println!("No Configuration for Time");
    let options = TimeOptions {
        run: true,
        color: (255,255,255),
        time_format: None, // Not Implemented yet
        timezone: None,    // Not Implemented yet
    };
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_configure() {
        let tested_configure = configure();
        assert_eq!(tested_configure.run, true);
        assert_eq!(tested_configure.color, (255,255,255));
        assert_eq!(tested_configure.time_format, None);
        assert_eq!(tested_configure.timezone, None);
    }
    #[test]
    fn test_convert_time_to_json() {
        let tested_options = TimeOptions{
            run: true,
            color: (255,255,244),
            time_format: None,
            timezone: None
        }.convert();
       assert_eq!(tested_options["run"], true);
       assert_eq!(tested_options["time_format"], "null");
    }
}