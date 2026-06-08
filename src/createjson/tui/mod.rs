//! Full-screen ratatui config builder — the `-c` flow.
//!
//! The wizard has two screens: a **Setup** screen (pick the display target —
//! Matrix or E-ink — fill its options, pick the output format) and a
//! **Modules** screen (toggle the applicable tiles, edit their fields, watch a
//! live preview). The pure form/projection layer lives in [`field`]; the
//! per-section schema dispatch in [`form_module`]; the config assembly +
//! serialization in [`preview`]; and the terminal shell in [`app`]/[`event`]/
//! [`ui`].

pub mod app;
pub mod event;
pub mod field;
pub mod form_module;
pub mod preview;
pub mod ui;

use app::{App, ConfigFormat};
use ratatui::crossterm::{
    event::{self as cevent, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use serde_json::Value;
use std::io::{self, Stdout};

type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Run the full-screen wizard. Returns the assembled config plus the chosen
/// output format on save, or `None` if the user quit without saving. Callers
/// must have already confirmed they're attached to a TTY.
pub fn run(
    existing: Option<Value>,
    initial_fmt: Option<ConfigFormat>,
) -> io::Result<Option<(Value, ConfigFormat)>> {
    let mut terminal = init_terminal()?;
    // Restore the terminal even if a render/handler panics, so the user isn't
    // left in raw mode on the alternate screen.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        emergency_restore();
        prev_hook(info);
    }));

    let mut app = App::new(existing, initial_fmt);
    let loop_result = run_loop(&mut terminal, &mut app);

    let _ = std::panic::take_hook();
    restore_terminal(&mut terminal)?;
    loop_result?;
    Ok(app.result)
}

fn run_loop(terminal: &mut Tui, app: &mut App) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;
        if let Event::Key(key) = cevent::read()? {
            // Ignore key-release / repeat noise (Windows + some terminals send
            // both edges); only act on presses.
            if key.kind == KeyEventKind::Press {
                event::handle_key(app, key);
            }
        }
    }
    Ok(())
}

fn init_terminal() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Best-effort terminal reset used from the panic hook (no `Terminal` handle).
fn emergency_restore() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}
