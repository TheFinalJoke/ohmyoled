mod createjson;
mod filelib;
extern crate log;
use clap::{Arg, ArgAction, Command};
use env_logger::Env;
use oledlib::modules::{registry, scheduler};
use ohmyoled_matrix::{MatrixOptions, RGBMatrix};

fn parse_json_file(file: &str) -> serde_json::Value {
    let contents = match filelib::open_file(file) {
        Err(e) => {
            println!("File: {} failed: {}", file, e);
            std::process::exit(2);
        }
        Ok(returned) => returned,
    };
    serde_json::from_str(&contents).unwrap_or_else(|e| {
        println!("Failed to parse {file}: {e}");
        std::process::exit(32)
    })
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

// Async-signal-safe SIGINT handler.
// Writes a message via raw libc::write, then calls libc::_exit to bypass
// the tokio runtime's shutdown path so in-flight HTTP fetches don't panic.
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

#[derive(serde::Deserialize)]
struct ParsedConfig {
    matrix_options: createjson::MatrixOptions,
}

#[tokio::main]
async fn main() {
    init_logger();
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
        println!("Building a dev environment, Replacing /etc/ohmyoled/ohmyoled.json with a dev json");
        let main_json = createjson::create_json(true);
        if filelib::check_if_exists(default_json_path) {
            std::fs::remove_file(default_json_path).expect("Can not Remove file");
        }
        ensure_config_dir(default_json_path);
        let file = std::fs::File::create(default_json_path).expect("Can not create file");
        serde_json::to_writer_pretty(file, &main_json).unwrap();
        println!("Wrote dev json to {}", default_json_path);
        return;
    }

    if matches.get_flag("create_json") {
        let default_json_path = "/etc/ohmyoled/ohmyoled.json";
        if filelib::check_if_exists(default_json_path) {
            println!("Would you like to overwrite ({})? (y/n)", &default_json_path);
            match oledlib::get_input().unwrap().to_lowercase().as_str() {
                "y" => {
                    let main_json = createjson::create_json(false);
                    std::fs::remove_file(default_json_path).expect("Can not Remove file");
                    ensure_config_dir(default_json_path);
                    let file = std::fs::File::create(default_json_path).expect("Can not create file");
                    serde_json::to_writer_pretty(file, &main_json).expect("write failed");
                    println!("Wrote changes to File: {}", default_json_path);
                }
                _ => {
                    println!("Exiting...");
                    std::process::exit(1)
                }
            }
        } else {
            let main_json = createjson::create_json(false);
            ensure_config_dir(default_json_path);
            let file = std::fs::File::create(default_json_path).expect("Can not create file");
            serde_json::to_writer_pretty(file, &main_json).unwrap();
        }
        return;
    }

    let configuration: serde_json::Value = if let Some(json_file) = matches.get_one::<String>("json_file") {
        parse_json_file(json_file)
    } else {
        parse_json_file("/etc/ohmyoled/ohmyoled.json")
    };

    let parsed: ParsedConfig = serde_json::from_value(configuration.clone()).unwrap_or_else(|e| {
        println!("Failed to deserialize config: {e}");
        std::process::exit(33);
    });
    let registry_cfg: registry::RegistryConfig =
        serde_json::from_value(configuration).unwrap_or_else(|e| {
            println!("Failed to deserialize registry config: {e}");
            std::process::exit(34);
        });
    let dev = std::env::var("DEV").is_ok();

    let matrix = build_matrix(&parsed.matrix_options, dev);
    let modules = registry::build(&registry_cfg).await;

    install_sigint_handler();
    if let Err(e) = scheduler::run(matrix, modules).await {
        eprintln!("scheduler: {e}");
        unsafe { libc::_exit(1) };
    }
}
