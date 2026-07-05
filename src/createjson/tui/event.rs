//! Key handling for the two wizard screens. Pure state mutation over [`App`] —
//! no rendering. Text-like fields forward their keystrokes to `tui-input`;
//! everything else is a navigation or command key.

use super::app::{App, Focus, Screen, Target};
use super::field::{FieldKind, FieldValue, Form};
use super::{form_module, preview};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

/// Top-level dispatch. `Ctrl-C` always quits without saving.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        app.should_quit = true;
        app.result = None;
        return;
    }
    match app.screen {
        Screen::Setup => setup_keys(app, key),
        Screen::Modules => modules_keys(app, key),
        Screen::ConfirmQuit => confirm_keys(app, key),
    }
}

// --- Setup screen -----------------------------------------------------------

/// Row count on the Setup screen: target radio + the target form's visible
/// fields + format radio.
fn setup_rows(app: &App) -> usize {
    2 + app.target_form.visible_indices().len()
}

fn setup_keys(app: &mut App, key: KeyEvent) {
    let rows = setup_rows(app);
    let last = rows - 1;
    match key.code {
        KeyCode::Esc => app.screen = Screen::ConfirmQuit,
        KeyCode::Enter => {
            app.screen = Screen::Modules;
            app.preview_fmt = app.format;
            app.focus = Focus::List;
            app.status.clear();
        }
        KeyCode::Up | KeyCode::BackTab => {
            app.setup_idx = app.setup_idx.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Tab => {
            app.setup_idx = (app.setup_idx + 1).min(last);
        }
        _ => {
            if app.setup_idx == 0 {
                // Target radio.
                if matches!(
                    key.code,
                    KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
                ) {
                    app.target = match app.target {
                        Target::Matrix => Target::Eink,
                        Target::Eink => Target::Matrix,
                    };
                    app.rebuild_target_form();
                    app.setup_idx = app.setup_idx.min(setup_rows(app) - 1);
                }
            } else if app.setup_idx == last {
                // Format radio.
                if matches!(
                    key.code,
                    KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
                ) {
                    app.format = app.format.next();
                    app.preview_fmt = app.format;
                }
            } else {
                // A target-form field.
                let fidx = app.setup_idx - 1;
                edit_field(&mut app.target_form, fidx, key);
            }
        }
    }
}

// --- Modules screen ---------------------------------------------------------

fn modules_keys(app: &mut App, key: KeyEvent) {
    // Commands available regardless of focus.
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') => {
                save(app);
                return;
            }
            KeyCode::Char('q') => {
                app.screen = Screen::ConfirmQuit;
                return;
            }
            _ => {}
        }
    }
    if matches!(key.code, KeyCode::F(2)) {
        app.preview_fmt = app.preview_fmt.next();
        return;
    }
    match app.focus {
        Focus::List => modules_list_keys(app, key),
        Focus::Form => modules_form_keys(app, key),
    }
}

fn modules_list_keys(app: &mut App, key: KeyEvent) {
    let kind = app.selected_kind();
    let inst_count = app.instances_of(kind).len();
    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::Setup;
        }
        KeyCode::Up => {
            app.kind_idx = app.kind_idx.saturating_sub(1);
            app.inst_idx = 0;
            app.field_idx = 0;
        }
        KeyCode::Down => {
            app.kind_idx = (app.kind_idx + 1).min(form_module::TILE_KINDS.len() - 1);
            app.inst_idx = 0;
            app.field_idx = 0;
        }
        KeyCode::Left => {
            app.inst_idx = app.inst_idx.saturating_sub(1);
        }
        KeyCode::Right if inst_count > 0 => {
            app.inst_idx = (app.inst_idx + 1).min(inst_count - 1);
        }
        KeyCode::Char(' ') => {
            if app.kind_enabled(kind) {
                app.remove_kind(kind);
                app.inst_idx = 0;
            } else {
                app.add_instance(kind);
            }
        }
        KeyCode::Char('a') => {
            // Most tiles may repeat — sections fold to a JSON array on save.
            // `sleep` is a single top-level object, so exactly one.
            if !form_module::allow_multi(kind) && app.kind_enabled(kind) {
                app.status = format!("Only one {} allowed", form_module::title(kind));
            } else {
                app.add_instance(kind);
                app.inst_idx = app.instances_of(kind).len().saturating_sub(1);
                app.status = format!(
                    "Added {} #{} — ←/→ to switch, d to remove",
                    form_module::title(kind),
                    app.inst_idx + 1
                );
            }
        }
        KeyCode::Char('d') if inst_count > 0 => {
            app.remove_active_instance();
            let new_count = app.instances_of(kind).len();
            app.inst_idx = app.inst_idx.min(new_count.saturating_sub(1));
        }
        KeyCode::Enter | KeyCode::Tab => {
            if app.kind_enabled(kind) {
                app.focus = Focus::Form;
                app.field_idx = 0;
            } else {
                app.status = "Press space to enable this tile first".to_string();
            }
        }
        _ => {}
    }
}

fn modules_form_keys(app: &mut App, key: KeyEvent) {
    let visible_len = app
        .active_form_mut()
        .map(|f| f.visible_indices().len())
        .unwrap_or(0);
    match key.code {
        KeyCode::Esc => {
            app.focus = Focus::List;
            return;
        }
        KeyCode::Up | KeyCode::BackTab => {
            app.field_idx = app.field_idx.saturating_sub(1);
            return;
        }
        KeyCode::Down | KeyCode::Tab => {
            if visible_len > 0 {
                app.field_idx = (app.field_idx + 1).min(visible_len - 1);
            }
            return;
        }
        _ => {}
    }
    let kind = app.selected_kind();
    let fidx = app.field_idx;
    if let Some(form) = app.active_form_mut() {
        if let Some(changed_id) = edit_field(form, fidx, key) {
            form_module::on_field_changed(kind, form, changed_id);
        }
    }
}

fn confirm_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.should_quit = true;
            app.result = None;
        }
        _ => {
            // Anything else cancels the quit and returns to the Modules screen.
            app.screen = Screen::Modules;
        }
    }
}

// --- Shared field editing ---------------------------------------------------

/// Apply a key to the `fidx`-th *visible* field of `form`. Returns the field's
/// id when the change is one a dependent field cares about (an `Enum` flip), so
/// the caller can fire `on_field_changed`.
fn edit_field(form: &mut Form, fidx: usize, key: KeyEvent) -> Option<&'static str> {
    let vis = form.visible_indices();
    let &i = vis.get(fidx)?;
    let id = form.defs[i].id;
    if form.defs[i].kind.is_text() {
        if let Some(input) = form.values[i].input_mut() {
            input.handle_event(&Event::Key(key));
        }
        return None;
    }
    match &form.defs[i].kind {
        FieldKind::Bool { .. } => {
            if matches!(
                key.code,
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right | KeyCode::Enter
            ) {
                if let FieldValue::Bool(b) = &mut form.values[i] {
                    *b = !*b;
                }
                return Some(id);
            }
        }
        FieldKind::Enum { choices, .. } => {
            let len = choices.len();
            if let FieldValue::Enum(sel) = &mut form.values[i] {
                if cycle(sel, key, len) {
                    return Some(id);
                }
            }
        }
        FieldKind::ValueEnum => {
            if let FieldValue::ValueEnum { options, selected } = &mut form.values[i] {
                let len = options.len();
                if cycle(selected, key, len) {
                    return Some(id);
                }
            }
        }
        _ => {}
    }
    None
}

/// Move a selection index left/right (with wrap). Returns whether it moved.
fn cycle(sel: &mut usize, key: KeyEvent, len: usize) -> bool {
    if len == 0 {
        return false;
    }
    match key.code {
        KeyCode::Left => {
            *sel = (*sel + len - 1) % len;
            true
        }
        KeyCode::Right | KeyCode::Char(' ') | KeyCode::Enter => {
            *sel = (*sel + 1) % len;
            true
        }
        _ => false,
    }
}

/// Validate + assemble the config. On success, stash the result + format and
/// quit; on failure, surface the first problem in the status line.
fn save(app: &mut App) {
    match preview::build_value(app, true) {
        Ok(v) => {
            app.result = Some((v, app.format));
            app.should_quit = true;
        }
        Err(errs) => {
            let first = errs
                .first()
                .map(|(_, m)| m.clone())
                .unwrap_or_else(|| "invalid config".to_string());
            app.status = format!("Can't save — {first} ({} issue(s))", errs.len());
        }
    }
}
