mod filelib;
mod createjson;

use clap::{Arg, App};

fn parse_json(contents: &str) -> json::JsonValue {
    let parsed = json::parse(contents).unwrap();
    parsed
}
fn parse_json_file(file: &str) -> json::JsonValue {
    let contents = filelib::open_file(file).unwrap();
    let final_parse = parse_json(&contents);
    final_parse
}
fn main() {
    let app = App::new("ohmyoled").version("2.0.0");
    let args_vec = vec![
        Arg::new("create_json")
        .short('c')
        .long("create_json")
        .help("Creates a json for oled configuration"),
        Arg::new("json_file")
        .short('f')
        .long("json_file")
        .help("Pass a location of json file")
        .takes_value(true),
    ];
    
    let app = app.args(args_vec);
    let matches = app.get_matches();
    if matches.is_present("create_json") {
        createjson::create_json();
        std::process::exit(0);
    } else if matches.is_present("json_file") {
        let main_json = parse_json_file(matches.value_of("json_file").unwrap());
    }
}