//! ratatui rendering — the thin, untested shell. Reads [`App`] state and draws
//! the two wizard screens. All logic lives in the pure modules; this file only
//! turns state into widgets.

use super::app::{App, Focus, Screen, Target};
use super::field::{FieldKind, FieldValue, Form};
use super::{form_module, preview};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const LABEL_W: usize = 18;
const PREFIX_W: u16 = LABEL_W as u16 + 1;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(f.area());
    match app.screen {
        Screen::Setup => render_setup(f, app, chunks[0]),
        Screen::Modules => render_modules(f, app, chunks[0]),
        Screen::ConfirmQuit => {
            render_modules(f, app, chunks[0]);
            render_confirm(f, chunks[0]);
        }
    }
    render_footer(f, app, chunks[1]);
}

// --- Setup screen -----------------------------------------------------------

fn render_setup(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ohmyoled config builder — Setup ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut y = inner.y;

    // Row 0: target radio.
    let target_line = Line::from(vec![
        Span::raw("Display target:   "),
        radio(app.target == Target::Matrix, "Matrix (LED)"),
        Span::raw("   "),
        radio(app.target == Target::Eink, "E-ink"),
    ]);
    draw_row(f, inner.x, y, inner.width, app.setup_idx == 0, target_line);
    y += 2;

    // Section header for the target's options.
    let header = match app.target {
        Target::Matrix => "Matrix panel options",
        Target::Eink => "E-ink panel options",
    };
    f.render_widget(
        Paragraph::new(Line::from(header.dim())),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;

    // Target form fields (rows 1..=N).
    let vis = app.target_form.visible_indices();
    for (j, &i) in vis.iter().enumerate() {
        if y >= inner.bottom() {
            break;
        }
        let selected = app.setup_idx == j + 1;
        draw_field_row(
            f,
            Rect::new(inner.x, y, inner.width, 1),
            &app.target_form,
            i,
            selected,
            selected,
        );
        y += 1;
    }
    y += 1;

    // Last row: format radio.
    let last = super_setup_last(app);
    let fmt_line = Line::from(vec![
        Span::raw("Config format:    "),
        radio(app.format.ext() == "json", "json"),
        Span::raw("  "),
        radio(app.format.ext() == "yaml", "yaml"),
        Span::raw("  "),
        radio(app.format.ext() == "toml", "toml"),
    ]);
    if y < inner.bottom() {
        draw_row(f, inner.x, y, inner.width, app.setup_idx == last, fmt_line);
    }
}

fn super_setup_last(app: &App) -> usize {
    1 + app.target_form.visible_indices().len()
}

fn radio(on: bool, label: &str) -> Span<'static> {
    let dot = if on { "(•) " } else { "( ) " };
    let s = format!("{dot}{label}");
    if on {
        Span::styled(s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Span::raw(s)
    }
}

// --- Modules screen ---------------------------------------------------------

fn render_modules(f: &mut Frame, app: &App, area: Rect) {
    // Wide: list | form | preview side by side. Narrow: preview drops below.
    if area.width >= 96 {
        let cols = Layout::horizontal([
            Constraint::Length(28),
            Constraint::Min(30),
            Constraint::Length(42),
        ])
        .split(area);
        render_list(f, app, cols[0]);
        render_form_pane(f, app, cols[1]);
        render_preview(f, app, cols[2]);
    } else {
        let rows = Layout::vertical([Constraint::Min(8), Constraint::Length(10)]).split(area);
        let top = Layout::horizontal([Constraint::Length(28), Constraint::Min(20)]).split(rows[0]);
        render_list(f, app, top[0]);
        render_form_pane(f, app, top[1]);
        render_preview(f, app, rows[1]);
    }
}

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focus, Focus::List);
    let block = pane_block(" Modules ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    for (idx, (kind, label)) in form_module::TILE_KINDS.iter().enumerate() {
        if idx as u16 >= inner.height {
            break;
        }
        let y = inner.y + idx as u16;
        let count = app.instances_of(kind).len();
        let mark = if count > 0 { "[x]" } else { "[ ]" };
        let suffix = if count > 1 {
            format!(" ×{count}")
        } else {
            String::new()
        };
        let text = format!("{mark} {label}{suffix}");
        let selected = idx == app.kind_idx;
        let mut style = Style::default();
        if count > 0 {
            style = style.fg(Color::Green);
        }
        if selected {
            style = style.add_modifier(Modifier::REVERSED);
        }
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(text, style))),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }
}

fn render_form_pane(f: &mut Frame, app: &App, area: Rect) {
    let kind = app.selected_kind();
    let focused = matches!(app.focus, Focus::Form);
    let inst_idxs = app.instances_of(kind);
    let title = if inst_idxs.len() > 1 {
        format!(
            " {} ({}/{}) ",
            form_module::title(kind),
            app.inst_idx + 1,
            inst_idxs.len()
        )
    } else {
        format!(" {} ", form_module::title(kind))
    };
    let block = pane_block(&title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(gi) = app.active_instance() else {
        f.render_widget(
            Paragraph::new(vec![
                Line::from("This tile is off.".dim()),
                Line::from(""),
                Line::from("Press space in the list to enable it.".dim()),
            ])
            .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    };
    let form = &app.instances[gi].form;

    let parts = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(inner);
    let fields_area = parts[0];
    let help_area = parts[1];

    let vis = form.visible_indices();
    for (row, &i) in vis.iter().enumerate() {
        if row as u16 >= fields_area.height {
            break;
        }
        let y = fields_area.y + row as u16;
        let selected = row == app.field_idx;
        draw_field_row(
            f,
            Rect::new(fields_area.x, y, fields_area.width, 1),
            form,
            i,
            selected,
            focused && selected,
        );
    }

    // Help line for the focused field.
    if let Some(&i) = vis.get(app.field_idx) {
        f.render_widget(
            Paragraph::new(Line::from(form.defs[i].help.dim())).wrap(Wrap { trim: true }),
            help_area,
        );
    }
}

fn render_preview(f: &mut Frame, app: &App, area: Rect) {
    let value = preview::preview_value(app);
    let text = preview::render_string(&value, app.preview_fmt);
    let title = format!(" Preview [{}] ", app.preview_fmt.label());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title);
    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn render_confirm(f: &mut Frame, area: Rect) {
    let w = 44u16.min(area.width);
    let h = 5u16.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Quit without saving? ");
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from("  y / Enter = quit    any other key = cancel"),
    ])
    .block(block);
    f.render_widget(para, rect);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let hint = match app.screen {
        Screen::Setup => "↑↓ move  ←→/space change  Enter next  Esc quit",
        Screen::Modules => match app.focus {
            Focus::List => {
                "↑↓ tile  space on/off  a add  d del  ←→ inst  Enter edit  ^S save  F2 fmt  Esc back"
            }
            Focus::Form => "↑↓ field  ←→/space change  type to edit  Esc list  ^S save  F2 fmt",
        },
        Screen::ConfirmQuit => "y = quit   any other key = cancel",
    };
    let left = Span::raw(format!(" {hint}"));
    let status = if app.status.is_empty() {
        String::new()
    } else {
        format!("{}  ", app.status)
    };
    let line = Line::from(vec![
        left,
        Span::raw("  "),
        Span::styled(status, Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Rgb(30, 30, 30))),
        area,
    );
}

// --- shared field rendering -------------------------------------------------

fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let color = if focused { Color::Cyan } else { Color::DarkGray };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(title.to_string())
}

fn draw_row(f: &mut Frame, x: u16, y: u16, width: u16, selected: bool, line: Line) {
    let style = if selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    f.render_widget(
        Paragraph::new(line).style(style),
        Rect::new(x, y, width, 1),
    );
}

/// Render one `label  value` field row in a 1-high `area`, optionally placing
/// the text cursor.
fn draw_field_row(f: &mut Frame, area: Rect, form: &Form, i: usize, selected: bool, cursor: bool) {
    let label = form.defs[i].label;
    let value = field_value_display(form, i);
    let text = format!("{label:<LABEL_W$} {value}");
    let style = if selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    f.render_widget(Paragraph::new(Line::from(Span::styled(text, style))), area);
    if cursor && form.defs[i].kind.is_text() {
        if let FieldValue::Input(input) = &form.values[i] {
            let max_x = area.x + area.width.saturating_sub(1);
            let cx = (area.x + PREFIX_W + input.visual_cursor() as u16).min(max_x);
            f.set_cursor_position((cx, area.y));
        }
    }
}

fn field_value_display(form: &Form, i: usize) -> String {
    match (&form.defs[i].kind, &form.values[i]) {
        (FieldKind::Bool { .. }, FieldValue::Bool(b)) => {
            if *b { "[x] yes".to_string() } else { "[ ] no".to_string() }
        }
        (FieldKind::Enum { choices, .. }, FieldValue::Enum(sel)) => {
            let slug = choices.get(*sel).map(|(s, _)| *s).unwrap_or("?");
            format!("< {slug} >")
        }
        (FieldKind::ValueEnum, FieldValue::ValueEnum { options, selected }) => options
            .get(*selected)
            .map(|(n, _)| format!("< {n} >"))
            .unwrap_or_else(|| "< (none) >".to_string()),
        (_, FieldValue::Input(input)) => {
            let v = input.value();
            if v.is_empty() {
                "·".to_string()
            } else {
                v.to_string()
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::createjson::tui::app::{App, Focus, Screen};
    use ratatui::{backend::TestBackend, Terminal};

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    /// The Setup screen renders without panicking and shows both targets.
    #[test]
    fn setup_screen_renders() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let app = App::new(None, None);
        terminal.draw(|f| render(f, &app)).unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Matrix"));
        assert!(text.contains("E-ink"));
        assert!(text.contains("json"));
    }

    /// The Modules screen renders with an enabled tile + live preview.
    #[test]
    fn modules_screen_renders_with_preview() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(None, None);
        app.screen = Screen::Modules;
        app.focus = Focus::Form;
        app.add_instance("weather");
        terminal.draw(|f| render(f, &app)).unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Modules"));
        assert!(text.contains("Preview"));
    }
}

