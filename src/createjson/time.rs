use json;
use pyo3::Python;
use pyo3::types::{PyDict, IntoPyDict};

#[derive(Debug)]
pub struct TimeOptions {
    pub run: bool,
    pub color: (i32, i32, i32),
    pub time_format: Option<String>,
    pub timezone: Option<String>,
}
impl IntoPyDict for TimeOptions {
    fn into_py_dict(self, py: Python) -> &PyDict {
        let result = PyDict::new(py);
        result.set_item("run", self.run).unwrap();
        result.set_item("color", self.color).unwrap();
        result.set_item("time_format", self.unwrap_time_format()).unwrap();
        result.set_item("timezone", self.unwrap_timezone()).unwrap();
        result.into()
    }
}
impl TimeOptions {
    pub fn unwrap_timezone(&self) -> String {
        match &self.timezone  {
            Some(timezone) => timezone.to_string(),
            None => "".to_string()
        }
    }
    pub fn unwrap_time_format(&self) -> String {
        match &self.time_format {
            Some(format) => format.to_string(),
            None => "".to_string()
        }
    }
    pub fn convert(&self) -> json::JsonValue {
        json::object!{
            "run": self.run,
            "color": json::array!(self.color.0, self.color.1, self.color.2),
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
    pub fn from_json(time_json: &json::JsonValue) -> Self {
        Self {
            run: time_json["run"].as_bool().unwrap(),
            color: (time_json["color"][0].as_i32().unwrap(), time_json["color"][1].as_i32().unwrap(), time_json["color"][2].as_i32().unwrap()),
            time_format: {
                if time_json["time_format"].as_str().unwrap() == "null" {
                    None
                } else {
                    Some(time_json["time_format"].as_str().unwrap().to_string())
                }
            },
            timezone: { 
                if time_json["timezone"].as_str().unwrap() == "null" {
                    None
                } else {
                    Some(time_json["timezone"].as_str().unwrap().to_string())
                }
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