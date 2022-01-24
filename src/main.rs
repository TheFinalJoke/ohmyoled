mod filelib;

use clap::Parser;


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    create: bool,
}

fn parse_json(contents: &str) -> json::JsonValue {
    let parsed = json::parse(contents).unwrap();
    parsed
}

fn main() {
    //let contents = filelib::open_file("src/testingjson.txt").unwrap();
    //let parsed = parse_json(&contents);
    //println!("{}", parsed["name"]);

    let args = Args::parse();
}