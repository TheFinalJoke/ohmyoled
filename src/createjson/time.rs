use json;

pub struct TimeOptions {
    pub run: bool,
    pub time_format: Option<String>,
    pub timezone: Option<String>,
}
impl TimeOptions {
    pub fn convert(&self) -> json::JsonValue {
        json::object!{
            "run": self.run,
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
        assert_eq!(tested_configure.time_format, None);
        assert_eq!(tested_configure.timezone, None);
    }
    #[test]
    fn test_convert_time_to_json() {
        let tested_options = TimeOptions{
            run: true,
            time_format: None,
            timezone: None
        }.convert();
       assert_eq!(tested_options["run"], true);
       assert_eq!(tested_options["time_format"], "null");
    }
}