mod createjson;
mod filelib;
extern crate log;
use clap::{Arg, ArgAction, Command};
use env_logger::Env;
use json;
use ohmyoled_matrix::modules::TimeMatrix;
use ohmyoled_matrix::{Color, MatrixOptions, RGBMatrix};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict, PyDictMethods, PyTuple};
use pyo3::Bound;
use std::time::Duration;

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

/// Build an `RGBMatrix` from the workspace `MatrixOptions` JSON section.
///
/// Mirrors what `Main.poll_rgbmatrix()` did in Python, but stays on the Rust
/// side so pure-Rust modules (e.g. `TimeMatrix`) can drive the panel without
/// going through pyo3.
fn build_matrix(cfg: &createjson::MatrixOptions, dev: bool) -> RGBMatrix {
    let opts = MatrixOptions {
        cols: 64,
        rows: 32,
        chain_length: cfg.chain_length.max(1) as u32,
        parallel: cfg.parallel.max(1) as u32,
        gpio_slowdown: cfg.oled_slowdown.max(0) as u32,
        brightness: cfg.brightness.max(0) as u32,
        hardware_mapping: "adafruit-hat".to_string(),
    };
    if dev {
        RGBMatrix::test(opts)
    } else {
        RGBMatrix::new(opts)
    }
}

/// Run the Rust-native `TimeMatrix` for one cycle (30 frames at 1s).
///
/// Replaces the deleted Python `ohmyoled.matrix.time.TimeMatrix`. The Python
/// `Main.main_run` loop no longer dispatches `time`; we handle it here before
/// handing off to Python for weather/stock/sport.
async fn run_time_module(time_cfg: &createjson::time::TimeOptions, dev: bool, matrix_cfg: &createjson::MatrixOptions) {
    let (r, g, b) = time_cfg.color;
    let color = Color::new(r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, b.clamp(0, 255) as u8);

    let tm = match TimeMatrix::new(color, None) {
        Ok(tm) => tm,
        Err(e) => {
            log::error!("Skipping time module: failed to load font ({e})");
            return;
        }
    };

    let mut matrix = build_matrix(matrix_cfg, dev);
    tm.run_async(&mut matrix, 30, Duration::from_secs(1)).await;
    matrix.clear();
}

// Async-signal-safe SIGINT handler.
// Writes a message via raw libc::write, then calls libc::_exit to bypass
// atexit handlers — std::process::exit triggers Python/pyo3 atexit code that
// panics during shutdown of in-flight async tasks.
extern "C" fn sigint_handler(_: std::os::raw::c_int) {
    unsafe {
        let msg = b"\nInterrupted\n";
        libc::write(libc::STDERR_FILENO, msg.as_ptr() as *const _, msg.len());
        libc::_exit(130);
    }
}

fn install_sigint_handler() {
    unsafe {
        libc::signal(libc::SIGINT, sigint_handler as *const () as libc::sighandler_t);
    }
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
    let dev = std::env::var("DEV").is_ok();

    // Pure-Rust time module: render before Python takes over so we don't share
    // the matrix concurrently. Python's `Main.main_run` no longer dispatches time.
    if let Some(ref time_cfg) = config_mod.time {
        if time_cfg.run {
            run_time_module(time_cfg, dev, &config_mod.matrix_options).await;
        }
    }

    let fut = Python::with_gil(|py| {
        let ohmyoled_import = py.import("ohmyoled.main")?;
        let args = PyTuple::new(py, &[config_mod.into_py_dict(py)?])?;
        let main = ohmyoled_import.getattr("Main")?.call1(&args)?;
        pyo3_async_runtimes::tokio::into_future(main.call_method0("main_run")?)
    })?;
    // Install AFTER Python init so we override Python's default SIGINT handler.
    // Ctrl+C now calls libc::_exit(130) directly — no atexit, no panic.
    install_sigint_handler();
    match fut.await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{e}");
            unsafe { libc::_exit(1); }
        }
    }
}
