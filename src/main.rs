mod config_io;
mod createjson;
mod filelib;
mod logger;
extern crate log;
use clap::{Arg, ArgAction, Command};
use oledlib::modules::{registry, scheduler};
use ohmyoled_matrix::{MatrixOptions, RGBMatrix};

fn parse_config_file(file: &str) -> serde_json::Value {
    config_io::load(file).unwrap_or_else(|e| {
        println!("Failed to load {file}: {e}");
        std::process::exit(32);
    })
}

/// Resolve the default config path, preferring an existing file across
/// known formats so the user can keep their config in their format of choice
/// without passing `-f` every time.
fn default_config_path() -> String {
    const CANDIDATES: &[&str] = &[
        "/etc/ohmyoled/ohmyoled.json",
        "/etc/ohmyoled/ohmyoled.yaml",
        "/etc/ohmyoled/ohmyoled.yml",
        "/etc/ohmyoled/ohmyoled.toml",
    ];
    for c in CANDIDATES {
        if filelib::check_if_exists(c) {
            return c.to_string();
        }
    }
    CANDIDATES[0].to_string()
}

fn ensure_config_dir(path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).expect("Can not create config directory");
    }
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
    let cmd = Command::new("ohmyoled").version(env!("CARGO_PKG_VERSION"));
    let args_vec = vec![
        Arg::new("create_config")
            .short('c')
            .long("create_config")
            .alias("create_json")
            .help("Interactively builds a new config file (format chosen by --config extension)")
            .action(ArgAction::SetTrue),
        Arg::new("init_config")
            .long("init-config")
            .alias("init_config")
            .help("Write a non-interactive starter config to PATH (json/yaml/toml — format from extension)")
            .value_name("PATH"),
        Arg::new("json_file")
            .short('f')
            .long("config")
            .alias("json_file")
            .help("Path to config file (json/yaml/toml — format chosen by extension)"),
        Arg::new("dev_mode")
            .long("dev")
            .help("creates a dev enviornment")
            .action(ArgAction::SetTrue),
        Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("Increase log verbosity: -v info, -vv debug, -vvv+ trace")
            .action(ArgAction::Count),
        Arg::new("log_file")
            .long("log-file")
            .help("Override log file path (default /var/log/ohmyoled.log)"),
    ];

    let cmd = cmd.args(args_vec);
    let matches = cmd.get_matches();

    let verbosity = matches.get_count("verbose");
    let log_file = matches.get_one::<String>("log_file").map(String::as_str);
    logger::init(verbosity, log_file);
    log::info!("ohmyoled v{} starting", env!("CARGO_PKG_VERSION"));

    // For -c/--dev, write to the path passed via -f if given; otherwise default JSON.
    let target_path = matches
        .get_one::<String>("json_file")
        .cloned()
        .unwrap_or_else(|| "/etc/ohmyoled/ohmyoled.json".to_string());

    if matches.get_flag("dev_mode") {
        println!("Building a dev environment, replacing {target_path} with a dev config");
        let main_json = createjson::create_json(true);
        if filelib::check_if_exists(&target_path) {
            std::fs::remove_file(&target_path).expect("Can not Remove file");
        }
        ensure_config_dir(&target_path);
        config_io::write(&target_path, &main_json).expect("write failed");
        println!("Wrote dev config to {target_path}");
        return;
    }

    // Non-interactive starter config — for users who downloaded a release
    // binary and want a config template without running through the
    // interactive `-c` flow.
    if let Some(init_path) = matches.get_one::<String>("init_config") {
        if filelib::check_if_exists(init_path) {
            eprintln!("init-config: refusing to overwrite existing file at {init_path}");
            std::process::exit(1);
        }
        let main_json = createjson::default_config();
        ensure_config_dir(init_path);
        config_io::write(init_path, &main_json).expect("write failed");
        println!("Wrote starter config to {init_path}");
        return;
    }

    if matches.get_flag("create_config") {
        if filelib::check_if_exists(&target_path) {
            println!("Would you like to overwrite ({})? (y/n)", &target_path);
            match oledlib::get_input().unwrap().to_lowercase().as_str() {
                "y" => {
                    let main_json = createjson::create_json(false);
                    std::fs::remove_file(&target_path).expect("Can not Remove file");
                    ensure_config_dir(&target_path);
                    config_io::write(&target_path, &main_json).expect("write failed");
                    println!("Wrote changes to File: {target_path}");
                }
                _ => {
                    println!("Exiting...");
                    std::process::exit(1)
                }
            }
        } else {
            let main_json = createjson::create_json(false);
            ensure_config_dir(&target_path);
            config_io::write(&target_path, &main_json).expect("write failed");
        }
        return;
    }

    let configuration: serde_json::Value = if let Some(json_file) = matches.get_one::<String>("json_file") {
        log::info!("loading config from {json_file}");
        parse_config_file(json_file)
    } else {
        let path = default_config_path();
        log::info!("loading default config from {path}");
        parse_config_file(&path)
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
    log::debug!(
        "matrix configured: chain={} parallel={} brightness={} slowdown={}",
        parsed.matrix_options.chain_length,
        parsed.matrix_options.parallel,
        parsed.matrix_options.brightness,
        parsed.matrix_options.oled_slowdown
    );
    let modules = registry::build(&registry_cfg).await;
    log::info!("registry built: {} module(s) active", modules.len());

    install_sigint_handler();
    if let Err(e) = scheduler::run(matrix, modules).await {
        eprintln!("scheduler: {e}");
        unsafe { libc::_exit(1) };
    }
}
