pub struct TimeOptions {
    run: bool,
    time_format: Option<String>,
    timezone: Option<String>
}
pub fn configure() -> TimeOptions{
    println!("Time Configuration");
    println!("No Configuration for Time");
    let options = TimeOptions {
        run: true,
        time_format: None, // Not Implemented yet
        timezone: None // Not Implemented yet 
    };
    options
}

