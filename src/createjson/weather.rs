enum WeatherFormat {
    IMPERIAL,
    METRIC
}
struct WeatherOptions {
    run: bool,
    api: String,
    current_location: bool,
    city: Option<String>,
    weather_format: Option<WeatherFormat>
}
