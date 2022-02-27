mod filelib;
mod createjson;
extern crate log;
use env_logger::{Env};
use clap::{Arg, App};
use json;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::collections::HashMap;

struct ModuleValue {
    api: Py<PyAny>
}
fn parse_json(contents: &str) -> json::JsonValue {
    let parsed = json::parse(contents).unwrap();
    parsed
}
fn parse_json_file(file: &str) -> json::JsonValue {
    let contents = filelib::open_file(file).unwrap();
    let final_parse = parse_json(&contents);
    final_parse
}
fn get_modules(json_config: &json::JsonValue, module_map: &mut HashMap<String, ModuleValue>) {
    /*
    if json_config.has_key("time") {
        module_map.insert("time", ModuleValue {
            api: {
                Python::with_gil(|py| {
                    let ohmyled_lib = PyModule::import(py, "ohmyoled.lib").unwrap();
                    let API = ohmyled_lib.getattr("API").unwrap();
                    dbg!(API.getattr("APISPORTS");
                })
            }
        })

    }
    */
    println!("stuff");
}
fn init_logger() {
    let env = Env::default()
        .filter_or("RUST_LOG", "error")
        .write_style("always");
    env_logger::init_from_env(env);
}
/*
fn to_hashmap(conf: &json::JsonValue) {
    println!("{}", conf.entries());
}
fn run_python(conf: &json::JsonValue) -> PyResult<()> {
    // https://pyo3.rs/v0.15.1/ecosystem/async-await.html
    /*
    let fut = Python::with_gil(|py| {
        let asyncio = py.import("asyncio")?;
        // convert asyncio.sleep into a Rust Future
        pyo3_asyncio::tokio::into_future(asyncio.call_method1("sleep", (1.into_py(py),))?)
    })?;
    fut.await?;
    Ok(())
    */
    to_hashmap(conf); 
    Python::with_gil(|py| {
        let wapi_import = py.import("ohmyoled.main").unwrap();
        dbg!(wapi_import);
        let weather_api_class = wapi_import.getattr("Main").unwrap();
        dbg!(weather_api_class);
        //let config = PyTuple::new(py, [conf.dump()]);       
        //let weather_api_object = weather_api_class.call1(config).unwrap();
        //dbg!(weather_api_object);
        //dbg!(weather_api_object.call_method0("parse_config_file").unwrap());
    });
    Ok(())
}
*/

fn main() {
    init_logger();
    let mut configuration = json::JsonValue::Null;
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
    if matches.is_present("create_json") { // make an array of options
        let default_json_path = "/etc/ohmyoled/ohmyoled.json";
        if filelib::check_if_exists(&default_json_path){
            println!("Would you like to overwrite ({})? (y/n)", &default_json_path);
            match oledlib::get_input().unwrap().to_lowercase().as_str() {
                "y" => {
                    let main_json = createjson::create_json();
                    std::fs::remove_file(&default_json_path).expect("Can not Remove file");
                    let mut file = std::fs::File::create(&default_json_path).expect("Can not create file");
                    println!("Writing config to file {}", &default_json_path);
                    main_json.write(&mut file).unwrap();
                },
                "n" => {
                    println!("Exiting...");
                    std::process::exit(1)
                },
                _ => {
                    println!("Exiting...");
                    std::process::exit(1)
                }
            }
        } else {
            let main_json = createjson::create_json();
            let mut file = std::fs::File::create(&default_json_path).expect("Can not create file");
            main_json.write(&mut file).unwrap();
        }
        std::process::exit(0);
    } else if matches.is_present("json_file") {
        configuration = parse_json_file(matches.value_of("json_file").unwrap());
    }

    if configuration == json::JsonValue::Null {
        configuration = parse_json_file("/etc/ohmyoled/ohmyoled.json");
    }
    // run_python(&configuration);
    let mut module_map: HashMap<String, ModuleValue> = HashMap::new();
    get_modules(&configuration, &mut module_map);
    
}