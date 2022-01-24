use std::fs::File;
use std::io::Read;

pub fn open_file(file_name: &str) -> std::io::Result<String> {
    let mut f = File::open(file_name)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents)
}