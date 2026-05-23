//! Shared terminal-UI helpers for the interactive config builder.
//!
//! ANSI colors are emitted only when stdout is a TTY and `NO_COLOR` is unset.
//! Falls back to plain text in non-interactive environments (tests, pipes).

use std::io::{self, IsTerminal, Write};

fn supports_color() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn paint(s: &str, code: &str) -> String {
    if supports_color() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn bold(s: &str) -> String { paint(s, "1") }
pub fn dim(s: &str) -> String { paint(s, "2") }
pub fn cyan(s: &str) -> String { paint(s, "36") }
pub fn green(s: &str) -> String { paint(s, "32") }
pub fn yellow(s: &str) -> String { paint(s, "33") }
pub fn red(s: &str) -> String { paint(s, "31") }
pub fn magenta(s: &str) -> String { paint(s, "35") }

const BANNER_WIDTH: usize = 52;

pub fn banner(title: &str, subtitle: &str) {
    let line = "═".repeat(BANNER_WIDTH);
    let title_line = format!("║{:^width$}║", title, width = BANNER_WIDTH);
    let subtitle_line = format!("║{:^width$}║", subtitle, width = BANNER_WIDTH);
    println!();
    println!("{}", cyan(&format!("╔{line}╗")));
    println!("{}", cyan(&title_line));
    println!("{}", cyan(&subtitle_line));
    println!("{}", cyan(&format!("╚{line}╝")));
    println!();
}

pub fn section(title: &str) {
    let dash = "─".repeat(BANNER_WIDTH);
    println!();
    println!("  {}", bold(&cyan(title)));
    println!("  {}", dim(&dash));
}

pub fn info(msg: &str) { println!("  {} {msg}", cyan("›")); }
pub fn success(msg: &str) { println!("  {} {msg}", green("✓")); }
pub fn warn(msg: &str) { println!("  {} {msg}", yellow("!")); }
pub fn error(msg: &str) { println!("  {} {msg}", red("✗")); }
pub fn hint(msg: &str) { println!("    {}", dim(msg)); }

/// Print the prompt arrow + label, then read a single trimmed line.
/// Returns `None` if the user submitted an empty line.
pub fn read_line(label: &str) -> Option<String> {
    print!("  {} {label} {} ", cyan("?"), bold("›"));
    let _ = io::stdout().flush();
    oledlib::get_input()
}

/// Like `read_line` but applies `default` on empty input. The default is shown
/// after the label in dim text.
pub fn read_line_default(label: &str, default: &str) -> String {
    let decorated = format!("{label} {}", dim(&format!("[{default}]")));
    read_line(&decorated).unwrap_or_else(|| default.to_string())
}

/// Read a required string. Re-prompts until non-empty input is supplied.
pub fn read_required(label: &str) -> String {
    loop {
        match read_line(label) {
            Some(s) => return s,
            None => warn("A value is required"),
        }
    }
}

pub fn read_yes_no(label: &str, default: bool) -> bool {
    let hint = if default { "Y/n" } else { "y/N" };
    let decorated = format!("{label} {}", dim(&format!("[{hint}]")));
    loop {
        match read_line(&decorated) {
            None => return default,
            Some(s) => match s.trim().to_lowercase().as_str() {
                "y" | "yes" => return true,
                "n" | "no" => return false,
                _ => warn("Please answer y or n"),
            },
        }
    }
}

/// Pick one of `choices` (slug, description). The first matching slug wins
/// (case-insensitive). Empty input falls back to `default`.
pub fn choose(label: &str, choices: &[(&str, &str)], default: &str) -> String {
    println!("  {} {}", cyan("?"), bold(label));
    for (slug, desc) in choices {
        let marker = if *slug == default { green("●") } else { dim("○") };
        println!("      {marker} {} {}", bold(slug), dim(&format!("— {desc}")));
    }
    loop {
        let labeled = format!("Choice {}", dim(&format!("[{default}]")));
        let raw = match read_line(&labeled) {
            None => return default.to_string(),
            Some(s) => s.trim().to_lowercase(),
        };
        if let Some((slug, _)) = choices.iter().find(|(s, _)| s.eq_ignore_ascii_case(&raw)) {
            return (*slug).to_string();
        }
        warn(&format!("'{raw}' is not a valid option"));
    }
}

/// Render the running summary of selected entries above the menu.
pub fn summary(entries: &[String]) {
    println!("  {}", bold(&magenta("Pending configuration")));
    let dash = "─".repeat(BANNER_WIDTH);
    println!("  {}", dim(&dash));
    if entries.is_empty() {
        println!("    {}", dim("(no modules selected yet)"));
    } else {
        for (idx, line) in entries.iter().enumerate() {
            println!("    {} {line}", dim(&format!("{:>2}.", idx + 1)));
        }
    }
    println!();
}
