//! Dual-target logger.
//!
//! Every record is fanned to stderr (with the env_logger formatter) and to a
//! persistent file. `/var/ohmyoled/ohmyoled.log` is the default sink so an
//! installed systemd unit can `tail -F` it without rebuilding the binary. A
//! `--log-file <path>` override is honored for dev environments where the
//! default path isn't writable.
//!
//! Verbosity is driven by the `-v` count on the CLI:
//!
//! | flag    | level   |
//! |---------|---------|
//! | (none)  | `warn`  |
//! | `-v`    | `info`  |
//! | `-vv`   | `debug` |
//! | `-vvv+` | `trace` |
//!
//! `RUST_LOG` is still honored on top of the CLI level for per-module overrides
//! (e.g. `RUST_LOG=oledlib::api::weather=trace`).

use env_logger::{Builder, Target};
use log::LevelFilter;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const DEFAULT_LOG_FILE: &str = "/var/ohmyoled/ohmyoled.log";

/// Map the `-v` repeat count to a [`LevelFilter`].
pub fn level_for(verbosity: u8) -> LevelFilter {
    match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

/// Writer that fans every byte to stderr and (optionally) a file. The file is
/// behind a `Mutex` so the logger stays `Send`. File-side errors are swallowed
/// — a busted log target must never tear down the panel.
struct FanWriter {
    file: Option<Mutex<File>>,
}

impl Write for FanWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = io::stderr().write(buf)?;
        if let Some(handle) = &self.file {
            if let Ok(mut f) = handle.lock() {
                let _ = f.write_all(buf);
            }
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = io::stderr().flush();
        if let Some(handle) = &self.file {
            if let Ok(mut f) = handle.lock() {
                let _ = f.flush();
            }
        }
        Ok(())
    }
}

fn open_log_file(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Initialize the global logger. Idempotent — calling twice in the same process
/// will panic via env_logger's underlying `set_logger`; we only call this once
/// from `main`.
pub fn init(verbosity: u8, file_override: Option<&str>) {
    let level = level_for(verbosity);
    let path = file_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LOG_FILE));

    let (file_handle, open_err) = match open_log_file(&path) {
        Ok(f) => (Some(Mutex::new(f)), None),
        Err(e) => (None, Some(e)),
    };
    let file_active = file_handle.is_some();
    let writer = FanWriter { file: file_handle };

    Builder::new()
        .filter_level(level)
        .parse_default_env()
        .format(|buf, record| {
            let ts = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f");
            writeln!(
                buf,
                "{ts} [{:<5}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .target(Target::Pipe(Box::new(writer)))
        .init();

    if let Some(e) = open_err {
        log::warn!(
            "logger: file output disabled ({}): {e}. Run as root, fix perms (chmod 666 {} or chown), or pass --log-file <path>",
            path.display(),
            path.display()
        );
    } else if file_active {
        log::info!("logger: level={level} file={}", path.display());
    }
}
