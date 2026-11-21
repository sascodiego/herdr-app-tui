//! The screen: one header row, the panel, the terminal area. No status bar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::app::App;
use super::glyph::{self, State};
use super::theme;
use crate::vt::CursorShape;

/// One row of text in an attached screen, with its styles resolved to tokens
/// the view can paint. Kept separate from `vt::Cell`, which is not `Debug` and
/// carries emulator state the view must not know about.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenCell {
    pub text: String,
    pub fg: ratatui::style::Color,
    pub bg: Option<ratatui::style::Color>,
    pub bold: bool,
    pub italic: bool,
    pub faint: bool,
    pub strike: bool,
    pub underline: bool,
}

#[derive(Clone, Debug)]
pub struct ScreenState {
    pub rows: Vec<Vec<ScreenCell>>,
    /// `(x, y, shape)` of the cursor.
    pub cursor: Option<(u16, u16, u8)>,
}

impl ScreenState {
    /// Take a snapshot of a libghostty frame.
    pub fn from_frame(frame: crate::vt::Frame) -> ScreenState {
        let color = |rgb: crate::vt::Rgb| ratatui::style::Color::Rgb(rgb.0, rgb.1, rgb.2);
        let rows = frame
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| ScreenCell {
                        text: cell.text.clone(),
                        fg: color(cell.fg),
                        bg: cell.bg.map(color),
                        bold: cell.bold,
                        italic: cell.italic,
                        faint: cell.faint,
                        strike: cell.strike,
                        underline: cell.underline.is_some(),
                    })
                    .collect()
            })
            .collect();
        let cursor = frame.cursor.map(|cursor| {
            let shape = match cursor.shape {
                CursorShape::Block => 0,
                CursorShape::Bar => 1,
                CursorShape::Underline => 2,
                CursorShape::Hollow => 3,
            };
            (cursor.x, cursor.y, shape)
        });
        ScreenState { rows, cursor }
    }
}

/// Below this many columns the panel opens over the whole screen; at or above
/// it, the panel is a rail that is always visible on the left.
pub const WIDE_AT: u16 = 60;
/// The rail's width when the screen is wide.
pub const RAIL_WIDTH: u16 = 26;

/// Where a pointer can land. The same zones drive the render and the input
/// routing, so a tap always hits what the screen shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zones {
    pub wide: bool,
    /// The single row at the top of a phone-sized screen.
    pub header: Option<Rect>,
    /// The sheet on a phone, the rail on a desktop.
    pub panel: Option<Rect>,
    /// The one-column rule between the rail and the terminal.
    pub rule: Option<Rect>,
    /// The live terminal or its preview.
    pub terminal: Rect,
}

impl Zones {
    pub fn contains(rect: Option<Rect>, x: u16, y: u16) -> bool {
        rect.is_some_and(|r| x >= r.x && x < r.x.saturating_add(r.width) && y >= r.y && y < r.y.saturating_add(r.height))
    }
}

/// Resolve the screen's zones once, for both drawing and hit testing.
pub fn zones(area: Rect, panel_open: bool) -> Zones {
    if area.width >= WIDE_AT {
        let [rail, rest] = Layout::horizontal([Constraint::Length(RAIL_WIDTH), Constraint::Min(1)]).areas(area);
        let [rule, terminal] = Layout::horizontal([Constraint::Length(1), Constraint::Min(1)]).areas(rest);
        Zones { wide: true, header: None, panel: Some(rail), rule: Some(rule), terminal }
    } else if panel_open {
        let [header, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
        Zones { wide: false, header: Some(header), panel: Some(body), rule: None, terminal: Rect::new(0, 0, 0, 0) }
    } else {
        let [header, terminal] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
        Zones { wide: false, header: Some(header), panel: None, rule: None, terminal }
    }
}

/// Draw one frame.
pub fn render(app: &App, frame: &mut Frame) {
    let z = zones(frame.area(), app.panel);
    if let Some(header) = z.header {
        draw_header(app, header, frame);
    }
    if let Some(panel) = z.panel {
        draw_panel(app, panel, frame, z.wide);
    }
    if let Some(rule) = z.rule {
        draw_rule(rule, frame);
    }
    if z.terminal.width > 0 && z.terminal.height > 0 {
        draw_terminal(app, frame, z.terminal);
    }
}

/// Which tab row, if any, is drawn at this screen coordinate.
pub fn panel_row_at(app: &App, area: Rect, y: u16, wide: bool) -> Option<usize> {
    let local = y.checked_sub(area.y)?;
    let (_, rows) = panel_content(app, area.width, wide);
    rows.get(local as usize).copied().flatten()
}

/// One row: where you are on the left, the core and the toggle on the right.
fn draw_header(app: &App, area: Rect, frame: &mut Frame) {
    let (mark, color) = connection(app);
    let toggle = if app.panel { format!("close {}", glyph::CLOSE) } else { "switch".to_string() };
    let copy = if app.copy { " copy" } else { "" };
    let right = format!("{} {mark}  {toggle}{copy}", app.core);
    let gap = (area.width as usize).saturating_sub(app.session.chars().count() + right.chars().count());
    let line = Line::from(vec![
        Span::styled(app.session.clone(), Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(gap)),
        Span::styled(format!("{} ", app.core), Style::default().fg(theme::MUTED)),
        Span::styled(mark.to_string(), Style::default().fg(color)),
        Span::styled(format!("  {toggle}{copy}"), Style::default().fg(theme::DIM)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

/// The core's connection marker and the color it takes.
fn connection(app: &App) -> (char, ratatui::style::Color) {
    if app.connected {
        (glyph::CONNECTED, theme::ACCENT)
    } else {
        (glyph::DISCONNECTED, theme::RED)
    }
}

fn draw_rule(area: Rect, frame: &mut Frame) {
    let rule = Block::new().borders(Borders::LEFT).border_style(Style::default().fg(theme::DIM));
    frame.render_widget(rule, area);
}

/// The panel: counted sections, the selected row banded, actions at the foot.
fn draw_panel(app: &App, area: Rect, frame: &mut Frame, wide: bool) {
    let (lines, _) = panel_content(app, area.width, wide);
    let style = Style::default().bg(theme::PANEL).fg(theme::TEXT);
    frame.render_widget(Paragraph::new(lines).style(style), area);
}

/// Panel lines plus, for each line, the tab row it shows.
#[derive(Default)]
struct PanelLines {
    lines: Vec<Line<'static>>,
    rows: Vec<Option<usize>>,
}

impl PanelLines {
    fn push(&mut self, line: Line<'static>) {
        self.lines.push(line);
        self.rows.push(None);
    }

    fn push_row(&mut self, line: Line<'static>, index: usize) {
        self.lines.push(line);
        self.rows.push(Some(index));
    }
}

/// The terminal area: the attached screen when there is one, the preview otherwise.
fn draw_terminal(app: &App, frame: &mut Frame, area: Rect) {
    // Copy mode uses the very same ScreenState, taken from the scrolled
    // libghostty viewport, so every cell keeps its program-defined style.
    if let Some(screen) = &app.screen {
        draw_screen(screen, area, frame);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    match app.selected() {
        Some(terminal) if !terminal.preview.is_empty() => {
            for line in &terminal.preview {
                lines.push(Line::from(Span::styled(line.clone(), Style::default().fg(theme::TEXT))));
            }
        }
        Some(terminal) => {
            lines.push(Line::from(Span::styled(
                format!("{} has printed nothing yet", terminal.name),
                Style::default().fg(theme::MUTED),
            )));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "no terminals yet: hc run <program>",
                Style::default().fg(theme::MUTED),
            )));
        }
    }
    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(theme::BG)), area);
}

/// Paint the attached screen into ratatui's buffer.
fn draw_screen(screen: &ScreenState, area: Rect, frame: &mut Frame) {
    let buf = frame.buffer_mut();
    for (y, row) in screen.rows.iter().enumerate().take(area.height as usize) {
        for (x, cell) in row.iter().enumerate().take(area.width as usize) {
            let mut style = Style::default().fg(cell.fg).bg(cell.bg.unwrap_or(theme::BG));
            if cell.bold { style = style.add_modifier(Modifier::BOLD); }
            if cell.italic { style = style.add_modifier(Modifier::ITALIC); }
            if cell.faint { style = style.add_modifier(Modifier::DIM); }
            if cell.strike { style = style.add_modifier(Modifier::CROSSED_OUT); }
            if cell.underline { style = style.add_modifier(Modifier::UNDERLINED); }
            let symbol = if cell.text.is_empty() { " " } else { &cell.text };
            buf[(area.x + x as u16, area.y + y as u16)].set_symbol(symbol).set_style(style);
        }
    }
    if let Some((x, y, shape)) = screen.cursor
        && y < area.height
        && x < area.width
    {
        let cell = &mut buf[(area.x + x, area.y + y)];
        match shape {
            0 => { cell.set_bg(theme::ACCENT); cell.set_fg(theme::BG); }
            _ => { cell.set_fg(theme::ACCENT); }
        }
    }
}

/// The panel's rows, paired with the tab index each line belongs to.
fn panel_content(app: &App, width: u16, wide: bool) -> (Vec<Line<'static>>, Vec<Option<usize>>) {
    let mut lines = PanelLines::default();

    if wide {
        lines.push(header_line(app, width));
    }

    let rows = app.rows();
    lines.push(section(&format!("{} TABS", glyph::EXPANDED), rows.len(), width));
    for (index, row) in rows.iter().enumerate() {
        lines.push_row(row_line(row, index == app.cursor, width), index);
        if let Some(address) = &row.address {
            for child in app.children_of(address) {
                lines.push(child_line(child));
            }
        }
    }

    let (working, needs_you, idle) = app.counts();
    let agents = working + needs_you + idle;
    if agents > 0 {
        lines.push(section(&format!("{} AGENTS", glyph::EXPANDED), agents, width));
        for (state, count) in [(State::Working, working), (State::NeedsYou, needs_you), (State::Idle, idle)] {
            if count > 0 {
                lines.push(counted_line(state, count, width));
            }
        }
    }

    if let Some(terminal) = app.selected()
        && !terminal.preview.is_empty()
    {
        lines.push(task_line(&terminal.name, width));
    }

    if app.hidden_finished > 0 {
        lines.push(Line::from(Span::styled(
            format!("{} finished hidden", app.hidden_finished),
            Style::default().fg(theme::DIM),
        )));
    }

    // The actions sit at the foot of the panel, not in a status bar.
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "settings  keys  reload  detach".to_string(),
        Style::default().fg(theme::DIM),
    )));
    (lines.lines, lines.rows)
}

fn header_line(app: &App, width: u16) -> Line<'static> {
    let (mark, color) = connection(app);
    let copy = if app.copy { " copy" } else { "" };
    let right = format!("{} {mark}{copy}", app.core);
    let gap = (width as usize).saturating_sub(app.session.chars().count() + right.chars().count());
    Line::from(vec![
        Span::styled(app.session.clone(), Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(gap)),
        Span::styled(format!("{} ", app.core), Style::default().fg(theme::MUTED)),
        Span::styled(mark.to_string(), Style::default().fg(color)),
        Span::styled(copy.to_string(), Style::default().fg(theme::DIM)),
    ])
}

fn section(label: &str, count: usize, width: u16) -> Line<'static> {
    let text = two_columns(label, &count.to_string(), width);
    Line::from(Span::styled(text, Style::default().fg(theme::MUTED).add_modifier(Modifier::BOLD)))
}

fn row_line(row: &super::app::Row, selected: bool, width: u16) -> Line<'static> {
    let left = format!("  {}", row.name);
    let state = format!("{} {}", row.state.glyph(), row.state.word());
    let gap = (width as usize).saturating_sub(left.chars().count() + state.chars().count());
    let band = selected.then(|| Style::default().bg(theme::SURFACE));
    let label = band.unwrap_or_default().fg(theme::TEXT).add_modifier(Modifier::BOLD);
    let padding = band.unwrap_or_default();
    let value = band.unwrap_or_else(|| Style::default().fg(row.state.color()));
    Line::from(vec![
        Span::styled(left, label),
        Span::styled(" ".repeat(gap), padding),
        Span::styled(state, value),
    ])
}

fn child_line(terminal: &super::app::Terminal) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("      {} {}", terminal.state.glyph(), terminal.name),
            Style::default().fg(theme::MUTED),
        ),
        Span::styled(
            format!(" {} {}", glyph::SEPARATOR, terminal.state.word()),
            Style::default().fg(theme::DIM),
        ),
    ])
}

fn counted_line(state: State, count: usize, width: u16) -> Line<'static> {
    let left = format!("  {} {}", state.glyph(), state.word());
    let gap = (width as usize).saturating_sub(left.chars().count() + count.to_string().chars().count());
    Line::from(vec![
        Span::styled(left, Style::default().fg(state.color())),
        Span::raw(" ".repeat(gap)),
        Span::styled(count.to_string(), Style::default().fg(theme::MUTED)),
    ])
}

fn task_line(name: &str, width: u16) -> Line<'static> {
    let label = format!("─ {name} ");
    let line = format!("{label}{}", "─".repeat((width as usize).saturating_sub(label.chars().count())));
    Line::from(Span::styled(line, Style::default().fg(theme::DIM)))
}

/// `left`, then spaces, then `right`, filling `width`.
fn two_columns(left: &str, right: &str, width: u16) -> String {
    let used = left.chars().count() + right.chars().count();
    let gap = (width as usize).saturating_sub(used);
    format!("{left}{}{right}", " ".repeat(gap))
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::App;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use serde_json::json;

    fn app(panel: bool) -> App {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "hidden_finished": 0,
            "machines": [],
            "terminals": [
                { "address": "t1", "name": "pi 1", "status": { "state": "idle" } },
                { "address": "t2", "name": "pi 2", "status": { "state": "blocked" } },
                { "address": "t3", "name": "pi 3", "status": { "state": "working" } }
            ]
        }));
        if panel {
            app.toggle_panel();
        }
        app
    }

    /// Every row of the screen as text.
    fn screen(app: &App, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
        terminal.draw(|frame| render(app, frame)).expect("draw");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>()
            })
            .collect()
    }

    #[test]
    fn the_header_zone_only_exists_on_a_phone() {
        let phone = zones(Rect::new(0, 0, 40, 22), false);
        assert!(Zones::contains(phone.header, 10, 0));
        assert!(!Zones::contains(phone.header, 10, 1));
        assert!(Zones::contains(Some(phone.terminal), 10, 1));
        assert!(!Zones::contains(phone.panel, 10, 5));

        let wide = zones(Rect::new(0, 0, 100, 24), false);
        assert!(wide.wide);
        assert!(wide.header.is_none());
        assert!(Zones::contains(wide.panel, 2, 4));
        assert_eq!(wide.terminal.x, RAIL_WIDTH + 1);
    }

    #[test]
    fn a_tap_on_a_tab_row_resolves_to_its_index() {
        let app = app(true);
        let panel = Rect::new(0, 1, 40, 21);
        let (_, rows) = panel_content(&app, panel.width, false);
        let line = rows.iter().position(|row| *row == Some(0)).expect("first tab row");
        assert_eq!(panel_row_at(&app, panel, 1 + line as u16, false), Some(0));
        assert_eq!(panel_row_at(&app, panel, 0, false), None, "the header is not a row");
    }

    #[test]
    fn copy_mode_paints_the_styled_scrolled_frame() {
        let mut app = app(false);
        app.enter_copy();
        app.screen = Some(ScreenState {
            rows: vec![vec![ScreenCell {
                text: "X".to_string(),
                fg: ratatui::style::Color::Red,
                bg: Some(ratatui::style::Color::Blue),
                bold: true,
                italic: false,
                faint: false,
                strike: false,
                underline: false,
            }]],
            cursor: None,
        });
        let mut terminal = Terminal::new(TestBackend::new(40, 22)).expect("test terminal");
        terminal.draw(|frame| render(&app, frame)).expect("draw");
        let cell = terminal.backend().buffer()[(0, 1)].clone();
        assert_eq!(cell.symbol(), "X");
        assert_eq!(cell.fg, ratatui::style::Color::Red);
        assert_eq!(cell.bg, ratatui::style::Color::Blue);
        assert!(cell.modifier.contains(Modifier::BOLD));
        let rows = screen(&app, 40, 22);
        assert!(rows[0].contains("copy"), "the header states the mode");
    }

    #[test]
    fn a_wide_screen_keeps_the_rail_on_the_left() {
        let rows = screen(&app(true), 100, 24);
        assert!(rows.first().is_some_and(|row| row.starts_with("default")), "{:?}", &rows[..2]);
        assert!(rows.iter().any(|row| row.starts_with("▾ TABS")), "rail sections start at column 0: {rows:?}");
        // The rule sits where the rail ends; the terminal area is to its right.
        assert!(rows.iter().any(|row| row.chars().nth(RAIL_WIDTH as usize) == Some('│')), "no rule after the rail");
    }

    #[test]
    fn a_narrow_screen_with_the_bar_closed_is_the_terminal_and_one_row() {
        let rows = screen(&app(false), 40, 22);
        assert!(rows[0].contains("default"), "header row: {:?}", rows[0]);
        assert!(rows[0].contains("switch"), "the toggle lives in the header: {:?}", rows[0]);
        assert!(!rows.iter().any(|row| row.contains("▾ TABS")), "the panel is closed: {rows:?}");
        assert!(!rows.iter().any(|row| row.contains("detach")), "no actions while closed: {rows:?}");
    }

    #[test]
    fn opening_the_panel_on_a_narrow_screen_covers_it_with_sections() {
        let rows = screen(&app(true), 40, 22);
        assert!(rows[0].contains("close"), "the toggle turns into close: {:?}", rows[0]);
        assert!(rows.iter().any(|row| row.contains("▾ TABS")), "{rows:?}");
        assert!(rows.iter().any(|row| row.contains("▾ AGENTS")), "{rows:?}");
        assert!(rows.iter().any(|row| row.contains("needs you")), "{rows:?}");
        assert!(rows.iter().any(|row| row.contains("detach")), "actions at the foot: {rows:?}");
    }

    #[test]
    fn the_session_name_is_written_once_per_screen() {
        for panel in [false, true] {
            let rows = screen(&app(panel), 40, 22);
            let joined = rows.join("\n");
            assert_eq!(joined.matches("default").count(), 1, "panel={panel}:\n{joined}");
        }
    }

    #[test]
    fn there_is_no_status_bar_at_the_bottom() {
        // The wide screen's last row belongs to the terminal area, not to chrome.
        let rows = screen(&app(true), 100, 24);
        let last = rows.last().expect("a last row");
        let rail_side: String = last.chars().take(RAIL_WIDTH as usize).collect();
        let terminal_side: String = last.chars().skip(RAIL_WIDTH as usize).collect();
        assert!(!terminal_side.contains("detach"), "the terminal area is not chrome: {terminal_side:?}");
        assert!(!rail_side.contains("detach") || terminal_side.trim().is_empty(), "{rail_side:?}");
    }

    #[test]
    fn whoever_needs_you_is_the_first_row() {
        let rows = screen(&app(true), 40, 22);
        let first_row = rows
            .iter()
            .find(|row| row.contains("pi "))
            .expect("a terminal row");
        assert!(first_row.contains("pi 2"), "blocked comes first: {first_row:?}");
        assert!(first_row.contains("needs you"), "{first_row:?}");
    }

    #[test]
    fn the_toggle_words_are_switch_and_close() {
        assert!(screen(&app(false), 40, 22)[0].contains("switch"));
        assert!(screen(&app(true), 40, 22)[0].contains("close"));
    }

    #[test]
    fn two_columns_fills_the_width_and_leaves_a_gap() {
        assert_eq!(two_columns("ab", "cd", 10), "ab      cd");
        assert_eq!(two_columns("abcdef", "gh", 4), "abcdefgh", "never truncates, never panics");
    }
}
