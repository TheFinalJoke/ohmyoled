use std::path::Path;

pub fn check_if_exists(path: &str) -> bool {
    Path::new(path).exists()
}
