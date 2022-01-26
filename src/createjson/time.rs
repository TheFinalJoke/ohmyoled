#[derive(Debug)]
struct TimeOptions {
    run: bool,
    time_format: Option<String>,
    timezone: Option<String>
}

pub fn configure(json: &mut json::JsonValue) {
    println!("Time Configuration");
    println!("No Configuration for Time");
    let options = TimeOptions {
        run: true,
        time_format: None, // Not Implemented yet
        timezone: None
    };
    json.insert("Time", json::object!{
            "run": options.run,
        }
    ).unwrap(); 
}