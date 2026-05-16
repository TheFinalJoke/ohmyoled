mod createjson;
mod filelib;
extern crate log;
use clap::{Arg, ArgAction, Command};
use env_logger::Env;
use json;
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict, PyDictMethods, PyTuple};
use pyo3::Bound;

#[derive(Debug)]
struct ModuleApiConfiguration {
    matrix_options: createjson::MatrixOptions,
    time: Option<createjson::time::TimeOptions>,
    weather: Option<createjson::weather::WeatherOptions>,
    stock: Option<createjson::stock::StockOptions>,
    sport: Option<createjson::sport::SportOptions>,
}
impl IntoPyDict<'_> for ModuleApiConfiguration {
    fn into_py_dict(self, py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
        let pydict = PyDict::new(py);
        pydict.set_item("matrix_options", self.matrix_options.into_py_dict(py)?)?;
        if let Some(time) = self.time {
            pydict.set_item("time", time.into_py_dict(py)?)?;
        }
        if let Some(weather) = self.weather {
            pydict.set_item("weather", weather.into_py_dict(py)?)?;
        }
        if let Some(stock) = self.stock {
            pydict.set_item("stock", stock.into_py_dict(py)?)?;
        }
        if let Some(sport) = self.sport {
            pydict.set_item("sport", sport.into_py_dict(py)?)?;
        }
        Ok(pydict)
    }
}
impl ModuleApiConfiguration {
    pub fn new(j: &json::JsonValue) -> Self {
        Self {
            matrix_options: createjson::MatrixOptions::from_json(&j["matrix_options"]),
            time: None,
            weather: None,
            stock: None,
            sport: None,
        }
    }
}
fn parse_json(contents: &str) -> json::JsonValue {
    let parsed = match json::parse(contents) {
        Err(e) => {
            println!("{}", e);
            std::process::exit(32)
        }
        Ok(parse) => parse,
    };
    parsed
}
fn parse_json_file(file: &str) -> json::JsonValue {
    let contents = match filelib::open_file(file) {
        Err(e) => {
            println!("File: {} failed: {}", file, e);
            std::process::exit(2);
        }
        Ok(returned) => returned,
    };
    let final_parse = parse_json(&contents);
    final_parse
}
fn get_modules(json_config: &json::JsonValue) -> ModuleApiConfiguration {
    let mut module_config = ModuleApiConfiguration::new(json_config);
    for entry in json_config.entries() {
        match entry.0 {
            "time" => module_config.time = Some(createjson::time::TimeOptions::from_json(entry.1)),
            "weather" => {
                module_config.weather =
                    Some(createjson::weather::WeatherOptions::from_json(entry.1))
            }
            "stock" => {
                module_config.stock = Some(createjson::stock::StockOptions::from_json(entry.1))
            }
            "sport" => {
                module_config.sport = Some(createjson::sport::SportOptions::from_json(entry.1))
            }
            _ => (),
        }
    }
    module_config
}
fn ensure_config_dir(path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).expect("Can not create config directory");
    }
}

fn init_logger() {
    let env = Env::default()
        .filter_or("RUST_LOG", "error")
        .write_style_or("RUST_LOG_STYLE", "always");
    env_logger::init_from_env(env);
}

#[pyo3_async_runtimes::tokio::main]
async fn main() -> PyResult<()> {
    init_logger();
    let mut configuration = json::JsonValue::Null;
    let cmd = Command::new("ohmyoled").version("2.2.8");
    let args_vec = vec![
        Arg::new("create_json")
            .short('c')
            .long("create_json")
            .help("Creates a json for oled configuration")
            .action(ArgAction::SetTrue),
        Arg::new("json_file")
            .short('f')
            .long("json_file")
            .help("Pass a location of json file"),
        Arg::new("dev_mode")
            .long("dev")
            .help("creates a dev enviornment")
            .action(ArgAction::SetTrue),
    ];

    let cmd = cmd.args(args_vec);
    let matches = cmd.get_matches();
    if matches.get_flag("dev_mode") {
        let default_json_path = "/etc/ohmyoled/ohmyoled.json";
        println!(
            "Building a dev environment, Replacing /etc/ohmyoled/ohmyoled.json with a dev json"
        );
        let main_json = createjson::create_json(true);
        if filelib::check_if_exists(&default_json_path) {
            std::fs::remove_file(&default_json_path).expect("Can not Remove file");
        }
        ensure_config_dir(default_json_path);
        let mut file = std::fs::File::create(&default_json_path).expect("Can not create file");
        println!("Writing config to file {}", &default_json_path);
        main_json.write(&mut file).unwrap();
        println!("Wrote to {}, a dev json", default_json_path);
        std::process::exit(0);
    }
    if matches.get_flag("create_json") {
        let default_json_path = "/etc/ohmyoled/ohmyoled.json";
        if filelib::check_if_exists(&default_json_path) {
            println!(
                "Would you like to overwrite ({})? (y/n)",
                &default_json_path
            );
            match oledlib::get_input().unwrap().to_lowercase().as_str() {
                "y" => {
                    let main_json = createjson::create_json(false);
                    std::fs::remove_file(&default_json_path).expect("Can not Remove file");
                    ensure_config_dir(default_json_path);
                    let mut file =
                        std::fs::File::create(&default_json_path).expect("Can not create file");
                    println!("Writing config to file {}", &default_json_path);
                    match main_json.write(&mut file) {
                        Err(e) => {
                            println!("{}", e);
                            std::process::exit(30)
                        }
                        Ok(_) => {
                            println!("Wrote changes to File: {}", default_json_path);
                        }
                    };
                }
                _ => {
                    println!("Exiting...");
                    std::process::exit(1)
                }
            }
        } else {
            let main_json = createjson::create_json(false);
            ensure_config_dir(default_json_path);
            let mut file = std::fs::File::create(&default_json_path).expect("Can not create file");
            main_json.write(&mut file).unwrap();
        }
        std::process::exit(0);
    } else if matches.contains_id("json_file") {
        if let Some(json_file) = matches.get_one::<String>("json_file") {
            configuration = parse_json_file(json_file);
        }
    }

    if configuration == json::JsonValue::Null {
        configuration = parse_json_file("/etc/ohmyoled/ohmyoled.json");
    }
    let config_mod: ModuleApiConfiguration = get_modules(&configuration);
    let fut = Python::with_gil(|py| {
        let ohmyoled_import = py.import("ohmyoled.main")?;
        let args = PyTuple::new(py, &[config_mod.into_py_dict(py)?])?;
        let main = ohmyoled_import.getattr("Main")?.call1(&args)?;
        pyo3_async_runtimes::tokio::into_future(main.call_method0("main_run")?)
    })?;
    if let Err(e) = fut.await {
        // KeyboardInterrupt → clean exit (standard SIGINT code 130), no panic, no traceback.
        let is_interrupt = Python::with_gil(|py| {
            e.is_instance_of::<pyo3::exceptions::PyKeyboardInterrupt>(py)
        });
        if is_interrupt {
            eprintln!("Interrupted");
            std::process::exit(130);
        }
        return Err(e);
    }
    Ok(())
}
