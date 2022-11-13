pub mod api;
pub mod teams;
pub mod debug;

pub fn get_input() -> Option<String> {
    let mut buffer = String::new();
    while std::io::stdin().read_line(&mut buffer).is_err() {
        println!("Please enter your data again");
    }
    let input = buffer.trim().to_owned();
    if &input == "" {
        None
    } else {
        Some(input)
    }
}
//pub mod types {
//    tonic::include_proto!("build");
//}