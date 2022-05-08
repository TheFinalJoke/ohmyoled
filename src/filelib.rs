use std::fs;
use std::io::Read;
use std::path::Path;

pub fn open_file(file_name: &str) -> std::io::Result<String> {
    let mut f = fs::File::open(file_name)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents)
}
pub fn check_if_exists(path: &str) -> bool {
    Path::new(path).exists()
}
#[allow(dead_code)]
pub fn write_to_file(path: &str, content: &str, overwrite: bool) -> std::io::Result<()> {
    if check_if_exists(path){
        if overwrite {
            fs::remove_file(&path)?;
            fs::File::create(&path).expect("Unable to create file");
            fs::write(path, content).expect("Unable to write to file");
            Ok(())
        } else {
            println!("File exists and do not want to overwite");
            std::process::exit(2)
        }

    } else {
        fs::File::create(&path).expect("Unable to create file");
        fs::write(path, content).expect("Unable to write to file");
        Ok(())
    }
}