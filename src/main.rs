mod config_io;
mod createjson;
mod filelib;
mod logger;
mod preview;
extern crate log;
use clap::{Arg, ArgAction, Command};
use oledlib::modules::{registry, scheduler};
use ohmyoled_matrix::{EinkDisplay, EinkMode, MatrixMode, MatrixOptions, RGBMatrix};

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
        hardware_mapping: cfg.hardware_mapping.clone(),
    };
    if dev {
        RGBMatrix::test(opts)
    } else {
        RGBMatrix::new(opts)
    }
}

/// Build an `EinkDisplay` from the parsed `eink` config block. Like
/// `build_matrix`, `dev` forces the terminal (hardware-free) backend.
fn build_eink_display(cfg: &registry::EinkRegistryConfig, dev: bool) -> EinkDisplay {
    let opts = cfg.options();
    if dev {
        EinkDisplay::test(opts)
    } else {
        EinkDisplay::new(opts)
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
    /// LED-panel geometry. Optional: an eink-only config doesn't need it (the
    /// defaults are used and the LED matrix isn't built unless it has tiles).
    #[serde(default)]
    matrix_options: createjson::MatrixOptions,
}

/// Pulls just the top-level `eink` block out of the config. Separate pass so
/// the LED `RegistryConfig` and `ParsedConfig` stay untouched and existing
/// configs (no `eink` key) parse to the disabled default.
#[derive(serde::Deserialize, Default)]
struct EinkTopConfig {
    #[serde(default)]
    eink: registry::EinkRegistryConfig,
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
        Arg::new("preview")
            .long("preview")
            .help("Render a single screen with built-in fake data and loop forever (no config or network). Names: time, weather, stock, stock_chart, sport, golf, f1, iss, quake, aurora, flights, launch, hass, pihole. Prefix with 'eink' for the e-paper display, e.g. 'eink:weather'")
            .value_name("NAME"),
        Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("Increase log verbosity above the info default: -v debug, -vv+ trace")
            .action(ArgAction::Count),
        Arg::new("log_file")
            .long("log-file")
            .help("Override log file path (default /var/ohmyoled/ohmyoled.log)"),
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

    // Preview mode — render one screen with fake data and loop. Bypasses
    // config loading entirely so it works on a fresh clone with no setup.
    if let Some(name) = matches.get_one::<String>("preview") {
        let dev = std::env::var("DEV").is_ok();
        install_sigint_handler();
        // `--preview eink` (or `eink:weather` / `eink_weather`) drives the
        // e-paper display instead of the LED matrix. Defaults to the weather
        // screen. Backend follows OHMYOLED_EINK_MODE (terminal off-Pi).
        if let Some(rest) = name.strip_prefix("eink") {
            let sub = rest.trim_start_matches([':', '_', '-']);
            let sub = if sub.is_empty() { "weather" } else { sub };
            let display = EinkDisplay::new(ohmyoled_matrix::EinkOptions::default());
            if let Err(e) = preview::run_eink(sub, display).await {
                eprintln!("preview: {e}");
                std::process::exit(2);
            }
            return;
        }
        let matrix = build_matrix(&createjson::MatrixOptions::default(), dev);
        if let Err(e) = preview::run(name, matrix).await {
            eprintln!("preview: {e}");
            std::process::exit(2);
        }
        return;
    }

    if matches.get_flag("dev_mode") {
        println!("Building a dev environment, replacing {target_path} with a dev config");
        let main_json = createjson::create_json(true, None);
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
            println!(
                "Config exists at {}. (m)erge into existing entries, (o)verwrite, or (c)ancel? [m]",
                &target_path
            );
            let raw = oledlib::get_input().unwrap_or_default();
            let choice = raw.trim().to_lowercase();
            let choice = if choice.is_empty() { "m" } else { choice.as_str() };
            match choice {
                "m" | "merge" => {
                    let existing = parse_config_file(&target_path);
                    let main_json = createjson::create_json(false, Some(existing));
                    if main_json.get("failure").and_then(|v| v.as_bool()).unwrap_or(false) {
                        return;
                    }
                    std::fs::remove_file(&target_path).expect("Can not Remove file");
                    ensure_config_dir(&target_path);
                    config_io::write(&target_path, &main_json).expect("write failed");
                    println!("Updated config at {target_path}");
                }
                "o" | "overwrite" => {
                    let main_json = createjson::create_json(false, None);
                    if main_json.get("failure").and_then(|v| v.as_bool()).unwrap_or(false) {
                        return;
                    }
                    std::fs::remove_file(&target_path).expect("Can not Remove file");
                    ensure_config_dir(&target_path);
                    config_io::write(&target_path, &main_json).expect("write failed");
                    println!("Wrote changes to File: {target_path}");
                }
                _ => {
                    println!("Cancelled — config not changed");
                    std::process::exit(0)
                }
            }
        } else {
            let main_json = createjson::create_json(false, None);
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

    // `serde_path_to_error` wraps the deserialize so failures report
    // the JSON path that broke (e.g. `stock[2].api: unknown variant
    // 'coingecko'`). The path is what makes the error actionable;
    // without it the user has to bisect the config to find the bad
    // section. Cost is one tiny dep, well worth it.
    let parsed: ParsedConfig =
        serde_path_to_error::deserialize(configuration.clone()).unwrap_or_else(|e| {
            println!("Failed to deserialize config at {}: {}", e.path(), e.inner());
            std::process::exit(33);
        });
    let eink_top: EinkTopConfig = serde_path_to_error::deserialize(configuration.clone())
        .unwrap_or_else(|e| {
            println!("Failed to deserialize eink config at {}: {}", e.path(), e.inner());
            std::process::exit(35);
        });
    let registry_cfg: registry::RegistryConfig = serde_path_to_error::deserialize(configuration)
        .unwrap_or_else(|e| {
            println!(
                "Failed to deserialize registry config at {}: {}",
                e.path(),
                e.inner()
            );
            std::process::exit(34);
        });
    let dev = std::env::var("DEV").is_ok();
    install_sigint_handler();

    // The LED matrix and the e-paper display are independent outputs that can
    // run at the same time (they're separate physical panels). `eink.enabled`
    // turns the e-paper side on; the LED side runs whenever it has tiles to
    // show. Either, both, or neither may be active.
    let eink_enabled = eink_top.eink.enabled;

    // ── e-paper side (opt-in) ───────────────────────────────────────────
    let eink = if eink_enabled {
        log::info!(
            "eink display enabled (model={}, threshold={})",
            eink_top.eink.model,
            eink_top.eink.threshold
        );
        let display = build_eink_display(&eink_top.eink, dev);
        let dims = (display.width(), display.height());
        let modules = registry::build_eink(&eink_top.eink.modules, dims).await;
        log::info!("eink registry built: {} module(s) active", modules.len());
        Some((display, modules))
    } else {
        None
    };

    // ── LED side (default) ──────────────────────────────────────────────
    let led_modules = registry::build(&registry_cfg).await;
    log::info!("LED registry built: {} module(s) active", led_modules.len());
    // Drive the LED panel when it has work, or when eink isn't taking over (so
    // a bare config still behaves as before). When eink is the only thing with
    // tiles, skip the LED path so we don't spin an idle loop — and, in dev
    // mode, so it doesn't clobber the eink terminal output.
    let run_led = !led_modules.is_empty() || !eink_enabled;
    let matrix = if run_led {
        log::debug!(
            "matrix configured: chain={} parallel={} brightness={} slowdown={} mapping={}",
            parsed.matrix_options.chain_length,
            parsed.matrix_options.parallel,
            parsed.matrix_options.brightness,
            parsed.matrix_options.oled_slowdown,
            parsed.matrix_options.hardware_mapping,
        );
        Some(build_matrix(&parsed.matrix_options, dev))
    } else {
        None
    };

    // Both outputs at once is supported. The one real conflict is when *both*
    // resolve to their terminal/test backend: they'd share this single stdout
    // and interleave into unreadable noise. Flag it rather than silently
    // mangling the preview — on real hardware they drive separate panels.
    if let (Some((display, _)), Some(m)) = (eink.as_ref(), matrix.as_ref()) {
        if display.mode == EinkMode::Terminal && m.mode == MatrixMode::Test {
            log::warn!(
                "both LED matrix (test) and e-paper (terminal) are active and share this terminal — \
                 their output will interleave. On hardware they drive separate panels; to preview \
                 just one, disable the other (eink.enabled / remove LED tiles)."
            );
        } else {
            log::info!("driving LED matrix and e-paper display concurrently");
        }
    }

    // ── dispatch ────────────────────────────────────────────────────────
    match (eink, matrix) {
        // Both panels: run the two schedulers concurrently and exit if either
        // returns (both normally loop forever).
        (Some((display, emods)), Some(m)) => {
            let (er, lr) =
                tokio::join!(scheduler::run_eink(display, emods), scheduler::run(m, led_modules));
            if let Err(e) = er {
                eprintln!("eink scheduler: {e}");
            }
            if let Err(e) = lr {
                eprintln!("scheduler: {e}");
            }
            unsafe { libc::_exit(1) };
        }
        // e-paper only.
        (Some((display, emods)), None) => {
            if let Err(e) = scheduler::run_eink(display, emods).await {
                eprintln!("eink scheduler: {e}");
                unsafe { libc::_exit(1) };
            }
        }
        // LED only (the default path).
        (None, Some(m)) => {
            if let Err(e) = scheduler::run(m, led_modules).await {
                eprintln!("scheduler: {e}");
                unsafe { libc::_exit(1) };
            }
        }
        // eink disabled forces run_led=true, so this is unreachable; handle it
        // defensively rather than panicking.
        (None, None) => log::warn!("no display active; nothing to run"),
    }
}
