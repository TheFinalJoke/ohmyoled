mod filelib;
mod createjson;
extern crate log;
use env_logger::{Env};
use clap::{Arg, App};
use json;
use pyo3::prelude::*;
use pyo3::types::{PyDict, IntoPyDict};
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
fn to_hashmap(conf: &json::JsonValue) -> HashMap<String, &json::JsonValue> {
    let mut converted = HashMap::new();
    for (s, entry) in conf.entries() {
        converted.insert(s.to_string(), entry);
    }
    converted
}

fn run_python(conf: &json::JsonValue) -> PyResult<()> {
    /*
    let gil = Python::acquire_gil();
    let py = gil.python();
    let sys = py.import("sys")?;
    let version: String = sys.get("version")?.extract()?;

    let locals = [("os", py.import("os")?)].into_py_dict(py);
    let code = "os.getenv('USER') or os.getenv('USERNAME') or 'Unknown'";
    let user: String = py.eval(code, None, Some(&locals))?.extract()?;
    let dict = createjson::weather::WeatherOptions::_from_json(&conf).into_py_dict(py);
    println!("Hello {}, I'm Python {}, python_dict {}", user, version, dict);

    // OKAY SO EACH Configuration needs to be implement intoPyDict
    // 
    // https://pyo3.rs/v0.15.1/ecosystem/async-await.html
    let fut = Python::with_gil(|py| {
        let asyncio = py.import("asyncio")?;
        // convert asyncio.sleep into a Rust Future
        pyo3_asyncio::tokio::into_future(asyncio.call_method1("sleep", (1.into_py(py),))?)
    })?;
    fut.await?;
    Ok(())
    */
        let gil = Python::acquire_gil();
        let py = gil.python();
        let wapi_import = py.import("ohmyoled.lib.weather.normal.WeatherApi").unwrap();
        dbg!(wapi_import);
        //let weather_api_class = wapi_import.getattr("WeatherApi").unwrap();
        //dbg!(weather_api_class);
        //let options = createjson::weather::WeatherOptions::_from_json(&conf).to_python_dict(py);
        //let weather_api_object = weather_api_class.call1(options);
        //dbg!(weather_api_object);
        //let config = PyTuple::new(py, [conf.dump()]);       
        //let weather_api_object = weather_api_class.call1(config).unwrap();
        //dbg!(weather_api_object);
        //dbg!(weather_api_object.call_method0("parse_config_file").unwrap());
    Ok(())

}


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
        Arg::new("dev_mode")
        .long("dev")
        .help("creates a dev enviornment")
    ];
    
    let app = app.args(args_vec);
    let matches = app.get_matches();
    if matches.is_present("dev_mode") {
        let default_json_path = "/etc/ohmyoled/ohmyoled.json";
        println!("Building a dev environment, Replacing /etc/ohmyoled/ohmyoled.json with a dev json");
        let main_json = createjson::create_json(true);
        std::fs::remove_file(&default_json_path).expect("Can not Remove file");
        let mut file = std::fs::File::create(&default_json_path).expect("Can not create file");
        println!("Writing config to file {}", &default_json_path);
        main_json.write(&mut file).unwrap();
        println!("Wrote to {}, a dev json", default_json_path);
        std::process::exit(0);
    }
    if matches.is_present("create_json") { // make an array of options
        let default_json_path = "/etc/ohmyoled/ohmyoled.json";
        if filelib::check_if_exists(&default_json_path){
            println!("Would you like to overwrite ({})? (y/n)", &default_json_path);
            match oledlib::get_input().unwrap().to_lowercase().as_str() {
                "y" => {
                    let main_json = createjson::create_json(false);
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
            let main_json = createjson::create_json(false);
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
    run_python(&configuration);
    let mut module_map: HashMap<String, ModuleValue> = HashMap::new();
    get_modules(&configuration, &mut module_map);
    
}