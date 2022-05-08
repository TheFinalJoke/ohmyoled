// This is for traits for the Options
use json::JsonValue;
pub trait ApiOptions {
    fn run(&self) -> bool;
}
// Figure out how to use traits
pub trait IntoJson {
    fn convert(&self) -> JsonValue;
}