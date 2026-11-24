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
/// Rows in the mobile information band.
pub const BAND_ROWS: u16 = 3;
/// Width of the fixed toggle block at the right of the band.
pub const TOGGLE_COLS: u16 = 8;
/// Minimum gutter around sidebar content, in cells.
pub const GUTTER: u16 = 1;

/// Where a pointer can land. The same zones drive the render and the input
/// routing, so a tap always hits what the screen shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zones {
    pub wide: bool,
    /// The three-row information band at the top of a phone-sized screen.
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
    } else {
        let band = BAND_ROWS.min(area.height);
        let [header, body] = Layout::vertical([Constraint::Length(band), Constraint::Min(0)]).areas(area);
        if panel_open {
            Zones { wide: false, header: Some(header), panel: Some(body), rule: None, terminal: Rect::new(0, 0, 0, 0) }
        } else {
            Zones { wide: false, header: Some(header), panel: None, rule: None, terminal: body }
        }
    }
}

/// The sidebar's content area: one cell in from every side.
fn panel_inner(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(GUTTER),
        area.y.saturating_add(GUTTER),
        area.width.saturating_sub(2 * GUTTER),
        area.height.saturating_sub(2 * GUTTER),
    )
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
    if app.prefix {
        draw_prefix(frame);
    }
}

/// Tab indices reachable from the detected agent list, in attention order.
pub fn agent_targets(app: &App) -> Vec<usize> {
    app.agents()
        .iter()
        .filter_map(|agent| {
            app.rows.iter().position(|row| {
                row.address.as_deref() == Some(agent.address.as_str())
                    || row.address.as_deref() == agent.parent.as_deref()
            })
        })
        .collect()
}

/// A temporary overlay while a workspace prefix command is pending.
fn draw_prefix(frame: &mut Frame) {
    let screen = frame.area();
    let width = 34.min(screen.width.saturating_sub(2));
    let height = 8.min(screen.height.saturating_sub(1));
    if width == 0 || height == 0 {
        return;
    }
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(
        Block::new().borders(Borders::ALL).border_style(Style::default().fg(theme::ACCENT)).style(Style::default().bg(theme::PANEL)),
        area,
    );
    let lines = vec![
        Line::from(Span::styled("WORKSPACE PREFIX", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))),
        Line::from("c  new tab      w  new space"),
        Line::from("r  rename tab   R  rename space"),
        Line::from("x  close tab    X  close space"),
        Line::from("o  toggle sidebar"),
        Line::from(Span::styled("esc cancels", Style::default().fg(theme::DIM))),
    ];
    let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    frame.render_widget(Paragraph::new(lines).style(Style::default().fg(theme::TEXT)), inner);
}

/// Which tab row, if any, is drawn at this screen coordinate.
pub fn panel_row_at(app: &App, area: Rect, x: u16, y: u16, wide: bool) -> Option<usize> {
    let inner = panel_inner(area);
    if x < inner.x
        || x >= inner.x.saturating_add(inner.width)
        || y < inner.y
        || y >= inner.y.saturating_add(inner.height)
    {
        return None;
    }
    let local = y - inner.y;
    let view = panel_view(app, inner.width, wide, inner.height);
    let rows = &view.rows;
    let offset = view.offset;
    rows.get((local as usize).saturating_add(offset)).copied().flatten()
}

/// Which space row, if any, is drawn at this screen coordinate.
pub fn panel_space_at(app: &App, area: Rect, x: u16, y: u16, wide: bool) -> Option<String> {
    let inner = panel_inner(area);
    if x < inner.x
        || x >= inner.x.saturating_add(inner.width)
        || y < inner.y
        || y >= inner.y.saturating_add(inner.height)
    {
        return None;
    }
    let local = y - inner.y;
    let view = panel_view(app, inner.width, wide, inner.height);
    let spaces = &view.spaces;
    let offset = view.offset;
    spaces.get((local as usize).saturating_add(offset)).cloned().flatten()
}

/// The three-row phone band: workspace and tabs, path, agent totals, toggle.
fn draw_header(app: &App, area: Rect, frame: &mut Frame) {
    frame.buffer_mut().set_style(area, Style::default().bg(theme::PANEL));
    let left_width = area.width.saturating_sub(TOGGLE_COLS);
    let left = Rect { x: area.x, y: area.y, width: left_width, height: area.height };
    let toggle = Rect {
        x: area.x.saturating_add(left_width),
        y: area.y,
        width: area.width.saturating_sub(left_width),
        height: area.height,
    };
    let (working, needs_you, idle) = app.counts();
    let (mark, color) = connection(app);
    let lines = vec![
        Line::from(vec![
            Span::styled(mark.to_string(), Style::default().fg(color)),
            Span::styled(
                format!(
                    " {} · {} {} · {}",
                    app.workspace(),
                    app.rows().len(),
                    tab_word(app.rows().len()),
                    app.pane_state()
                ),
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(fit(&app.path, left_width as usize), Style::default().fg(theme::MUTED))),
        Line::from(Span::styled(
            fit(
                &format!(
                    "{} idle {} · {} work {} · {} needs {}",
                    State::Idle.glyph(),
                    idle,
                    State::Working.glyph(),
                    working,
                    State::NeedsYou.glyph(),
                    needs_you,
                ),
                left_width as usize,
            ),
            Style::default().fg(theme::TEXT),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), left);

    let label = if app.panel { "x" } else { "switch" };
    let pad = (toggle.width as usize).saturating_sub(label.chars().count()) / 2;
    let mut toggle_lines = vec![Line::from(""); area.height as usize];
    if let Some(middle) = toggle_lines.get_mut(area.height as usize / 2) {
        *middle = Line::from(Span::styled(
            format!("{}{}", " ".repeat(pad), label),
            Style::default().fg(theme::TEXT),
        ));
    }
    frame.render_widget(Paragraph::new(toggle_lines).style(Style::default().bg(theme::SURFACE)), toggle);
}

/// The band reads `1 tab`, not `1 tabs`.
fn tab_word(count: usize) -> &'static str {
    if count == 1 { "tab" } else { "tabs" }
}

/// Truncate chrome text with `…` instead of wrapping it.
fn fit(text: &str, width: usize) -> String {
    let length = text.chars().count();
    if length <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let head: String = text.chars().take(width - 1).collect();
    format!("{head}…")
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

/// The background band painted under one panel line. Chrome sits on BG;
/// an item block paints PANEL, or SURFACE when it is the current item.
/// Every nesting level steps two cells further in: that empty step is the
/// hierarchy, never a drawn connector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Band {
    None,
    Item { selected: bool, child: bool, step: u16 },
}

impl Band {
    fn color(self) -> Option<ratatui::style::Color> {
        match self {
            Band::None => None,
            Band::Item { selected: true, .. } => Some(theme::SURFACE),
            Band::Item { selected: false, .. } => Some(theme::PANEL),
        }
    }

    fn step(self) -> u16 {
        match self {
            Band::Item { step, .. } => step,
            Band::None => 0,
        }
    }
}

/// One renderable unit. Item blocks own two lines and are the only clip and
/// scroll unit; chrome owns one line and is never clipped on its own.
#[derive(Clone)]
struct PanelBlock {
    lines: Vec<Line<'static>>,
    rows: Vec<Option<usize>>,
    spaces: Vec<Option<String>>,
    bands: Vec<Band>,
    item: bool,
}

impl PanelBlock {
    fn chrome(line: Line<'static>) -> PanelBlock {
        PanelBlock {
            lines: vec![line],
            rows: vec![None],
            spaces: vec![None],
            bands: vec![Band::None],
            item: false,
        }
    }

    fn item(
        top: Line<'static>,
        detail: Line<'static>,
        row: Option<usize>,
        space: Option<String>,
        selected: bool,
        step: u16,
    ) -> PanelBlock {
        let band = Band::Item { selected, child: step > 0, step };
        PanelBlock {
            lines: vec![top, detail],
            rows: vec![row, row],
            spaces: vec![space.clone(), space],
            bands: vec![band, band],
            item: true,
        }
    }
}

/// The panel: counted sections, item blocks and actions at the foot.
fn draw_panel(app: &App, area: Rect, frame: &mut Frame, wide: bool) {
    // BG is the canvas: only item bands cover it, so a gap between blocks is
    // visible without drawing a box or a connector.
    frame.buffer_mut().set_style(area, Style::default().bg(theme::BG));
    let inner = panel_inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let view = panel_view(app, inner.width, wide, inner.height);
    let take = view.len.min(inner.height as usize);
    let visible = view.lines.iter().skip(view.offset).take(take).cloned().collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(visible).style(Style::default().fg(theme::TEXT)), inner);

    // Paint after the text: Buffer::set_style patches only the fields a Style
    // sets, so a background here never erases the glyphs or their foreground.
    let bands = view.bands.into_iter().skip(view.offset).take(take);
    let buffer = frame.buffer_mut();
    for (line, band) in bands.enumerate() {
        let Some(color) = band.color() else { continue };
        let y = inner.y + line as u16;
        if y >= inner.y.saturating_add(inner.height) {
            break;
        }
        let step = band.step().min(inner.width);
        buffer.set_style(
            Rect::new(inner.x + step, y, inner.width.saturating_sub(step), 1),
            Style::default().bg(color),
        );
    }
}

/// Panel lines, tab targets, space targets, their bands and the scroll offset.
struct PanelView {
    lines: Vec<Line<'static>>,
    rows: Vec<Option<usize>>,
    spaces: Vec<Option<String>>,
    bands: Vec<Band>,
    offset: usize,
    /// Exact number of lines the viewport paints, indicators included.
    len: usize,
}

/// Flatten blocks into parallel line arrays, remembering each block's range.
fn flatten(blocks: &[PanelBlock]) -> Flattened {
    let mut flat = Flattened::default();
    for block in blocks {
        let start = flat.lines.len();
        flat.lines.extend(block.lines.iter().cloned());
        flat.rows.extend(block.rows.iter().copied());
        flat.spaces.extend(block.spaces.iter().cloned());
        flat.bands.extend(block.bands.iter().copied());
        flat.ranges.push(start..flat.lines.len());
    }
    flat
}

#[derive(Default)]
struct Flattened {
    lines: Vec<Line<'static>>,
    rows: Vec<Option<usize>>,
    spaces: Vec<Option<String>>,
    bands: Vec<Band>,
    ranges: Vec<std::ops::Range<usize>>,
}

impl Flattened {
    fn push_indicator(&mut self, line: Line<'static>) {
        self.lines.push(line);
        self.rows.push(None);
        self.spaces.push(None);
        self.bands.push(Band::None);
    }
}

fn more_line(up: bool, count: usize) -> Line<'static> {
    let mark = if up { '↑' } else { '↓' };
    Line::from(Span::styled(format!("{mark} {count} more"), Style::default().fg(theme::DIM)))
}

/// Build the parallel line arrays for one consecutive range of blocks.
fn append_blocks(target: &mut Flattened, source: &Flattened, from: usize, to: usize) {
    if from >= to {
        return;
    }
    let start = source.ranges[from].start;
    let end = source.ranges[to - 1].end;
    target.lines.extend(source.lines[start..end].iter().cloned());
    target.rows.extend(source.rows[start..end].iter().copied());
    target.spaces.extend(source.spaces[start..end].iter().cloned());
    target.bands.extend(source.bands[start..end].iter().copied());
}

/// All panel lines that fit, plus an offset that is always zero: the returned
/// arrays are the viewport itself. Clipping happens only at block edges, and
/// each clipped edge becomes one dim `↑ N more` / `↓ N more` line. The blank
/// row and the action row stay pinned at the foot.
fn panel_view(app: &App, width: u16, wide: bool, height: u16) -> PanelView {
    let blocks = panel_blocks(app, width, wide);
    let flat = flatten(&blocks);
    let height = height as usize;
    if height == 0 || flat.lines.len() <= height {
        let len = flat.lines.len();
        return PanelView { lines: flat.lines, rows: flat.rows, spaces: flat.spaces, bands: flat.bands, offset: 0, len };
    }

    // The last two blocks are the separator and the actions: chrome that must
    // stay on screen even when the list above it is clipped.
    let foot = blocks.len().saturating_sub(2);
    let foot_lines = blocks[foot..].iter().map(|block| block.lines.len()).sum::<usize>();
    let room = height.saturating_sub(foot_lines).min(flat.ranges[foot].start);
    let selected = blocks[..foot]
        .iter()
        .position(|block| block.rows.contains(&Some(app.cursor)))
        .or_else(|| blocks[..foot].iter().position(|block| block.item))
        .unwrap_or(0);

    let mut lo = selected;
    let mut hi = selected;
    let mut used = flat.ranges[selected].end - flat.ranges[selected].start;
    loop {
        let spare = room.saturating_sub(usize::from(lo > 0) + usize::from(hi + 1 < foot));
        let above = lo.checked_sub(1).map(|at| flat.ranges[at].end - flat.ranges[at].start);
        let below = blocks.get(hi + 1).filter(|_| hi + 1 < foot).map(|block| block.lines.len());
        if let Some(size) = above
            && used + size <= spare
        {
            lo -= 1;
            used += size;
            continue;
        }
        if let Some(size) = below
            && used + size <= spare
        {
            hi += 1;
            used += size;
            continue;
        }
        break;
    }

    // Clipping a lone section header is not worth an indicator: keep those
    // chrome lines on screen instead of claiming "0 more".
    if lo > 0 && !blocks[..lo].iter().any(|block| block.item) {
        let extra = flat.ranges[lo - 1].end - flat.ranges[0].start;
        if used + extra <= room.saturating_sub(usize::from(hi + 1 < foot)) {
            used += extra;
            lo = 0;
        }
    }
    if hi + 1 < foot && !blocks[hi + 1..foot].iter().any(|block| block.item) {
        let extra = flat.ranges[foot - 1].end - flat.ranges[hi + 1].start;
        if used + extra <= room.saturating_sub(usize::from(lo > 0)) {
            hi = foot - 1;
        }
    }

    let above = blocks[..lo].iter().filter(|block| block.item).count();
    let below = blocks[hi + 1..foot].iter().filter(|block| block.item).count();
    let mut view = Flattened::default();
    if lo > 0 {
        view.push_indicator(more_line(true, above));
    }
    append_blocks(&mut view, &flat, lo, hi + 1);
    if hi + 1 < foot {
        view.push_indicator(more_line(false, below));
    }
    append_blocks(&mut view, &flat, foot, blocks.len());
    let len = view.lines.len();
    PanelView { lines: view.lines, rows: view.rows, spaces: view.spaces, bands: view.bands, offset: 0, len }
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

/// The panel's content as chrome lines and two-row item blocks.
fn panel_blocks(app: &App, width: u16, wide: bool) -> Vec<PanelBlock> {
    let mut blocks = Vec::new();

    if wide {
        blocks.push(PanelBlock::chrome(header_line(app, width)));
    }
    if let Some(prompt) = &app.prompt {
        let text = match prompt {
            super::app::Prompt::RenameSpace { buffer, .. } => format!("space name: {buffer}"),
            super::app::Prompt::RenameTab { buffer, .. } => format!("tab name: {buffer}"),
            super::app::Prompt::ConfirmClose { label, .. } => format!("{label} [y/N]"),
        };
        blocks.push(PanelBlock::chrome(Line::from(Span::styled(
            fit(&text, width as usize),
            Style::default().fg(theme::ACCENT),
        ))));
    }

    let home = crate::mods::home().display().to_string();
    blocks.push(PanelBlock::chrome(section("spaces", width)));
    let mut nested = false;
    for space in &app.model.spaces {
        let name = app.model.space_name(space, &home);
        let active = app.model.active.as_deref() == Some(space.id.as_str());
        blocks.push(PanelBlock::item(
            space_line(&name, active, width),
            space_detail(&space.folder, space.tabs.len(), width),
            None,
            Some(space.id.clone()),
            active,
            0,
        ));
        if active {
            // Only the chosen space expands. Its tabs are child blocks of
            // this space, and their own terminals step one level further in.
            nested = true;
            append_tab_blocks(&mut blocks, app, width);
        }
    }
    if !nested {
        // Before a workspace document arrives the core's top-level terminals
        // are the tabs; they still need blocks, without a space to nest under.
        append_tab_blocks(&mut blocks, app, width);
    }

    let agents = app.agents();
    if !agents.is_empty() {
        blocks.push(PanelBlock::chrome(section("agents", width)));
        for agent in agents {
            let target = app.rows.iter().position(|row| {
                row.address.as_deref() == Some(agent.address.as_str())
                    || row.address.as_deref() == agent.parent.as_deref()
            });
            blocks.push(PanelBlock::item(agent_line(agent, width), agent_detail(agent, width), target, None, false, 0));
        }
    }

    if let Some(terminal) = app.selected()
        && !terminal.preview.is_empty()
    {
        blocks.push(PanelBlock::chrome(task_line(&terminal.name, width)));
    }

    // The actions sit at the foot of the panel, not in a status bar.
    blocks.push(PanelBlock::chrome(Line::from("")));
    let actions = action_labels(wide).join("  ");
    blocks.push(PanelBlock::chrome(Line::from(Span::styled(actions, Style::default().fg(theme::DIM)))));
    blocks
}

/// One tab and its terminals, as two-row children. Both rows keep the tab's
/// index as their hit target; a terminal's tap selects the tab it belongs to.
fn append_tab_blocks(blocks: &mut Vec<PanelBlock>, app: &App, width: u16) {
    for (index, row) in app.rows().iter().enumerate() {
        let selected = index == app.cursor;
        blocks.push(PanelBlock::item(
            row_line(row, selected, width),
            row_detail(app, row, width),
            Some(index),
            None,
            selected,
            2,
        ));
        if let Some(address) = &row.address {
            for child in app.children_of(address) {
                blocks.push(PanelBlock::item(
                    child_line_top(child, width),
                    child_line_detail(child, width),
                    Some(index),
                    None,
                    false,
                    4,
                ));
            }
        }
    }
}

/// The four workspace actions rendered inside the sidebar's footer.
fn action_labels(wide: bool) -> [&'static str; 4] {
    if wide { ["new sp", "new tb", "ren", "x"] } else { ["new space", "new tab", "rename", "close"] }
}

/// Which workspace action a tap on the sidebar footer selects.
pub fn panel_action_at(app: &App, area: Rect, x: u16, y: u16, wide: bool) -> Option<super::workspace::Command> {
    let inner = panel_inner(area);
    if x < inner.x || x >= inner.x.saturating_add(inner.width) || y < inner.y || y >= inner.y.saturating_add(inner.height) {
        return None;
    }
    let view = panel_view(app, inner.width, wide, inner.height);
    let local = (y - inner.y) as usize + view.offset;
    let lines = &view.lines;
    let labels = action_labels(wide);
    let joined = labels.join("  ");
    if rendered(lines.get(local)?) != joined {
        return None;
    }
    let column = (x - inner.x) as usize;
    let mut start = 0;
    for (label, command) in labels.iter().zip([
        super::workspace::Command::NewSpace,
        super::workspace::Command::NewTab,
        super::workspace::Command::RenameTab,
        super::workspace::Command::CloseTab,
    ]) {
        let end = start + label.chars().count();
        if column >= start && column < end {
            return Some(command);
        }
        start = end + 2;
    }
    None
}

fn rendered(line: &Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
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

fn section(label: &str, width: u16) -> Line<'static> {
    Line::from(Span::styled(
        fit(label, width as usize),
        Style::default().fg(theme::MUTED).add_modifier(Modifier::BOLD),
    ))
}

/// One workspace: a coloured dot, then its name. The tab count moved to the
/// detail row, next to the folder it belongs to.
fn space_line(name: &str, active: bool, width: u16) -> Line<'static> {
    let dot = if active { theme::ACCENT } else { theme::MUTED };
    let label = Style::default().fg(theme::TEXT);
    let label = if active { label.add_modifier(Modifier::BOLD) } else { label };
    Line::from(vec![
        Span::raw("  "),
        Span::styled("●", Style::default().fg(dot)),
        Span::raw(" "),
        Span::styled(fit(name, width.saturating_sub(4) as usize), label),
    ])
}

/// The first row of a tab block: state glyph plus name, nothing else.
fn row_line(row: &super::app::Row, selected: bool, width: u16) -> Line<'static> {
    let left = format!("  {} {}", row.state.glyph(), row.name);
    let label = Style::default().fg(theme::TEXT);
    let label = if selected { label.add_modifier(Modifier::BOLD) } else { label };
    Line::from(Span::styled(fit(&left, width as usize), label))
}

/// The pane count of a document tab, when the workspace is available.
fn pane_count(app: &App, row: &super::app::Row) -> Option<usize> {
    let space = app.model.active_space()?;
    let tab = space.tabs.iter().find(|tab| tab.id == row.id)?;
    Some(tab.root.panes().len())
}

/// The second row of a tab block: `state · app`, then panes and helpers when
/// present. Two cells deeper than its name, on the same continuous band.
fn row_detail(app: &App, row: &super::app::Row, width: u16) -> Line<'static> {
    let terminal = row.address.as_ref().and_then(|addr| app.terminals.iter().find(|t| t.address == *addr));
    let app_name = terminal.and_then(|t| t.app.as_deref()).filter(|name| !name.is_empty());
    let helpers = row.address.as_ref().map_or(0, |address| app.children_of(address).len());
    let mut parts = vec![row.state.word().to_string()];
    if let Some(name) = app_name {
        parts.push(name.to_string());
    }
    if let Some(panes) = pane_count(app, row)
        && panes > 0
    {
        parts.push(format!("{panes} {}", if panes == 1 { "pane" } else { "panes" }));
    }
    if helpers > 0 {
        parts.push(format!("{helpers} {}", if helpers == 1 { "helper" } else { "helpers" }));
    }
    detail_line(&parts.join(" · "), 4, width)
}

/// A space's folder and tab count, with the path's tail preserved when it
/// must truncate. The count is part of the tail, so it never disappears.
fn space_detail(folder: &str, tabs: usize, width: u16) -> Line<'static> {
    let count = format!("{} {}", tabs, tab_word(tabs));
    let text = if folder.is_empty() { count } else { format!("{folder} · {count}") };
    detail_line(&fit_path(&text, width.saturating_sub(6) as usize), 4, width)
}

/// Tail-preserving path truncation, safe for multi-byte path components.
fn fit_path(path: &str, width: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= width {
        return path.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let separator = chars.iter().rposition(|c| *c == '/' || *c == '\\');
    let tail: Vec<char> = match separator {
        Some(at) => chars[at + 1..].to_vec(),
        None => chars.clone(),
    };
    if tail.len() >= width {
        let head: String = tail.into_iter().take(width - 1).collect();
        return format!("{head}…");
    }
    let head: String = chars.iter().take(width - tail.len() - 1).collect();
    let tail: String = tail.into_iter().collect();
    format!("{head}…{tail}")
}

/// Child tab name: two cells deeper than its parent, state glyph included.
fn child_line_top(terminal: &super::app::Terminal, width: u16) -> Line<'static> {
    let left = format!("    {} {}", terminal.state.glyph(), terminal.name);
    Line::from(Span::styled(fit(&left, width as usize), Style::default().fg(theme::TEXT)))
}

/// Child tab detail: two cells deeper than the child name, `state · app`.
fn child_line_detail(terminal: &super::app::Terminal, width: u16) -> Line<'static> {
    let app_name = terminal.app.as_deref().filter(|name| !name.is_empty());
    let mut parts = vec![terminal.state.word().to_string()];
    if let Some(name) = app_name {
        parts.push(name.to_string());
    }
    detail_line(&parts.join(" · "), 6, width)
}

/// One detected agent's name row: glyph plus name, state kept for its detail.
fn agent_line(agent: &super::app::Terminal, width: u16) -> Line<'static> {
    let left = format!("  {} {}", agent.state.glyph(), agent.name);
    Line::from(Span::styled(fit(&left, width as usize), Style::default().fg(agent.state.color())))
}

/// Agent detail row: `state · app`, aligned two cells deeper than its name.
fn agent_detail(agent: &super::app::Terminal, width: u16) -> Line<'static> {
    let app_name = agent.app.as_deref().filter(|name| !name.is_empty());
    let mut parts = vec![agent.state.word().to_string()];
    if let Some(name) = app_name {
        parts.push(name.to_string());
    }
    detail_line(&parts.join(" · "), 4, width)
}

/// One detail row, indented `indent` cells and truncated with `…`.
fn detail_line(text: &str, indent: usize, width: u16) -> Line<'static> {
    let available = (width as usize).saturating_sub(indent);
    Line::from(Span::styled(
        format!("{}{}", " ".repeat(indent), fit(text, available)),
        Style::default().fg(theme::MUTED),
    ))
}

fn task_line(name: &str, width: u16) -> Line<'static> {
    let label = format!("─ {name} ");
    let line = format!("{label}{}", "─".repeat((width as usize).saturating_sub(label.chars().count())));
    Line::from(Span::styled(line, Style::default().fg(theme::DIM)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::App;
    use crate::tui::workspace::Command;
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

    /// Two spaces: `alpha` is active with one tab, `beta` is collapsed with two.
    fn spaces_app() -> App {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [
                { "id": "s1", "name": "alpha", "folder": "/home/dsasco/alpha", "tabs": [
                    { "id": "tb1", "name": "one", "root": { "id": "tb1", "type": "row", "children": [] } }
                ] },
                { "id": "s2", "name": "beta", "folder": "/home/dsasco/beta", "tabs": [
                    { "id": "tb2", "name": "two", "root": { "id": "tb2", "type": "row", "children": [] } },
                    { "id": "tb3", "name": "three", "root": { "id": "tb3", "type": "row", "children": [] } }
                ] }
            ] }
        }));
        app.set_terminals(&json!({
            "hidden_finished": 0, "machines": [],
            "terminals": [{ "address": "t9", "name": "pi", "status": { "state": "working" } }]
        }));
        app.toggle_panel();
        app
    }

    /// The panel's rows, paired with the tab index and space each line belongs to.
    fn panel_content(app: &App, width: u16, wide: bool) -> (Vec<Line<'static>>, Vec<Option<usize>>, Vec<Option<String>>) {
        let flat = flatten(&panel_blocks(app, width, wide));
        (flat.lines, flat.rows, flat.spaces)
    }

    /// The real buffer's backgrounds, for band assertions that cannot be
    /// fooled by a line's own spans.
    fn backgrounds(app: &App, width: u16, height: u16) -> Vec<Vec<ratatui::style::Color>> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
        terminal.draw(|frame| render(app, frame)).expect("draw");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].bg).collect())
            .collect()
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
    fn the_band_zone_only_exists_on_a_phone() {
        let phone = zones(Rect::new(0, 0, 40, 22), false);
        assert!(Zones::contains(phone.header, 10, 0));
        assert!(Zones::contains(phone.header, 10, 2));
        assert!(!Zones::contains(phone.header, 10, 3));
        assert!(Zones::contains(Some(phone.terminal), 10, 3));
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
        let panel = zones(Rect::new(0, 0, 40, 22), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let (_, rows, _) = panel_content(&app, inner.width, false);
        let line = rows.iter().position(|row| *row == Some(0)).expect("first tab row");
        let x = inner.x + 2;
        let y = inner.y + line as u16;
        assert_eq!(panel_row_at(&app, panel, x, y, false), Some(0));
        assert_eq!(panel_row_at(&app, panel, panel.x, y, false), None, "the gutter is not a row");
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
        let cell = terminal.backend().buffer()[(0, BAND_ROWS)].clone();
        assert_eq!(cell.symbol(), "X");
        assert_eq!(cell.fg, ratatui::style::Color::Red);
        assert_eq!(cell.bg, ratatui::style::Color::Blue);
        assert!(cell.modifier.contains(Modifier::BOLD));
        let rows = screen(&app, 40, 22);
        assert!(rows[0].contains("scrolled"), "the band states the mode: {:?}", &rows[..3]);
    }

    #[test]
    fn a_wide_screen_keeps_the_rail_on_the_left() {
        let rows = screen(&app(true), 100, 24);
        assert!(rows[1].contains("default"), "{:?}", &rows[..3]);
        let rail = rows.iter().map(|row| row.chars().take(RAIL_WIDTH as usize).collect::<String>()).collect::<Vec<_>>();
        assert!(rail.iter().any(|row| row.trim() == "spaces"), "rail sections start after the gutter: {rail:?}");
        // The rule sits where the rail ends; the terminal area is to its right.
        assert!(rows.iter().any(|row| row.chars().nth(RAIL_WIDTH as usize) == Some('│')), "no rule after the rail");
    }

    #[test]
    fn a_narrow_screen_with_the_bar_closed_is_the_terminal_below_it() {
        let rows = screen(&app(false), 40, 22);
        assert!(rows[0].contains("pi 2"), "band row: {:?}", rows[0]);
        assert!(rows[1].contains("switch"), "the toggle is centred in the band: {:?}", rows[1]);
        assert!(!rows.iter().any(|row| row.trim() == "spaces"), "the panel is closed: {rows:?}");
        assert!(!rows.iter().any(|row| row.contains("new space")), "no actions while closed: {rows:?}");
    }

    #[test]
    fn opening_the_panel_on_a_narrow_screen_covers_it_with_sections() {
        let rows = screen(&app(true), 40, 22);
        assert!(rows[1].trim_end().ends_with('x'), "the toggle turns into x: {:?}", rows[1]);
        assert!(rows.iter().any(|row| row.trim() == "spaces"), "{rows:?}");
        assert!(rows.iter().any(|row| row.trim() == "agents"), "{rows:?}");
        assert!(rows.iter().any(|row| row.contains("needs you")), "{rows:?}");
        assert!(rows.iter().any(|row| row.contains("new space")), "actions at the foot: {rows:?}");
    }

    #[test]
    fn the_band_names_the_workspace_once() {
        for panel in [false, true] {
            let rows = screen(&app(panel), 40, 22);
            assert_eq!(rows[0].matches("pi 2").count(), 1, "panel={panel}: {:?}", rows[0]);
        }
    }

    #[test]
    fn there_is_no_status_bar_at_the_bottom() {
        // The wide screen's last row belongs to the terminal area, not to chrome.
        let rows = screen(&app(true), 100, 24);
        let last = rows.last().expect("a last row");
        let rail_side: String = last.chars().take(RAIL_WIDTH as usize).collect();
        let terminal_side: String = last.chars().skip(RAIL_WIDTH as usize).collect();
        assert!(!terminal_side.contains("new sp"), "the terminal area is not chrome: {terminal_side:?}");
        assert!(!rail_side.contains("new sp") || terminal_side.trim().is_empty(), "{rail_side:?}");
    }

    #[test]
    fn whoever_needs_you_is_the_first_row() {
        let rows = screen(&app(true), 40, 22);
        let first = rows.iter().position(|row| row.contains("▲ pi 2")).expect("blocked tab");
        assert!(rows[first + 1].contains("needs you"), "state is on the detail row: {rows:?}");
    }

    #[test]
    fn the_toggle_words_are_switch_and_x() {
        assert!(screen(&app(false), 40, 22)[1].contains("switch"));
        assert!(screen(&app(true), 40, 22)[1].trim_end().ends_with('x'));
    }

    #[test]
    fn a_phone_band_has_three_rows_and_a_fixed_toggle() {
        let mut app = app(false);
        app.path = "~/src/herdr/herdr-app-tui".to_string();
        let rows = screen(&app, 40, 20);
        assert!(rows[0].contains("pi 2") && rows[0].contains("tabs"), "{:?}", &rows[..3]);
        assert!(rows[1].contains("~/src/herdr/herdr-app-tui"), "{:?}", &rows[..3]);
        assert!(rows[2].contains("○ idle") && rows[2].contains("▲ needs"), "{:?}", &rows[..3]);
        let toggle_x = (40 - TOGGLE_COLS) as usize;
        assert!(rows[1][toggle_x..].contains("switch"), "{:?}", rows[1]);
        let z = zones(Rect::new(0, 0, 40, 20), false);
        assert_eq!(z.header.expect("a phone band").height, BAND_ROWS);
    }

    #[test]
    fn a_phone_tab_target_spans_two_cells_inside_the_gutter() {
        let app = app(true);
        let z = zones(Rect::new(0, 0, 40, 20), true);
        let panel = z.panel.expect("the phone sheet");
        let inner = Rect::new(
            panel.x + GUTTER,
            panel.y + GUTTER,
            panel.width.saturating_sub(2 * GUTTER),
            panel.height.saturating_sub(2 * GUTTER),
        );
        let (_, lines, _) = panel_content(&app, inner.width, false);
        let first = lines.iter().position(|line| *line == Some(0)).expect("the first tab");
        assert_eq!(lines[first], lines[first + 1], "both cells belong to one target");
        let y = inner.y + first as u16;
        assert_eq!(panel_row_at(&app, panel, inner.x + 2, y, false), Some(0));
        assert_eq!(panel_row_at(&app, panel, inner.x + 2, y + 1, false), Some(0));
    }

    #[test]
    fn the_sidebar_keeps_a_one_cell_gutter_on_desktop() {
        let rows = screen(&app(true), 100, 24);
        let header = rows.get(1).expect("the first rail row");
        assert_eq!(header.chars().next(), Some(' '), "left gutter: {header:?}");
        assert_ne!(header.chars().nth(1), Some(' '), "content starts at column 1: {header:?}");
    }

    #[test]
    fn the_band_names_the_pane_state() {
        let plain = screen(&app(false), 40, 20);
        assert!(plain[0].contains("preview"), "{:?}", plain[0]);

        let mut attached = app(false);
        attached.screen = Some(ScreenState { rows: Vec::new(), cursor: None });
        let live = screen(&attached, 40, 20);
        assert!(live[0].contains("attached"), "{:?}", live[0]);

        attached.enter_copy();
        let scrolled = screen(&attached, 40, 20);
        assert!(scrolled[0].contains("scrolled"), "{:?}", scrolled[0]);
    }

    #[test]
    fn the_agents_section_lists_navigable_agents() {
        let rows = screen(&app(true), 40, 24);
        let start = rows.iter().position(|row| row.trim() == "agents").expect("the agents section");
        let section = &rows[start + 1..];
        let blocked = section.iter().position(|row| row.contains("▲ pi 2")).expect("blocked agent");
        assert!(section[blocked + 1].contains("needs you"), "{section:?}");
        let working = section.iter().position(|row| row.contains("◐ pi 3")).expect("working agent");
        assert!(section[working + 1].contains("working"), "{section:?}");
        assert!(!section.iter().any(|row| row.trim() == "◐ working 1"), "no statistic row: {section:?}");
    }

    #[test]
    fn a_plain_shell_does_not_appear_as_an_agent() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "hidden_finished": 0,
            "machines": [],
            "terminals": [
                { "address": "t1", "name": "bash", "state": "live" },
                { "address": "t2", "name": "pi", "status": { "state": "working" } }
            ]
        }));
        app.toggle_panel();
        let rows = screen(&app, 40, 20);
        let start = rows.iter().position(|row| row.trim() == "agents").expect("the agents section");
        let section = &rows[start + 1..];
        assert!(!section.iter().any(|row| row.contains("bash")), "no plain shell: {section:?}");
        let agent = section.iter().position(|row| row.contains("◐ pi")).expect("the real agent");
        assert!(section[agent + 1].contains("working"), "its state is on the detail row: {section:?}");
        let targets = agent_targets(&app);
        assert_eq!(targets, vec![0], "Alt+Up/Down skips the shell");
        assert_eq!(app.rows()[targets[0]].address.as_deref(), Some("t2"));

        let mut shell_only = App::new("default", "local");
        shell_only.set_terminals(&json!({
            "hidden_finished": 0, "machines": [],
            "terminals": [{ "address": "t1", "name": "bash", "state": "live" }]
        }));
        shell_only.toggle_panel();
        let rows = screen(&shell_only, 40, 20);
        assert!(!rows.iter().any(|row| row.trim() == "agents"), "no phantom section: {rows:?}");
    }

    #[test]
    fn tabs_live_inside_the_active_space_not_in_a_tabs_section() {
        let app = spaces_app();
        let panel = zones(Rect::new(0, 0, 40, 24), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let (lines, rows, spaces) = panel_content(&app, inner.width, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();

        assert!(!texts.iter().any(|line| line.contains("TABS") || line.trim() == "tabs"), "no separate TABS section: {texts:?}");
        let spaces_label = texts.iter().position(|line| line.trim() == "spaces").expect("the spaces label");
        let agents_label = texts.iter().position(|line| line.trim() == "agents").expect("the agents label");
        let alpha = texts.iter().position(|line| line.contains("alpha")).expect("the active space");
        let one = texts.iter().position(|line| line.contains("one")).expect("its tab");
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the inactive space");
        assert!(spaces_label < alpha && alpha < one && one < beta && beta < agents_label, "tab under its space: {texts:?}");
        assert!(!texts.iter().any(|line| line.contains("two") || line.contains("three")), "beta stays collapsed: {texts:?}");

        assert_eq!(rows[one], Some(0), "the tab's first row maps to its index");
        assert_eq!(rows[one + 1], Some(0), "the tab's second row maps to its index");
        assert_eq!(spaces[alpha], Some("s1".to_string()));
        assert_eq!(spaces[alpha + 1], Some("s1".to_string()));
        assert_eq!(rows[beta], None, "the collapsed space is not a tab");

        let flat = flatten(&panel_blocks(&app, inner.width, false));
        assert!(matches!(flat.bands[one], Band::Item { child: true, .. }), "a nested tab is a child block");
        assert_eq!(flat.bands[one], flat.bands[one + 1], "one band across both rows");

        let view = panel_view(&app, inner.width, false, inner.height);
        let target = view.spaces.iter().position(|space| space.as_deref() == Some("s2")).expect("the beta row");
        let y = inner.y + (target - view.offset) as u16;
        assert_eq!(panel_space_at(&app, panel, inner.x + 2, y, false), Some("s2".to_string()));
    }

    #[test]
    fn selecting_a_space_switches_which_tabs_are_nested() {
        let mut app = spaces_app();
        app.model.active = Some("s2".to_string());
        app.rebuild_rows();
        let panel = zones(Rect::new(0, 0, 40, 24), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let (lines, rows, _) = panel_content(&app, inner.width, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        assert!(!texts.iter().any(|line| line.contains("one")), "alpha collapsed: {texts:?}");
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the selected space");
        let two = texts.iter().position(|line| line.contains("two")).expect("its first tab");
        let three = texts.iter().position(|line| line.contains("three")).expect("its second tab");
        assert!(beta < two && two < three, "tabs nested under beta: {texts:?}");
        assert_eq!(rows[two], Some(0), "first tab maps to row 0");
        assert_eq!(rows[three], Some(1), "second tab maps to row 1");
    }

    #[test]
    fn sections_are_lowercase_and_carry_no_counts_or_disclosure_glyphs() {
        let app = spaces_app();
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        assert_eq!(texts.iter().filter(|line| line.trim() == "spaces").count(), 1, "{texts:?}");
        assert_eq!(texts.iter().filter(|line| line.trim() == "agents").count(), 1, "{texts:?}");
        assert!(!texts.iter().any(|line| line.contains('▾')), "no disclosure glyph: {texts:?}");
        assert!(
            !texts.iter().any(|line| line.contains("SPACES") || line.contains("AGENTS") || line.contains("TABS")),
            "no uppercase section: {texts:?}"
        );
    }

    #[test]
    fn space_rows_use_a_coloured_dot_and_a_detailed_tab_count() {
        let app = spaces_app();
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let alpha = texts.iter().position(|line| line.contains("alpha")).expect("the active space");
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the inactive space");
        assert!(texts[alpha].starts_with("  ● "), "dot plus name: {:?}", texts[alpha]);
        assert!(texts[alpha + 1].contains("/home/dsasco/alpha · 1 tab"), "path and singular count: {:?}", texts[alpha + 1]);
        assert!(texts[beta + 1].contains("/home/dsasco/beta · 2 tabs"), "path and plural count: {:?}", texts[beta + 1]);
        let dot = |line: &Line<'static>| {
            line.spans
                .iter()
                .find(|span| span.content.as_ref() == "●")
                .and_then(|span| span.style.fg)
                .expect("a coloured dot")
        };
        assert_eq!(dot(&lines[alpha]), theme::ACCENT, "active space dot");
        assert_eq!(dot(&lines[beta]), theme::MUTED, "inactive space dot");
    }

    #[test]
    fn one_tab_is_singular_in_the_band() {
        let mut app = app(false);
        app.cursor = 0;
        let rows = screen(&app, 40, 20);
        assert!(rows[0].contains("3 tabs") || rows[0].contains("1 tab ·"), "{:?}", rows[0]);
        let mut single = App::new("default", "local");
        single.set_terminals(&json!({
            "terminals": [{ "address": "t1", "name": "solo", "status": { "state": "idle" } }],
            "hidden_finished": 0,
            "machines": [],
        }));
        let rows = screen(&single, 40, 20);
        assert!(rows[0].contains("1 tab ·"), "{:?}", rows[0]);
        assert!(!rows[0].contains("1 tabs"), "{:?}", rows[0]);
    }

    #[test]
    fn the_rail_abbreviates_its_actions() {
        let rows = screen(&app(true), 100, 24);
        assert!(rows.iter().any(|row| row.contains("new sp  new tb  ren  x")), "{rows:?}");
    }

    #[test]
    fn the_sidebar_footer_maps_taps_to_workspace_commands() {
        let app = app(true);
        let panel = zones(Rect::new(0, 0, 40, 20), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let view = panel_view(&app, inner.width, false, inner.height);
        let lines = &view.lines;
        let row = lines.iter().position(|line| rendered(line) == "new space  new tab  rename  close").expect("the action row");
        let y = inner.y + (row as u16).saturating_sub(view.offset as u16);
        assert_eq!(panel_action_at(&app, panel, inner.x, y, false), Some(Command::NewSpace));
        assert_eq!(panel_action_at(&app, panel, inner.x + 28, y, false), Some(Command::CloseTab));
        assert_eq!(panel_action_at(&app, panel, inner.x, y + 1, false), None);
    }

    #[test]
    fn a_live_shell_is_idle_and_hidden_finished_stays_quiet() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [{ "address": "t9", "name": "shell", "state": "live" }],
            "hidden_finished": 11,
            "machines": [],
        }));
        app.toggle_panel();
        let joined = screen(&app, 40, 20).join("\n");
        assert!(joined.contains("○ idle"), "{joined}");
        assert!(!joined.contains("unknown"), "{joined}");
        assert!(!joined.contains("finished hidden"), "{joined}");
    }

    #[test]
    fn the_prefix_shows_a_floating_binding_panel() {
        let mut app = app(false);
        app.prefix = true;
        let rows = screen(&app, 40, 20);
        let joined = rows.join("\n");
        assert!(joined.contains("WORKSPACE PREFIX"), "{joined}");
        assert!(joined.contains("c  new tab"), "{joined}");
        assert!(joined.contains("esc cancels"), "{joined}");
    }

    #[test]
    fn the_selected_tab_stays_visible_when_the_panel_scrolls() {
        let terminals: Vec<serde_json::Value> = (1..=8)
            .map(|index| {
                json!({
                    "address": format!("t{index}"),
                    "name": format!("pi {index}"),
                    "status": { "state": "idle" },
                })
            })
            .collect();
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({ "terminals": terminals, "hidden_finished": 0, "machines": [] }));
        app.toggle_panel();
        app.cursor = 7;
        let rows = screen(&app, 40, 10);
        let sheet = &rows[(BAND_ROWS + GUTTER) as usize..];
        assert!(sheet.iter().any(|row| row.contains("pi 8")), "selection left the viewport: {sheet:?}");
    }

    // ---- RED tests for 04 item blocks ----

    /// T1: a space owns two consecutive rows; the detail shows the folder
    /// path with a tail-preserving ellipsis.
    #[test]
    fn a_space_detail_shows_the_folder_path_with_a_tail_ellipsis() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [
                { "id": "s1", "name": "work", "folder": "/home/user/src/herdr/herdr-app-tui/with/a/very/long/name", "tabs": [] }
            ] }
        }));
        app.toggle_panel();
        let rows = screen(&app, 40, 20);
        let name = rows.iter().position(|row| row.contains("● work")).expect("space name");
        let detail = &rows[name + 1];
        assert!(detail.contains('…'), "one ellipsis, not three dots: {detail:?}");
        assert!(!detail.contains("..."), "no ASCII triple dot: {detail:?}");
        assert!(detail.trim_end().ends_with("name · 0 tabs"), "path tail and count preserved: {detail:?}");
        assert!(detail.starts_with("     /home/"), "path head kept before the ellipsis: {detail:?}");
    }

    #[test]
    fn a_multibyte_path_truncates_without_cutting_a_character() {
        assert_eq!(fit_path("α/βγδεζη", 5), "βγδε…");
        let path = "/home/ünïcøde/herdr/tui";
        let fitted = fit_path(path, 8);
        assert!(fitted.contains('…'), "{fitted:?}");
        assert!(fitted.ends_with("tui"), "tail survives: {fitted:?}");
        assert!(fitted.chars().count() <= 8, "at most the width: {fitted:?}");
    }

    /// T2: a child terminal's band starts two cells past its tab. Nested
    /// tabs themselves step two cells past their space.
    #[test]
    fn a_child_band_steps_two_cells_past_its_parent() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "parent-tab", "status": { "state": "idle" } },
                { "address": "t2", "name": "child-tab", "status": { "state": "idle" }, "parent": "t1" }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        let rows = screen(&app, 40, 20);
        let body = &rows[(BAND_ROWS + GUTTER) as usize..];
        let parent = body.iter().find(|row| row.contains("parent-tab")).expect("parent row");
        let child = body.iter().find(|row| row.contains("child-tab")).expect("child row");
        let parent_glyph = parent.chars().position(|c| c == '○').expect("parent glyph");
        let child_glyph = child.chars().position(|c| c == '○').expect("child glyph");
        assert_eq!(child_glyph - parent_glyph, 2, "hierarchy is a two-cell step: {parent:?} / {child:?}");

        // The step is empty canvas: only the band from its own step onward
        // is PANEL. A terminal is one level deeper than its tab.
        let z = zones(Rect::new(0, 0, 40, 20), true);
        let inner = panel_inner(z.panel.expect("sheet"));
        let bg = backgrounds(&app, 40, 20);
        let view = panel_view(&app, inner.width, false, inner.height);
        let parent_line = view
            .bands
            .iter()
            .position(|band| band.step() == 2)
            .expect("the tab's band");
        let child_line = view
            .bands
            .iter()
            .position(|band| band.step() == 4)
            .expect("the terminal's band");
        assert_eq!(view.bands[child_line].step() - view.bands[parent_line].step(), 2);
        for (line, band) in view.bands.iter().enumerate().take(view.len) {
            let step = band.step();
            let Some(color) = band.color() else { continue };
            let y = (inner.y + line as u16) as usize;
            for x in inner.x..inner.x + step {
                assert_eq!(bg[y][x as usize], theme::BG, "step cell is BG: line {line}");
            }
            for x in inner.x + step..inner.x + inner.width {
                assert_eq!(bg[y][x as usize], color, "band is painted: line {line}");
            }
        }
    }

    /// T2: no connector glyphs anywhere in the sidebar.
    #[test]
    fn no_connector_glyphs_in_the_sidebar() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "pi", "status": { "state": "idle" } },
                { "address": "t2", "name": "claude", "status": { "state": "idle" }, "parent": "t1" }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        let rows = screen(&app, 40, 20);
        for row in &rows {
            assert!(!row.contains('│'), "no │ connector: {row:?}");
            assert!(!row.contains('└'), "no └ connector: {row:?}");
        }
    }

    /// T1: each item has a PANEL band on its two rows.
    #[test]
    fn each_item_has_a_panel_band() {
        let app = app(true);
        let z = zones(Rect::new(0, 0, 40, 22), true);
        let inner = panel_inner(z.panel.expect("the phone sheet"));
        let bg = backgrounds(&app, 40, 22);
        let view = panel_view(&app, inner.width, false, inner.height);
        let mut bands = 0;
        for (line, band) in view.bands.iter().enumerate().take(view.len) {
            let Band::Item { selected, step, .. } = *band else { continue };
            bands += 1;
            let y = (inner.y + line as u16) as usize;
            let from = inner.x + step;
            let color = if selected { theme::SURFACE } else { theme::PANEL };
            for x in from..inner.x + inner.width {
                assert_eq!(bg[y][x as usize], color, "band line {line}");
            }
        }
        assert!(bands >= 12, "spaces, tabs and agents own two banded rows each: {bands}");
        // Chrome sits on BG, not on a band.
        let header = view.bands.iter().position(|band| *band == Band::None).expect("chrome");
        let y = (inner.y + header as u16) as usize;
        assert_eq!(bg[y][inner.x as usize], theme::BG);
    }

    /// T1: the selected item's two rows have a SURFACE band.
    #[test]
    fn the_selected_item_has_a_surface_band_on_both_rows() {
        let mut app = app(true);
        app.cursor = 0;
        let z = zones(Rect::new(0, 0, 40, 22), true);
        let inner = panel_inner(z.panel.expect("the phone sheet"));
        let bg = backgrounds(&app, 40, 22);
        let view = panel_view(&app, inner.width, false, inner.height);
        let selected: Vec<usize> = view
            .bands
            .iter()
            .enumerate()
            .filter_map(|(line, band)| matches!(band, Band::Item { selected: true, .. }).then_some(line))
            .collect();
        assert_eq!(selected.len(), 2, "one two-row item is selected: {selected:?}");
        let step = view.bands[selected[0]].step();
        let from = inner.x + step;
        for line in selected {
            assert_eq!(view.rows[line], Some(0), "the selected item is the current tab");
            let y = (inner.y + line as u16) as usize;
            for x in inner.x..from {
                assert_eq!(bg[y][x as usize], theme::BG, "the child step is empty canvas");
            }
            for x in from..inner.x + inner.width {
                assert_eq!(bg[y][x as usize], theme::SURFACE, "row {line}");
            }
        }
    }

    /// T3: overflow indicators appear when content exceeds the viewport.
    #[test]
    fn overflow_indicators_appear_when_content_is_too_long() {
        let terminals: Vec<serde_json::Value> = (1..=10)
            .map(|index| {
                json!({
                    "address": format!("t{index}"),
                    "name": format!("pi {index}"),
                    "status": { "state": "idle" },
                })
            })
            .collect();
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({ "terminals": terminals, "hidden_finished": 0, "machines": [] }));
        app.toggle_panel();
        let rows = screen(&app, 40, 10);
        let joined = rows.join("\n");
        assert!(joined.contains("↑"), "should show ↑ indicator: {joined}");
        assert!(joined.contains("↓"), "should show ↓ indicator: {joined}");
    }

    /// T4: both rows of a tab map to the same target for hit-testing.
    #[test]
    fn both_rows_of_a_tab_map_to_the_same_target() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "pi", "status": { "state": "idle" } }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        let panel = zones(Rect::new(0, 0, 40, 22), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let (_, rows, _) = panel_content(&app, inner.width, false);
        let first = rows.iter().position(|row| *row == Some(0)).expect("tab 0");
        assert_eq!(rows[first + 1], Some(0), "both rows of a block hit the same tab");
    }

    /// T4: both rows of a space map to the same target.
    #[test]
    fn both_rows_of_a_space_map_to_the_same_target() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [
                { "id": "s1", "name": "work", "folder": "/home/user", "tabs": [] }
            ] }
        }));
        app.toggle_panel();
        let panel = zones(Rect::new(0, 0, 40, 22), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let (_, _, spaces) = panel_content(&app, inner.width, false);
        let count = spaces.iter().filter(|s| s.as_deref() == Some("s1")).count();
        assert_eq!(count, 2, "space should have 2 rows in the mapping");
    }

    /// T4: scroll keeps selected block with one block of context.
    #[test]
    fn child_blocks_are_not_space_targets() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "parent", "status": { "state": "idle" } },
                { "address": "t2", "name": "child", "status": { "state": "idle" }, "parent": "t1" }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        let panel = zones(Rect::new(0, 0, 40, 20), true).panel.expect("sheet");
        let inner = panel_inner(panel);
        let (_, _rows, spaces) = panel_content(&app, inner.width, false);
        assert!(spaces.iter().all(|space| space.is_none()), "terminals never select a space");
        // Every banded row is either a tab target or a space target, never both.
        let flat = flatten(&panel_blocks(&app, inner.width, false));
        for (row, space) in flat.rows.iter().zip(flat.spaces.iter()) {
            assert!(row.is_none() || space.is_none(), "a block has one target kind");
        }
    }

    #[test]
    fn indicator_rows_have_no_targets() {
        let terminals: Vec<serde_json::Value> = (1..=10)
            .map(|index| json!({
                "address": format!("t{index}"),
                "name": format!("pi {index}"),
                "status": { "state": "idle" },
            }))
            .collect();
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({ "terminals": terminals, "hidden_finished": 0, "machines": [] }));
        app.toggle_panel();
        app.cursor = 5;
        let inner = panel_inner(zones(Rect::new(0, 0, 40, 10), true).panel.expect("sheet"));
        let view = panel_view(&app, inner.width, false, inner.height);
        let up = view.lines.iter().position(|line| rendered(line).starts_with('↑')).expect("top indicator");
        let down = view.lines.iter().position(|line| rendered(line).starts_with('↓')).expect("bottom indicator");
        assert_eq!(view.rows[up], None);
        assert_eq!(view.spaces[up], None);
        assert_eq!(view.rows[down], None);
        assert_eq!(view.spaces[down], None);
        assert_eq!(view.bands[up], Band::None);
        assert_eq!(view.bands[down], Band::None);
        assert!(up < down, "top indicator comes first");
    }

    #[test]
    fn detail_rows_show_state_app_and_panes() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [{ "id": "s1", "name": "work", "folder": "", "tabs": [
                { "id": "tb1", "name": "claude", "root": { "id": "tb1", "type": "row", "children": [
                    { "id": "p1", "type": "block", "content": { "terminal": "t1" } },
                    { "id": "p2", "type": "block", "content": { "terminal": "t2" } }
                ] } }
            ] }] }
        }));
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "claude", "status": { "state": "working", "app": "pi" } },
                { "address": "t2", "name": "helper", "status": { "state": "idle", "app": "bash" }, "parent": "t1" }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        let rows = screen(&app, 40, 24);
        let tab = rows.iter().position(|row| row.contains("◐ claude")).expect("tab name row");
        let detail = &rows[tab + 1];
        assert!(detail.contains("working"), "{detail:?}");
        assert!(detail.contains("pi"), "{detail:?}");
        assert!(detail.contains("2 panes"), "pane count from the layout: {detail:?}");
        let agents = rows.iter().position(|row| row.trim() == "agents").expect("agents section");
        let agent = rows[agents + 1..].iter().position(|row| row.contains("◐ claude")).expect("agent row") + agents + 1;
        assert!(rows[agent + 1].contains("working · pi"), "agent detail: {:?}", rows[agent + 1]);
    }

    #[test]
    fn scroll_keeps_selected_block_with_context() {
        let terminals: Vec<serde_json::Value> = (1..=8)
            .map(|index| {
                json!({
                    "address": format!("t{index}"),
                    "name": format!("pi {index}"),
                    "status": { "state": "idle" },
                })
            })
            .collect();
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({ "terminals": terminals, "hidden_finished": 0, "machines": [] }));
        app.toggle_panel();
        app.cursor = 6;
        let rows = screen(&app, 40, 10);
        let sheet = &rows[(BAND_ROWS + GUTTER) as usize..];
        // The selected item must be fully visible (both rows).
        assert!(sheet.iter().any(|row| row.contains("pi 7")), "selected tab pi 7 must be visible: {sheet:?}");
    }
}
