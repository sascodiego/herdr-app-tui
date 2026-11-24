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
pub const RAIL_WIDTH: u16 = 34;
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

/// Open agent addresses in attention order, whatever space they live in.
pub fn agent_targets(app: &App) -> Vec<String> {
    app.agents().iter().map(|agent| agent.address.clone()).collect()
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
        Block::new().borders(Borders::ALL).border_style(Style::default().fg(theme::LINE)).style(Style::default().bg(theme::ELEVATED)),
        area,
    );
    let lines = vec![
        Line::from(Span::styled("WORKSPACE PREFIX", Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD))),
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

/// Which agent's terminal is drawn at this screen coordinate.
pub fn panel_agent_at(app: &App, area: Rect, x: u16, y: u16, wide: bool) -> Option<String> {
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
    let agents = &view.agents;
    let offset = view.offset;
    agents.get((local as usize).saturating_add(offset)).cloned().flatten()
}

/// The three-row phone band: workspace and tabs, path, agent totals, toggle.
fn draw_header(app: &App, area: Rect, frame: &mut Frame) {
    frame.buffer_mut().set_style(area, Style::default().bg(theme::ELEVATED));
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
            Style::default().fg(theme::BODY),
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
    frame.render_widget(Paragraph::new(toggle_lines).style(Style::default().bg(theme::PANEL)), toggle);
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
        (glyph::CONNECTED, theme::GREEN)
    } else {
        (glyph::DISCONNECTED, theme::RED)
    }
}

fn draw_rule(area: Rect, frame: &mut Frame) {
    let rule = Block::new().borders(Borders::LEFT).border_style(Style::default().fg(theme::LINE));
    frame.render_widget(rule, area);
}

/// The fill painted behind one panel line. An active two-row block gets a
/// one-cell rust edge at the panel's left inset and a warm wash across the
/// rest of both rows. There is no rail and no nesting step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Band {
    None,
    Item { selected: bool },
}

/// One renderable unit. Item blocks own two lines and are the only clip and
/// scroll unit; chrome owns one line and is never clipped on its own.
#[derive(Clone)]
struct PanelBlock {
    lines: Vec<Line<'static>>,
    rows: Vec<Option<usize>>,
    spaces: Vec<Option<String>>,
    agents: Vec<Option<String>>,
    bands: Vec<Band>,
    item: bool,
}

impl PanelBlock {
    fn chrome(line: Line<'static>) -> PanelBlock {
        PanelBlock {
            lines: vec![line],
            rows: vec![None],
            spaces: vec![None],
            agents: vec![None],
            bands: vec![Band::None],
            item: false,
        }
    }

    fn item(
        top: Line<'static>,
        detail: Line<'static>,
        row: Option<usize>,
        space: Option<String>,
        agent: Option<String>,
        selected: bool,
    ) -> PanelBlock {
        let band = Band::Item { selected };
        PanelBlock {
            lines: vec![top, detail],
            rows: vec![row, row],
            spaces: vec![space.clone(), space],
            agents: vec![agent.clone(), agent],
            bands: vec![band, band],
            item: true,
        }
    }
}

/// Append an item with one blank row before it when it follows another item.
/// The blank belongs to the block it separates, so clipping never leaves a
/// half-gap. It has no hit target and no band.
fn spaced_item(blocks: &mut Vec<PanelBlock>, mut block: PanelBlock) {
    if blocks.last().is_some_and(|previous| previous.item) {
        block.lines.insert(0, Line::from(""));
        block.rows.insert(0, None);
        block.spaces.insert(0, None);
        block.agents.insert(0, None);
        block.bands.insert(0, Band::None);
    }
    blocks.push(block);
}

/// The panel: labelled sections, item blocks and actions at the foot.
fn draw_panel(app: &App, area: Rect, frame: &mut Frame, wide: bool) {
    // The rail is one step above the canvas, the phone sheet two: neither is
    // a full-bleed colour block, and no connector is drawn behind the items.
    let surface = if wide { theme::PANEL } else { theme::ELEVATED };
    frame.buffer_mut().set_style(area, Style::default().bg(surface));
    let inner = panel_inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let view = panel_view(app, inner.width, wide, inner.height);
    let take = view.len.min(inner.height as usize);
    let visible = view.lines.iter().skip(view.offset).take(take).cloned().collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(visible).style(Style::default().fg(theme::TEXT)), inner);

    // Paint after the text so only backgrounds change: the active block gets
    // a one-cell rust edge and a warm wash across the rest of both rows.
    let bands = view.bands.into_iter().skip(view.offset).take(take);
    let buffer = frame.buffer_mut();
    for (line, band) in bands.enumerate() {
        let Band::Item { selected: true } = band else { continue };
        let y = inner.y + line as u16;
        if y >= inner.y.saturating_add(inner.height) {
            break;
        }
        for x in inner.x..inner.x + inner.width {
            buffer[(x, y)].set_bg(if x == inner.x { theme::RUST_LINE } else { theme::WASH });
        }
    }
}

/// Panel lines, tab targets, space targets, their bands and the scroll offset.
struct PanelView {
    lines: Vec<Line<'static>>,
    rows: Vec<Option<usize>>,
    spaces: Vec<Option<String>>,
    agents: Vec<Option<String>>,
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
        flat.agents.extend(block.agents.iter().cloned());
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
    agents: Vec<Option<String>>,
    bands: Vec<Band>,
    ranges: Vec<std::ops::Range<usize>>,
}

impl Flattened {
    fn push_indicator(&mut self, line: Line<'static>) {
        self.lines.push(line);
        self.rows.push(None);
        self.spaces.push(None);
        self.agents.push(None);
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
    target.agents.extend(source.agents[start..end].iter().cloned());
    target.bands.extend(source.bands[start..end].iter().copied());
}

/// Clip one run of blocks to `height`, keeping the selected block and adding
/// one indicator at each clipped edge. Blocks are never split.
fn clip_blocks(blocks: &[PanelBlock], height: usize, cursor: usize) -> Flattened {
    let flat = flatten(blocks);
    if blocks.is_empty() || height == 0 || flat.lines.len() <= height {
        return flat;
    }
    let foot = blocks.len();
    let selected = blocks
        .iter()
        .position(|block| block.rows.contains(&Some(cursor)))
        .or_else(|| blocks.iter().position(|block| block.item))
        .unwrap_or(0);

    let mut lo = selected;
    let mut hi = selected;
    let mut used = flat.ranges[selected].end - flat.ranges[selected].start;
    loop {
        let spare = height.saturating_sub(usize::from(lo > 0) + usize::from(hi + 1 < foot));
        let above = lo.checked_sub(1).map(|at| flat.ranges[at].end - flat.ranges[at].start);
        let below = blocks.get(hi + 1).map(|block| block.lines.len());
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

    if lo > 0 && !blocks[..lo].iter().any(|block| block.item) {
        let extra = flat.ranges[lo - 1].end - flat.ranges[0].start;
        if used + extra <= height.saturating_sub(usize::from(hi + 1 < foot)) {
            lo = 0;
        }
    }
    if hi + 1 < foot && !blocks[hi + 1..].iter().any(|block| block.item) {
        let extra = flat.ranges[foot - 1].end - flat.ranges[hi + 1].start;
        if used + extra <= height.saturating_sub(usize::from(lo > 0)) {
            hi = foot - 1;
        }
    }

    let above = blocks[..lo].iter().filter(|block| block.item).count();
    let below = blocks[hi + 1..].iter().filter(|block| block.item).count();
    let mut view = Flattened::default();
    // A lone section header is not worth claiming "0 more".
    if above > 0 {
        view.push_indicator(more_line(true, above));
    }
    append_blocks(&mut view, &flat, lo, hi + 1);
    if below > 0 {
        view.push_indicator(more_line(false, below));
    }
    view
}

fn extend_flat(target: &mut Flattened, source: Flattened) {
    target.lines.extend(source.lines);
    target.rows.extend(source.rows);
    target.spaces.extend(source.spaces);
    target.agents.extend(source.agents);
    target.bands.extend(source.bands);
}

/// All panel lines that fit, plus an offset that is always zero: the returned
/// arrays are the viewport itself. Agents and the actions stay pinned; only
/// the workspaces and tabs between them scroll, with an indicator per edge.
fn panel_view(app: &App, width: u16, wide: bool, height: u16) -> PanelView {
    let blocks = panel_blocks(app, width, wide);
    let flat = flatten(&blocks);
    let height = height as usize;
    if height == 0 || flat.lines.len() <= height {
        let len = flat.lines.len();
        return PanelView { lines: flat.lines, rows: flat.rows, spaces: flat.spaces, agents: flat.agents, bands: flat.bands, offset: 0, len };
    }

    let foot = blocks.len().saturating_sub(2);
    let head = blocks
        .iter()
        .position(|block| block.lines.iter().any(|line| rendered(line).trim_start().starts_with("02 WORKSPACES ")))
        .unwrap_or(foot.min(blocks.len()));
    if head >= foot {
        let len = flat.lines.len();
        return PanelView { lines: flat.lines, rows: flat.rows, spaces: flat.spaces, agents: flat.agents, bands: flat.bands, offset: 0, len };
    }
    let head_lines = blocks[..head].iter().map(|block| block.lines.len()).sum::<usize>();
    let foot_lines = blocks[foot..].iter().map(|block| block.lines.len()).sum::<usize>();
    // Too many open agents to pin and still paint the actions: fall back to
    // clipping the whole panel rather than losing the footer off-screen.
    if head_lines + foot_lines >= height {
        let view = clip_blocks(&blocks, height, app.cursor);
        let len = view.lines.len();
        return PanelView { lines: view.lines, rows: view.rows, spaces: view.spaces, agents: view.agents, bands: view.bands, offset: 0, len };
    }
    let room = height.saturating_sub(head_lines + foot_lines);
    let mut view = flatten(&blocks[..head]);
    extend_flat(&mut view, clip_blocks(&blocks[head..foot], room, app.cursor));
    extend_flat(&mut view, flatten(&blocks[foot..]));
    let len = view.lines.len();
    PanelView { lines: view.lines, rows: view.rows, spaces: view.spaces, agents: view.agents, bands: view.bands, offset: 0, len }
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

/// The panel's three numbered sections. Each item owns exactly two rows and
/// one target; nothing is nested, and the active space and tab live apart.
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

    // Agents lead: they stay visible while the other two sections scroll.
    blocks.push(section_block("01", "AGENTS", app.agents().len(), width));
    for agent in app.agents() {
        let is_selected = |row: &super::app::Row| {
            row.address.as_deref().is_some_and(|address| {
                address == agent.address.as_str() || agent.parent.as_deref() == Some(address)
            })
        };
        let target = app.rows.iter().position(is_selected);
        let active = app.selected().is_some_and(is_selected);
        spaced_item(&mut blocks, PanelBlock::item(
            agent_line(agent, width, active),
            agent_detail(app, agent, width),
            target,
            None,
            Some(agent.address.clone()),
            active,
        ));
    }

    blocks.push(section_block("02", "WORKSPACES", app.model.spaces.len(), width));
    let home = crate::mods::home().display().to_string();
    for space in &app.model.spaces {
        let name = app.model.space_name(space, &home);
        let active = app.model.active.as_deref() == Some(space.id.as_str());
        spaced_item(&mut blocks, PanelBlock::item(
            space_line(&name, width, active),
            space_detail(app, space, width),
            None,
            Some(space.id.clone()),
            None,
            active,
        ));
    }

    blocks.push(section_block("03", "TABS", app.rows().len(), width));
    for (index, row) in app.rows().iter().enumerate() {
        let selected = index == app.cursor;
        spaced_item(&mut blocks, PanelBlock::item(
            row_line(row, width, index, selected),
            row_detail(app, row, width),
            Some(index),
            None,
            None,
            selected,
        ));
    }

    if let Some(terminal) = app.selected()
        && !terminal.preview.is_empty()
    {
        blocks.push(PanelBlock::chrome(task_line(&terminal.name, width)));
    }

    // The actions sit at the foot of the panel, not in a status bar.
    blocks.push(PanelBlock::chrome(Line::from("")));
    let actions = action_labels(wide).join("  ");
    blocks.push(PanelBlock::chrome(Line::from(Span::styled(actions, Style::default().fg(theme::MUTED)))));
    blocks
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

/// A numbered title, a hairline rule in `LINE` and a right-aligned count.
/// The rule belongs to the header, so a panel needs no divider row.
/// A section starts after one empty row, so its title leads a group instead
/// of continuing the previous item. The gap has no target and no band.
fn section_block(number: &str, title: &str, count: usize, width: u16) -> PanelBlock {
    let mut block = PanelBlock::chrome(Line::from(""));
    block.lines.push(section(number, title, count, width));
    block.rows.push(None);
    block.spaces.push(None);
    block.agents.push(None);
    block.bands.push(Band::None);
    block
}

fn section(number: &str, title: &str, count: usize, width: u16) -> Line<'static> {
    let width = width as usize;
    let count = count.to_string();
    let prefix = format!("{number} {title} ");
    let rule = width.saturating_sub(prefix.chars().count() + count.chars().count() + 1);
    Line::from(vec![
        Span::styled(number.to_string(), Style::default().fg(theme::ACCENT_BRIGHT)),
        Span::raw(" "),
        Span::styled(title.to_string(), Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled("─".repeat(rule), Style::default().fg(theme::LINE)),
        Span::raw(" "),
        Span::styled(count, Style::default().fg(theme::DIM)),
    ])
}

/// One workspace: a focus glyph, then its name. The repo branch, reported by
/// core for this space's first terminal, is the second row.
fn space_line(name: &str, width: u16, active: bool) -> Line<'static> {
    let (focus, color) = if active { ('▸', theme::ACCENT_BRIGHT) } else { ('○', theme::MUTED) };
    let mut style = Style::default().fg(theme::TEXT);
    if active {
        style = style.add_modifier(Modifier::BOLD);
    }
    Line::from(vec![
        Span::styled(focus.to_string(), Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(fit(name, width.saturating_sub(2) as usize), style),
    ])
}

/// Alt+N selects a tab by its visible position; the tenth is Alt+0.
fn tab_digit(index: usize) -> Option<char> {
    match index {
        0..=8 => char::from_digit((index + 1) as u32, 10),
        9 => Some('0'),
        _ => None,
    }
}

/// The first row of a tab block: its shortcut digit, the state glyph and the
/// name. Only the active tab's name is bold; the digit carries the accent.
fn row_line(row: &super::app::Row, width: u16, index: usize, active: bool) -> Line<'static> {
    let mut spans = match tab_digit(index) {
        Some(digit) => {
            let color = if active { theme::ACCENT_BRIGHT } else { theme::MUTED };
            vec![Span::styled(digit.to_string(), Style::default().fg(color)), Span::raw(" ")]
        }
        None => vec![Span::raw(" ")],
    };
    spans.push(Span::styled(row.state.glyph().to_string(), Style::default().fg(row.state.color())));
    spans.push(Span::raw(" "));
    let used = spans.iter().map(|span| span.content.chars().count()).sum::<usize>();
    let mut style = Style::default().fg(theme::TEXT);
    if active {
        style = style.add_modifier(Modifier::BOLD);
    }
    spans.push(Span::styled(fit(&row.name, width as usize - used.min(width as usize)), style));
    Line::from(spans)
}

/// The pane count of a document tab, when the workspace is available.
fn pane_count(app: &App, row: &super::app::Row) -> Option<usize> {
    let space = app.model.active_space()?;
    let tab = space.tabs.iter().find(|tab| tab.id == row.id)?;
    Some(tab.root.panes().len())
}

/// The second row of a tab block: `state · app · N panes`, with only the
/// fields core reported. The state word keeps its glyph's colour.
fn row_detail(app: &App, row: &super::app::Row, width: u16) -> Line<'static> {
    let terminal = row.address.as_ref().and_then(|addr| app.terminals.iter().find(|t| t.address == *addr));
    let app_name = terminal.and_then(|t| t.app.as_deref()).filter(|name| !name.is_empty());
    let mut parts = vec![row.state.word().to_string()];
    if let Some(name) = app_name {
        parts.push(name.to_string());
    }
    if let Some(panes) = pane_count(app, row) {
        parts.push(format!("{panes} {}", if panes == 1 { "pane" } else { "panes" }));
    }
    state_detail_line(row.state, &parts, 0, width)
}

/// A space's core-reported branch, looked up through its first terminal, else
/// its folder path. The path's tail survives when it must truncate.
fn space_detail(app: &App, space: &crate::model::Space, width: u16) -> Line<'static> {
    let first = space
        .tabs
        .iter()
        .flat_map(|tab| tab.root.terminals())
        .find_map(|node| node.terminal().map(str::to_owned));
    let text = first
        .as_deref()
        .and_then(|address| app.model.folder_info.get(address))
        .and_then(|info| info.branch.as_deref())
        .filter(|branch| !branch.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| space.folder.clone());
    detail_line(&fit_path(&text, width as usize), 0, width)
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

/// One detected agent's name row: state glyph in its colour, then the name
/// in TEXT. The word is on the detail row, so state is never colour alone.
fn agent_line(agent: &super::app::Terminal, width: u16, active: bool) -> Line<'static> {
    let name = Style::default().fg(theme::TEXT);
    let name = if active { name.add_modifier(Modifier::BOLD) } else { name };
    Line::from(vec![
        Span::styled(agent.state.glyph().to_string(), Style::default().fg(agent.state.color())),
        Span::raw(" "),
        Span::styled(fit(&agent.name, width.saturating_sub(2) as usize), name),
    ])
}

/// Agent detail row: `state · app · elapsed`, with no invented fields.
fn agent_detail(app: &App, agent: &super::app::Terminal, width: u16) -> Line<'static> {
    let app_name = agent.app.as_deref().filter(|name| !name.is_empty());
    let mut parts = vec![agent.state.word().to_string()];
    if let Some(name) = app_name {
        parts.push(name.to_string());
    }
    if let Some(elapsed) = agent_elapsed(app, &agent.address) {
        parts.push(elapsed);
    }
    state_detail_line(agent.state, &parts, 0, width)
}

/// The elapsed time since a core agent's last state change. A `since` the
/// app never received means the field is omitted, not guessed.
fn agent_elapsed(app: &App, address: &str) -> Option<String> {
    let since = app.model.agents.get(address).map(|agent| agent.since).unwrap_or(0);
    elapsed_label(since, crate::model::now_ms())
}

fn elapsed_label(since: u64, now: u64) -> Option<String> {
    if since == 0 {
        return None;
    }
    let seconds = now.saturating_sub(since) / 1000;
    Some(if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h", seconds / 3600)
    })
}

/// One detail row, indented `indent` cells and truncated with `…`.
fn detail_line(text: &str, indent: usize, width: u16) -> Line<'static> {
    let available = (width as usize).saturating_sub(indent);
    Line::from(Span::styled(
        format!("{}{}", " ".repeat(indent), fit(text, available)),
        Style::default().fg(theme::BODY),
    ))
}

/// A state word leads the detail row in its own token, paired with the badge
/// above it. Everything after it is ordinary BODY metadata.
fn state_detail_line(state: State, parts: &[String], indent: usize, width: u16) -> Line<'static> {
    let available = (width as usize).saturating_sub(indent);
    let fitted = fit(&parts.join(" · "), available);
    let mut spans = vec![Span::raw(" ".repeat(indent))];
    match fitted.strip_prefix(state.word()) {
        Some(rest) => {
            spans.push(Span::styled(state.word().to_string(), Style::default().fg(state.color())));
            spans.push(Span::styled(rest.to_string(), Style::default().fg(theme::BODY)));
        }
        None => spans.push(Span::styled(fitted, Style::default().fg(theme::BODY))),
    }
    Line::from(spans)
}

fn task_line(name: &str, width: u16) -> Line<'static> {
    let label = format!("─ {name} ");
    let line = format!("{label}{}", "─".repeat((width as usize).saturating_sub(label.chars().count())));
    Line::from(Span::styled(line, Style::default().fg(theme::LINE)))
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

    /// Two spaces: `alpha` is active, `beta` holds one agent with the given state.
    fn attention_app(state: &str) -> App {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [
                { "id": "s1", "name": "alpha", "folder": "/a", "tabs": [
                    { "id": "tb1", "name": "active-tab", "root": { "id": "tb1", "type": "row", "children": [] } }
                ] },
                { "id": "s2", "name": "beta", "folder": "/b", "tabs": [
                    { "id": "tb2", "name": "beta-tab", "root": { "id": "tb2", "type": "row", "children": [
                        { "id": "p1", "type": "block", "content": { "terminal": "t1" } }
                    ] } }
                ] }
            ] }
        }));
        app.set_terminals(&json!({
            "hidden_finished": 0, "machines": [],
            "terminals": [{ "address": "t1", "name": "watcher", "status": { "state": state } }]
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

    /// The surface the panel paints before any item fill.
    fn panel_bg(wide: bool) -> ratatui::style::Color {
        if wide { theme::PANEL } else { theme::ELEVATED }
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
        assert!(rail.iter().any(|row| row.trim_start().starts_with("02 WORKSPACES ")), "rail panels start after the gutter: {rail:?}");
        // The rule sits where the rail ends; the terminal area is to its right.
        assert!(rows.iter().any(|row| row.chars().nth(RAIL_WIDTH as usize) == Some('│')), "no rule after the rail");
    }

    #[test]
    fn a_narrow_screen_with_the_bar_closed_is_the_terminal_below_it() {
        let rows = screen(&app(false), 40, 22);
        assert!(rows[0].contains("pi 2"), "band row: {:?}", rows[0]);
        assert!(rows[1].contains("switch"), "the toggle is centred in the band: {:?}", rows[1]);
        assert!(!rows.iter().any(|row| row.trim_start().starts_with("02 WORKSPACES ")), "the panel is closed: {rows:?}");
        assert!(!rows.iter().any(|row| row.contains("new space")), "no actions while closed: {rows:?}");
    }

    #[test]
    fn opening_the_panel_on_a_narrow_screen_covers_it_with_sections() {
        let rows = screen(&app(true), 40, 22);
        assert!(rows[1].trim_end().ends_with('x'), "the toggle turns into x: {:?}", rows[1]);
        assert!(rows.iter().any(|row| row.trim_start().starts_with("1 ▲ pi 2")), "the tabs are reachable: {rows:?}");
        assert!(rows.iter().any(|row| row.trim_start().starts_with("01 AGENTS ")), "{rows:?}");
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
        let flat = flatten(&panel_blocks(&app(true), 38, false));
        let rows = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let start = rows.iter().position(|row| row.trim_start().starts_with("01 AGENTS ")).expect("the agents section");
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
        let start = rows.iter().position(|row| row.trim_start().starts_with("01 AGENTS ")).expect("the agents section");
        let space_header = rows.iter().position(|row| row.trim_start().starts_with("02 WORKSPACES ")).expect("the workspaces header");
        let section = &rows[start + 1..space_header];
        assert!(!section.iter().any(|row| row.contains("bash")), "no plain shell: {section:?}");
        let agent = section.iter().position(|row| row.contains("◐ pi")).expect("the real agent");
        assert!(section[agent + 1].contains("working"), "its state is on the detail row: {section:?}");
        let targets = agent_targets(&app);
        assert_eq!(targets, vec!["t2".to_string()], "Alt+Up/Down skips the shell");
        assert_eq!(app.rows()[0].address.as_deref(), Some("t2"));

        let mut shell_only = App::new("default", "local");
        shell_only.set_terminals(&json!({
            "hidden_finished": 0, "machines": [],
            "terminals": [{ "address": "t1", "name": "bash", "state": "live" }]
        }));
        shell_only.toggle_panel();
        let rows = screen(&shell_only, 40, 20);
        let header = rows.iter().find(|row| row.trim_start().starts_with("01 AGENTS ")).expect("the agents panel is always shown");
        assert!(header.trim_end().ends_with('0'), "a shell is not an agent: {header:?}");
        let tabs = rows.iter().position(|row| row.trim_start().starts_with("03 TABS ")).expect("the tabs header");
        let header_at = rows.iter().position(|row| row.trim_start().starts_with("01 AGENTS ")).expect("the agents header");
        assert!(!rows[header_at + 1..tabs].iter().any(|row| row.contains("bash")), "no agent row: {rows:?}");
    }

    #[test]
    fn tabs_live_in_their_own_panel_not_nested_under_a_space() {
        let app = spaces_app();
        let panel = zones(Rect::new(0, 0, 40, 24), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let (lines, rows, spaces) = panel_content(&app, inner.width, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();

        let spaces_label = texts.iter().position(|line| line.trim_start().starts_with("02 WORKSPACES ")).expect("the workspaces panel");
        let tabs_label = texts.iter().position(|line| line.trim_start().starts_with("03 TABS ")).expect("the tabs panel");
        let agents_label = texts.iter().position(|line| line.trim_start().starts_with("01 AGENTS ")).expect("the agents panel");
        let alpha = texts.iter().position(|line| line.contains("alpha")).expect("the active space");
        let one = texts.iter().position(|line| line.contains("one")).expect("its tab");
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the inactive space");
        assert!(agents_label < spaces_label, "agents lead: {texts:?}");
        assert!(spaces_label < alpha && alpha < beta && beta < tabs_label, "both spaces precede the tabs panel: {texts:?}");
        assert!(tabs_label < one, "the active tab is in its own panel: {texts:?}");
        assert!(!texts.iter().any(|line| line.contains("two") || line.contains("three")), "the other space's tabs stay out: {texts:?}");

        assert_eq!(rows[one], Some(0), "the tab's first row maps to its index");
        assert_eq!(rows[one + 1], Some(0), "the tab's second row maps to its index");
        assert_eq!(spaces[alpha], Some("s1".to_string()));
        assert_eq!(spaces[alpha + 1], Some("s1".to_string()));
        assert_eq!(rows[beta], None, "a space block is not a tab");

        let flat = flatten(&panel_blocks(&app, inner.width, false));
        assert!(matches!(flat.bands[one], Band::Item { selected: true }), "the active tab is its own block");
        assert_eq!(flat.bands[one], flat.bands[one + 1], "one band across both rows");

        let view = panel_view(&app, inner.width, false, inner.height);
        let target = view.spaces.iter().position(|space| space.as_deref() == Some("s2")).expect("the beta row");
        let y = inner.y + (target - view.offset) as u16;
        assert_eq!(panel_space_at(&app, panel, inner.x + 2, y, false), Some("s2".to_string()));
    }

    #[test]
    fn selecting_a_space_switches_the_tabs_panel() {
        let mut app = spaces_app();
        app.model.active = Some("s2".to_string());
        app.rebuild_rows();
        let panel = zones(Rect::new(0, 0, 40, 24), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let (lines, rows, _) = panel_content(&app, inner.width, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        assert!(!texts.iter().any(|line| line.contains("one")), "alpha's tab left the panel: {texts:?}");
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the selected space");
        let tabs_label = texts.iter().position(|line| line.trim_start().starts_with("03 TABS ")).expect("the tabs panel");
        let two = texts.iter().position(|line| line.contains("two")).expect("its first tab");
        let three = texts.iter().position(|line| line.contains("three")).expect("its second tab");
        assert!(beta < tabs_label && tabs_label < two && two < three, "beta's tabs fill the tabs panel: {texts:?}");
        assert_eq!(rows[two], Some(0), "first tab maps to row 0");
        assert_eq!(rows[three], Some(1), "second tab maps to row 1");
    }

    #[test]
    fn there_are_no_legacy_section_labels_or_divider_rows() {
        let app = spaces_app();
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        for label in ["02 WORKSPACES ", "03 TABS ", "01 AGENTS "] {
            let at = texts
                .iter()
                .position(|line| line.trim_start().starts_with(label))
                .unwrap_or_else(|| panic!("the {label} header: {texts:?}"));
            let line = &lines[at];
            assert_eq!(rendered(line).chars().count(), 38, "the header fills the row: {:?}", rendered(line));
            let rule = line
                .spans
                .iter()
                .find(|span| !span.content.is_empty() && span.content.chars().all(|c| c == '─'))
                .expect("one rule on the header");
            assert_eq!(rule.style.fg, Some(theme::LINE));
        }
        assert!(!texts.iter().any(|text| !text.is_empty() && text.chars().all(|c| c == '─')), "no separate divider row");
        assert!(!texts.iter().any(|line| line.contains('▾') || line.trim() == "spaces" || line.trim() == "agents"), "no old label: {texts:?}");
    }

    #[test]
    fn tab_counts_live_in_headers_not_in_space_details() {
        let app = spaces_app();
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let alpha = texts.iter().position(|line| line.contains("alpha")).expect("the active space");
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the inactive space");
        assert!(texts[alpha + 1].contains("/home/dsasco/alpha"), "the branch or folder only: {:?}", texts[alpha + 1]);
        assert!(texts[beta + 1].contains("/home/dsasco/beta"), "{:?}", texts[beta + 1]);
        assert!(!texts.iter().any(|line| line.contains("1 tab") || line.contains("2 tabs")), "counts are only in headers: {texts:?}");
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
        let rows = screen(&app, 40, 20);
        let y = rows.iter().position(|row| row.trim_start().starts_with("new space  new tab")).expect("the action row") as u16;
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
        let name = rows.iter().position(|row| row.contains("▸ work")).expect("space name");
        let detail = &rows[name + 1];
        assert!(detail.contains('…'), "one ellipsis, not three dots: {detail:?}");
        assert!(!detail.contains("..."), "no ASCII triple dot: {detail:?}");
        assert!(detail.trim_end().ends_with("name"), "path tail preserved: {detail:?}");
        assert!(detail.trim_start().starts_with("/home/"), "path head kept before the ellipsis: {detail:?}");
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

    /// A helper terminal never becomes an indented sibling row: the tabs
    /// panel lists tabs, and the agent panel lists the helper itself.
    #[test]
    fn helper_terminals_have_no_nested_block() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "parent-tab", "status": { "state": "idle" } },
                { "address": "t2", "name": "child-tab", "state": "live", "parent": "t1" }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        let rows = screen(&app, 40, 20);
        assert!(rows.iter().any(|row| row.contains("parent-tab")), "the tab is listed");
        assert!(!rows.iter().any(|row| row.contains("child-tab")), "no nested helper block: {rows:?}");
        let z = zones(Rect::new(0, 0, 40, 20), true);
        let inner = panel_inner(z.panel.expect("sheet"));
        let flat = flatten(&panel_blocks(&app, inner.width, false));
        assert!(flat.bands.iter().all(|band| matches!(band, Band::None | Band::Item { .. })), "only flat blocks");
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

    /// T8: only the active block breaks the sheet surface, and never with a
    /// hard `SURFACE` band.
    #[test]
    fn only_the_active_item_takes_a_wash_fill() {
        let app = app(true);
        let z = zones(Rect::new(0, 0, 40, 22), true);
        let inner = panel_inner(z.panel.expect("the phone sheet"));
        let bg = backgrounds(&app, 40, 22);
        let view = panel_view(&app, inner.width, false, inner.height);
        assert!(view.bands.iter().any(|band| matches!(band, Band::Item { .. })), "there are items");
        let mut washed = 0;
        let mut plain = 0;
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                let color = bg[y as usize][x as usize];
                assert!(
                    color == theme::ELEVATED || color == theme::WASH || color == theme::RUST_LINE,
                    "background {color:?} at ({x},{y})"
                );
                washed += usize::from(color == theme::WASH);
                plain += usize::from(color == theme::ELEVATED);
            }
        }
        assert!(washed > 0, "the active block is washed");
        assert!(plain > 0, "inactive rows keep the sheet surface");
        // Chrome sits on the sheet too.
        let header = view.bands.iter().position(|band| *band == Band::None).expect("chrome");
        let y = (inner.y + header as u16) as usize;
        assert_eq!(bg[y][inner.x as usize], theme::ELEVATED);
    }

    /// T9: the active marker is a one-cell rust edge plus a warm wash across
    /// the rest of both rows, and never an accent rail.
    #[test]
    fn the_selected_item_has_a_rust_edge_and_a_wash_fill() {
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
            .filter_map(|(line, band)| matches!(band, Band::Item { selected: true }).then_some(line))
            .collect();
        assert_eq!(selected.len(), 4, "the selected agent and tab each own two rows: {selected:?}");
        for line in selected {
            assert_eq!(view.rows[line], Some(0), "the selected item is the current tab");
            let y = (inner.y + line as u16) as usize;
            assert_eq!(bg[y][inner.x as usize], theme::RUST_LINE, "edge at the panel's left inset");
            for x in inner.x + 1..inner.x + inner.width {
                assert_eq!(bg[y][x as usize], theme::WASH, "warm wash behind the rest of the item");
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
        app.cursor = 5;
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
        let tabs = rows.iter().position(|row| row.trim_start().starts_with("03 TABS ")).expect("the tabs panel");
        let tab = rows[tabs + 1..].iter().position(|row| row.contains("◐ claude")).expect("tab name row") + tabs + 1;
        let detail = &rows[tab + 1];
        assert!(detail.contains("working"), "{detail:?}");
        assert!(detail.contains("pi"), "{detail:?}");
        assert!(detail.contains("2 panes"), "pane count from the layout: {detail:?}");
        let agents = rows.iter().position(|row| row.trim_start().starts_with("01 AGENTS ")).expect("agents section");
        let agent = rows[agents + 1..].iter().position(|row| row.contains("◐ claude")).expect("agent row") + agents + 1;
        assert!(rows[agent + 1].contains("working · pi"), "agent detail: {:?}", rows[agent + 1]);
    }

    // ---- S10 aesthetic pass ----

    #[test]
    fn the_desktop_rail_is_34_columns_with_one_gutter_and_one_rule() {
        assert_eq!(RAIL_WIDTH, 34);
        let rows = screen(&app(true), 100, 24);
        assert_eq!(rows[1].chars().next(), Some(' '), "one gutter cell");
        assert_ne!(rows[1].chars().nth(1), Some(' '), "content starts at column 1");
        for y in [1, 12, 23] {
            assert_eq!(rows[y].chars().nth(RAIL_WIDTH as usize), Some('│'), "the single rule");
            assert_ne!(rows[y].chars().nth(RAIL_WIDTH as usize + 1), Some('│'), "only one rule");
        }
    }

    #[test]
    fn one_blank_row_separates_item_blocks_without_a_target_or_band() {
        let app = spaces_app();
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let alpha = texts.iter().position(|line| line.contains("alpha")).expect("the active space");
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the next space");
        let spaces = texts.iter().position(|line| line.trim_start().starts_with("02 WORKSPACES ")).expect("the workspaces panel");
        let tabs = texts.iter().position(|line| line.trim_start().starts_with("03 TABS ")).expect("the tabs panel");
        let one = texts.iter().position(|line| line.contains("one")).expect("its tab");
        let agent = texts.iter().position(|line| line.contains("◐ pi")).expect("the agent");
        assert_eq!(beta, alpha + 3, "one blank row between two spaces: {texts:?}");
        assert_eq!(spaces, agent + 3, "a gap precedes the workspaces panel: {texts:?}");
        assert_eq!(tabs, beta + 3, "a gap precedes the next panel header: {texts:?}");
        assert_eq!(one, tabs + 1, "the first tab follows its header: {texts:?}");
        let gap = alpha + 2;
        assert!(texts[gap].trim().is_empty(), "the separator is blank: {:?}", texts[gap]);
        assert_eq!(flat.rows[gap], None, "no hit target on a blank row");
        assert_eq!(flat.spaces[gap], None, "no space target on a blank row");
        assert_eq!(flat.bands[gap], Band::None, "no band on a blank row");
        // The two rows of an item stay contiguous.
        assert!(texts[alpha + 1].contains('/'), "the space detail follows its name");
        assert!(!texts[one + 1].trim().is_empty(), "the tab detail follows its name: {texts:?}");

        let z = zones(Rect::new(0, 0, 40, 24), true);
        let panel = z.panel.expect("the sheet");
        let inner = panel_inner(panel);
        let bg = backgrounds(&app, 40, 24);
        let y = inner.y + gap as u16;
        assert_eq!(panel_row_at(&app, panel, inner.x + 2, y, false), None, "no tab hit");
        assert_eq!(panel_space_at(&app, panel, inner.x + 2, y, false), None, "no space hit");
        for x in inner.x..inner.x + inner.width {
            assert_eq!(bg[y as usize][x as usize], panel_bg(false), "clean sheet on the blank row");
        }
    }

    #[test]
    fn every_open_agent_appears_regardless_of_the_active_space_or_tab() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [
                { "id": "s1", "name": "alpha", "folder": "/a", "tabs": [
                    { "id": "tb1", "name": "active-tab", "root": { "id": "tb1", "type": "row", "children": [
                        { "id": "p1", "type": "block", "content": { "terminal": "t1" } }
                    ] } }
                ] },
                { "id": "s2", "name": "beta", "folder": "/b", "tabs": [
                    { "id": "tb2", "name": "inactive-tab", "root": { "id": "tb2", "type": "row", "children": [
                        { "id": "p2", "type": "block", "content": { "terminal": "t2" } }
                    ] } }
                ] }
            ] }
        }));
        app.set_terminals(&json!({
            "hidden_finished": 0, "machines": [],
            "terminals": [
                { "address": "t1", "name": "pi-one", "status": { "state": "working" } },
                { "address": "t2", "name": "pi-two", "status": { "state": "blocked" } }
            ]
        }));
        app.toggle_panel();
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let agents = texts.iter().position(|line| line.trim_start().starts_with("01 AGENTS ")).expect("the agents panel");
        let one = texts.iter().position(|line| line.contains("pi-one")).expect("the active space's agent");
        let two = texts.iter().position(|line| line.contains("pi-two")).expect("the inactive space's agent");
        assert!(agents < one && agents < two, "both agents are in the agents panel: {texts:?}");
        assert!(two < one, "the blocked agent is first, whatever its space: {texts:?}");
        assert!(texts[agents].trim_end().ends_with('2'), "both agents are counted: {:?}", texts[agents]);
        assert!(!texts[..agents].iter().any(|line| line.contains("pi-two")), "no agent row outside its panel: {texts:?}");
    }

    /// Four spaces, two agents: one in the active space, one in another.
    fn many_spaces_app() -> App {
        let mut app = App::new("default", "local");
        let mut spaces = Vec::new();
        for (id, name) in [("s1", "alpha"), ("s2", "beta"), ("s3", "gamma"), ("s4", "delta")] {
            spaces.push(json!({
                "id": id, "name": name, "folder": "/work", "tabs": [
                    { "id": format!("tb-{id}"), "name": format!("{name}-tab"), "root": { "id": "root", "type": "row", "children": [
                        { "id": "pane", "type": "block", "content": { "terminal": format!("t-{id}") } }
                    ] } }
                ]
            }));
        }
        app.set_workspace(&json!({ "version": 7, "value": { "spaces": spaces } }));
        app.set_terminals(&json!({
            "hidden_finished": 0, "machines": [],
            "terminals": [
                { "address": "t-s1", "name": "pi-alpha", "status": { "state": "working" } },
                { "address": "t-s2", "name": "pi-beta", "status": { "state": "blocked" } },
                { "address": "t-s3", "name": "shell", "state": "live" },
                { "address": "t-s4", "name": "shell", "state": "live" }
            ]
        }));
        app.toggle_panel();
        app
    }

    #[test]
    fn agents_come_before_spaces_and_tabs() {
        let app = many_spaces_app();
        let flat = flatten(&panel_blocks(&app, 30, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let at = |label: &str| texts.iter().position(|line| line.trim_start().starts_with(label)).expect("a section header");
        assert!(at("01 AGENTS ") < at("02 WORKSPACES "), "agents lead: {texts:?}");
        assert!(at("02 WORKSPACES ") < at("03 TABS "), "tabs close the panel: {texts:?}");
    }

    #[test]
    fn the_agents_section_stays_visible_when_the_panel_scrolls() {
        let app = many_spaces_app();
        let panel = zones(Rect::new(0, 0, 40, 20), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let view = panel_view(&app, inner.width, false, inner.height);
        let texts = view.lines.iter().map(rendered).collect::<Vec<_>>();
        assert!(texts.iter().any(|line| line.trim_start().starts_with("01 AGENTS ")), "the header is pinned: {texts:?}");
        assert!(texts.iter().any(|line| line.contains("pi-alpha")), "the first agent is pinned: {texts:?}");
        assert!(texts.iter().any(|line| line.contains("pi-beta")), "the second agent is pinned: {texts:?}");
        assert_eq!(view.agents.iter().flatten().count(), 4, "both agent rows carry their address");
    }

    #[test]
    fn an_agent_row_hits_its_terminal_address() {
        let app = many_spaces_app();
        let panel = zones(Rect::new(0, 0, 40, 20), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let view = panel_view(&app, inner.width, false, inner.height);
        let texts = view.lines.iter().map(rendered).collect::<Vec<_>>();
        let row = texts.iter().position(|line| line.contains("pi-beta")).expect("the other space's agent");
        let y = inner.y + row as u16;
        assert_eq!(panel_agent_at(&app, panel, inner.x + 2, y, false).as_deref(), Some("t-s2"));
        assert_eq!(panel_space_at(&app, panel, inner.x + 2, y, false), None, "not a space target");
    }

    #[test]
    fn activating_an_agent_selects_its_space_and_tab() {
        let mut app = many_spaces_app();
        assert_eq!(app.model.active.as_deref(), Some("s1"), "alpha starts active");
        assert!(app.activate_agent("t-s2"), "an open agent is reachable");
        assert_eq!(app.model.active.as_deref(), Some("s2"), "its space becomes active");
        assert_eq!(app.selected().map(|row| row.name.as_str()), Some("beta-tab"), "its tab becomes selected");
        assert!(!app.activate_agent("missing"), "an unknown address is not activated");
    }

    #[test]
    fn the_selected_agent_uses_the_active_band_and_a_bold_name() {
        let app = app(true);
        let agent_name = app.selected().expect("a selected tab").name.clone();
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let selected = texts.iter().position(|line| line.contains(&agent_name)).expect("the selected tab's agent");
        let other = texts.iter().position(|line| line.contains("pi 3") && !line.contains(&agent_name)).expect("another agent");
        assert!(matches!(flat.bands[selected], Band::Item { selected: true }), "the active agent is highlighted: {texts:?}");
        assert!(matches!(flat.bands[other], Band::Item { selected: false }), "the rest stay plain: {texts:?}");
        let name = flat.lines[selected]
            .spans
            .iter()
            .find(|span| span.content.as_ref().contains(agent_name.as_str()))
            .unwrap_or_else(|| panic!("the agent name: {:?}", flat.lines[selected]));
        assert!(name.style.add_modifier.contains(Modifier::BOLD), "the active name is bold");
    }

    #[test]
    fn a_blank_row_precedes_every_section_header() {
        let app = spaces_app();
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let headers: Vec<usize> = ["02 WORKSPACES ", "03 TABS ", "01 AGENTS "]
            .iter()
            .map(|label| texts.iter().position(|line| line.trim_start().starts_with(label)).expect("a section header"))
            .collect();
        for header in headers {
            let gap = header - 1;
            assert!(texts[gap].trim().is_empty(), "a blank row before the header: {texts:?}");
            assert_eq!(flat.rows[gap], None, "no hit target on the gap");
            assert_eq!(flat.spaces[gap], None, "no space target on the gap");
            assert_eq!(flat.bands[gap], Band::None, "no band on the gap");
        }
        let beta = texts.iter().position(|line| line.contains("beta")).expect("the last space");
        let tabs = texts.iter().position(|line| line.trim_start().starts_with("03 TABS ")).expect("the tabs header");
        assert_eq!(tabs, beta + 3, "detail, gap, then the next section: {texts:?}");
    }

    #[test]
    fn the_section_title_carries_more_weight_than_its_number() {
        let line = section("01", "WORKSPACES", 2, 38);
        let spans = &line.spans;
        assert_eq!(spans[0].style.fg, Some(theme::ACCENT_BRIGHT), "the numeral keeps the accent");
        assert_eq!(spans[2].style.fg, Some(theme::TEXT), "the title is body text, not muted");
        assert!(spans[2].style.add_modifier.contains(Modifier::BOLD), "the title names the section");
        assert_eq!(spans[6].style.fg, Some(theme::DIM), "the count recedes");
    }

    #[test]
    fn the_rail_rule_and_section_hairlines_use_line() {
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("test terminal");
        terminal.draw(|frame| render(&app(true), frame)).expect("draw");
        let buffer = terminal.backend().buffer();
        for y in 0..24 {
            assert_eq!(buffer[(RAIL_WIDTH, y)].fg, theme::LINE, "rail rule on row {y}");
        }

        let app = spaces_app();
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let section = texts
            .iter()
            .position(|line| line.trim_start().starts_with("02 WORKSPACES "))
            .expect("the section label");
        let rule = flat.lines[section]
            .spans
            .iter()
            .find(|span| !span.content.is_empty() && span.content.chars().all(|c| c == '─'))
            .expect("a header rule");
        assert_eq!(rule.style.fg, Some(theme::LINE), "section rule");
        assert_eq!(flat.rows[section], None, "no hit target on a label");
        assert_eq!(flat.spaces[section], None);
        assert_eq!(flat.bands[section], Band::None, "no band on a label");
    }

    #[test]
    fn names_are_bold_only_when_active_and_details_use_body() {
        let app = spaces_app();
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let name_style = |name: &str| {
            let at = texts.iter().position(|line| line.contains(name)).expect(name);
            flat.lines[at]
                .spans
                .iter()
                .find(|span| span.content.as_ref() == name)
                .unwrap_or_else(|| panic!("a span for {name}: {:?}", flat.lines[at]))
                .style
        };
        let bold = |name: &str| name_style(name).add_modifier.contains(Modifier::BOLD);
        for name in ["alpha", "beta", "one", "pi"] {
            assert_eq!(name_style(name).fg, Some(theme::TEXT), "{name} is TEXT");
        }
        assert!(bold("alpha"), "the active space is bold");
        assert!(!bold("beta"), "the inactive space is not bold");
        assert!(bold("one"), "the selected tab is bold");
        assert!(!bold("pi"), "an agent is not the active item");

        let detail_fg = |name: &str| {
            let at = texts.iter().position(|line| line.contains(name)).expect(name);
            flat.lines[at + 1]
                .spans
                .iter()
                .find_map(|span| span.style.fg)
                .expect("a coloured detail")
        };
        assert_eq!(detail_fg("alpha"), theme::BODY, "space metadata is BODY");
        assert_eq!(detail_fg("beta"), theme::BODY, "space metadata is BODY");
    }

    #[test]
    fn items_show_a_badge_then_the_name_on_row_one() {
        let app = spaces_app();
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let badge = |name: &str, glyph: &str| {
            let at = texts.iter().position(|line| line.contains(name)).expect(name);
            let spans = &flat.lines[at].spans;
            let badge_at = spans.iter().position(|span| span.content.as_ref() == glyph).expect(glyph);
            let name_at = spans.iter().position(|span| span.content.as_ref() == name).expect(name);
            assert_eq!(name_at, badge_at + 2, "badge, space, name: {:?}", flat.lines[at]);
            assert!(spans[badge_at].style.fg.is_some(), "the badge is coloured");
            assert_eq!(spans[name_at].style.fg, Some(theme::TEXT));
        };
        badge("alpha", "▸");
        badge("beta", "○");
        badge("one", "·");
        badge("pi", "◐");
        for line in &texts {
            assert!(!line.contains("1 tab  ") && !line.ends_with(" idle"), "no state word on row one: {line:?}");
        }
    }

    #[test]
    fn the_phone_sheet_and_band_are_elevated_and_the_rail_is_panel() {
        let sheet = spaces_app();
        let phone = backgrounds(&sheet, 40, 24);
        assert_eq!(phone[0][0], theme::ELEVATED, "the header band is ELEVATED");
        let z = zones(Rect::new(0, 0, 40, 24), true);
        let inner = panel_inner(z.panel.expect("the sheet"));
        let flat = flatten(&panel_blocks(&sheet, inner.width, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let section = texts.iter().position(|line| line.trim_start().starts_with("02 WORKSPACES ")).expect("a label");
        let y = (inner.y + section as u16) as usize;
        assert_eq!(phone[y][inner.x as usize], theme::ELEVATED, "the sheet is ELEVATED");

        let rail = backgrounds(&app(true), 100, 24);
        let inner = panel_inner(zones(Rect::new(0, 0, 100, 24), true).panel.expect("the rail"));
        let y = (inner.y + section as u16) as usize;
        assert_eq!(rail[y][inner.x as usize], theme::PANEL, "the rail is PANEL");
    }

    #[test]
    fn the_renderer_never_paints_a_surface_band() {
        let legacy = concat!("theme", "::SURFACE");
        assert!(!include_str!("view.rs").contains(legacy), "no SURFACE band in the renderer");
    }

    #[test]
    fn state_dots_keep_their_token_and_the_word_stays_with_them() {
        let app = app(true);
        let flat = flatten(&panel_blocks(&app, 38, false));
        let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
        let state_dot = |at: usize| {
            flat.lines[at]
                .spans
                .iter()
                .find(|span| matches!(span.content.as_ref(), "○" | "◐" | "▲"))
                .and_then(|span| span.style.fg)
                .expect("a state dot")
        };
        for (name, glyph, color, word) in [
            ("pi 1", "○", State::Idle.color(), "idle"),
            ("pi 2", "▲", State::NeedsYou.color(), "needs you"),
            ("pi 3", "◐", State::Working.color(), "working"),
        ] {
            let at = texts.iter().position(|line| line.contains(name)).expect(name);
            assert!(texts[at].contains(glyph), "{name} keeps its glyph");
            assert_eq!(state_dot(at), color, "{name} badge keeps its state token");
            assert!(texts[at + 1].contains(word), "{name} detail keeps its word");
            let word_fg = flat.lines[at + 1]
                .spans
                .iter()
                .find(|span| span.content.as_ref() == word)
                .and_then(|span| span.style.fg)
                .expect("a coloured state word");
            assert_eq!(word_fg, color, "{name} word keeps its state token");
        }
    }

    #[test]
    fn the_active_space_has_a_rust_edge_and_a_wash_across_both_rows() {
        let app = spaces_app();
        let z = zones(Rect::new(0, 0, 40, 24), true);
        let inner = panel_inner(z.panel.expect("the sheet"));
        let bg = backgrounds(&app, 40, 24);
        let view = panel_view(&app, inner.width, false, inner.height);
        let texts = view.lines.iter().map(rendered).collect::<Vec<_>>();
        let alpha = texts.iter().position(|line| line.contains("alpha")).expect("the active space");
        for line in [alpha, alpha + 1] {
            assert!(matches!(view.bands[line], Band::Item { selected: true }));
            let y = (inner.y + line as u16) as usize;
            assert_eq!(bg[y][inner.x as usize], theme::RUST_LINE, "rust edge on row {line}");
            for x in inner.x + 1..inner.x + inner.width {
                assert_eq!(bg[y][x as usize], theme::WASH, "warm wash behind the space");
            }
        }
    }

    #[test]
    fn inactive_spaces_keep_a_muted_focus_glyph_while_agents_keep_attention() {
        let fg = |app: &App, name: &str, glyph: &str| {
            let flat = flatten(&panel_blocks(app, 38, false));
            let texts = flat.lines.iter().map(rendered).collect::<Vec<_>>();
            let at = texts.iter().position(|line| line.contains(name)).expect(name);
            flat.lines[at]
                .spans
                .iter()
                .find(|span| span.content.as_ref() == glyph)
                .and_then(|span| span.style.fg)
                .unwrap_or_else(|| panic!("a {glyph} span on {name}: {:?}", flat.lines[at]))
        };
        for (state, color) in [
            ("blocked", theme::RED),
            ("working", theme::YELLOW),
            ("idle", theme::GREEN),
        ] {
            let app = attention_app(state);
            assert_eq!(fg(&app, "alpha", "▸"), theme::ACCENT_BRIGHT);
            assert_eq!(fg(&app, "beta", "○"), theme::MUTED, "inactive focus is not an attention dot");
            assert_eq!(fg(&app, "watcher", match state { "blocked" => "▲", "working" => "◐", _ => "○" }), color);
        }
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

    // ---- T9: three panels, no accent rail ----

    #[test]
    fn three_numbered_panels_carry_counts_on_the_rule_row() {
        let app = spaces_app();
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let header = |number: &str, title: &str| {
            let prefix = format!("{number} {title} ");
            texts
                .iter()
                .position(|line| line.starts_with(prefix.as_str()))
                .unwrap_or_else(|| panic!("the {number} {title} header: {texts:?}"))
        };
        let agents = header("01", "AGENTS");
        let workspaces = header("02", "WORKSPACES");
        let tabs = header("03", "TABS");
        assert!(agents < workspaces && workspaces < tabs, "panel order: {texts:?}");
        for (at, number, title, count) in [
            (agents, "01", "AGENTS", "1"),
            (workspaces, "02", "WORKSPACES", "2"),
            (tabs, "03", "TABS", "1"),
        ] {
            let line = &lines[at];
            assert_eq!(rendered(line).chars().count(), 38, "the header fills the row");
            assert!(rendered(line).ends_with(count), "count is right-aligned: {:?}", rendered(line));
            let span = |text: &str| {
                line.spans
                    .iter()
                    .find(|span| span.content.as_ref() == text)
                    .unwrap_or_else(|| panic!("a {text} span: {line:?}"))
            };
            assert_eq!(span(number).style.fg, Some(theme::ACCENT_BRIGHT), "the numeral is ACCENT_BRIGHT");
            assert_eq!(span(title).style.fg, Some(theme::TEXT), "the title is TEXT");
            assert!(span(title).style.add_modifier.contains(Modifier::BOLD), "the title is bold");
            let rule = line
                .spans
                .iter()
                .find(|span| !span.content.is_empty() && span.content.chars().all(|c| c == '─'))
                .expect("a rule out to the count");
            assert_eq!(rule.style.fg, Some(theme::LINE), "the rule is LINE");
            assert_eq!(line.spans.last().expect("the count").content.as_ref(), count);
        }
    }

    #[test]
    fn tab_rows_show_the_shortcut_digit_and_keep_the_state_glyph() {
        let active_app = spaces_app();
        let (lines, _, _) = panel_content(&active_app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let at = texts.iter().position(|line| line.contains("one")).expect("the active tab");
        assert!(texts[at].starts_with("1 "), "first tab takes Alt+1: {:?}", texts[at]);
        assert!(texts[at].contains('·'), "the state glyph is kept: {:?}", texts[at]);
        let digit = lines[at]
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "1")
            .expect("the active digit");
        assert_eq!(digit.style.fg, Some(ratatui::style::Color::Rgb(0xff, 0x6a, 0x1f)), "active digit is bright rust");

        let plain = app(true);
        let (lines, _, _) = panel_content(&plain, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let tabs = texts.iter().position(|line| line.trim_start().starts_with("03 TABS ")).expect("the tabs panel");
        let at = texts[tabs + 1..].iter().position(|line| line.contains("pi 3")).expect("an inactive tab") + tabs + 1;
        assert!(texts[at].starts_with("2 "), "second tab takes Alt+2: {:?}", texts[at]);
        let digit = lines[at]
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "2")
            .expect("the inactive digit");
        assert_eq!(digit.style.fg, Some(theme::MUTED), "inactive digit is muted");

        let terminals: Vec<serde_json::Value> = (1..=10)
            .map(|index| {
                json!({
                    "address": format!("t{index}"),
                    "name": format!("tab-{index:02}"),
                    "status": { "state": "idle" },
                })
            })
            .collect();
        let mut ten = App::new("default", "local");
        ten.set_terminals(&json!({ "terminals": terminals, "hidden_finished": 0, "machines": [] }));
        ten.toggle_panel();
        let (lines, _, _) = panel_content(&ten, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let tabs = texts.iter().position(|line| line.trim_start().starts_with("03 TABS ")).expect("the tabs panel");
        let at = texts[tabs + 1..].iter().position(|line| line.contains("tab-10")).expect("the tenth tab") + tabs + 1;
        assert!(texts[at].starts_with("0 "), "the tenth tab takes Alt+0: {:?}", texts[at]);
    }

    #[test]
    fn tab_detail_is_state_app_and_pane_count_only() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [{ "id": "s1", "name": "alpha", "folder": "/a", "tabs": [
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
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let tabs = texts.iter().position(|line| line.trim_start().starts_with("03 TABS ")).expect("the tabs panel");
        let at = texts[tabs + 1..].iter().position(|line| line.contains("claude")).expect("the tab") + tabs + 1;
        assert_eq!(rendered(&lines[at + 1]).trim_start(), "working · pi · 2 panes");
        let word = lines[at + 1]
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "working")
            .expect("the state word");
        assert_eq!(word.style.fg, Some(State::Working.color()), "the word keeps its state colour");
    }

    #[test]
    fn space_detail_uses_the_core_branch_or_the_folder_path() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [{ "id": "s1", "name": "alpha", "folder": "/src/herdr", "tabs": [
                { "id": "tb1", "name": "one", "root": { "id": "tb1", "type": "row", "children": [
                    { "id": "p1", "type": "block", "content": { "terminal": "t1" } }
                ] } }
            ] }] }
        }));
        app.set_terminals(&json!({
            "terminals": [{ "address": "t1", "name": "one", "status": { "state": "idle" } }],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        app.model.folder_info.insert(
            "t1".to_string(),
            crate::model::FolderInfo { top: "/src/herdr".to_string(), branch: Some("workspace-admin".to_string()) },
        );
        let detail = |app: &App| {
            let (lines, _, _) = panel_content(app, 38, false);
            let texts = lines.iter().map(rendered).collect::<Vec<_>>();
            let at = texts.iter().position(|line| line.contains("alpha")).expect("the space") + 1;
            (rendered(&lines[at]).trim_start().to_string(), lines[at].clone())
        };
        let (text, line) = detail(&app);
        assert_eq!(text, "workspace-admin", "branch from the first terminal's folder info");
        assert_eq!(line.spans.iter().find_map(|span| span.style.fg), Some(theme::BODY));

        app.model.folder_info.clear();
        let (text, _) = detail(&app);
        assert_eq!(text, "/src/herdr", "the folder path when core has no branch");
    }

    #[test]
    fn active_blocks_have_a_rust_edge_and_a_warm_wash_with_no_accent_rail() {
        let app = spaces_app();
        let z = zones(Rect::new(0, 0, 40, 24), true);
        let inner = panel_inner(z.panel.expect("the phone sheet"));
        let bg = backgrounds(&app, 40, 24);
        let view = panel_view(&app, inner.width, false, inner.height);
        let texts = view.lines.iter().map(rendered).collect::<Vec<_>>();
        let alpha = texts.iter().position(|line| line.contains("alpha")).expect("the active space");
        let one = texts.iter().position(|line| line.contains("one")).expect("the active tab");
        let edge = ratatui::style::Color::Rgb(0x5c, 0x28, 0x10);
        let wash = ratatui::style::Color::Rgb(0x2a, 0x16, 0x0c);
        for at in [alpha, alpha + 1, one, one + 1] {
            assert!(matches!(view.bands[at], Band::Item { selected: true, .. }), "line {at} is an active block");
            let y = (inner.y + at as u16) as usize;
            assert_eq!(bg[y][inner.x as usize], edge, "one rust-cell edge at line {at}");
            for x in inner.x + 1..inner.x + inner.width {
                assert_eq!(bg[y][x as usize], wash, "wash behind line {at}");
            }
        }
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                assert_ne!(bg[y as usize][x as usize], theme::ACCENT, "no accent rail at ({x},{y})");
            }
        }
    }

    #[test]
    fn space_focus_glyph_and_name_follow_the_active_state() {
        let app = spaces_app();
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let at = |name: &str| texts.iter().position(|line| line.contains(name)).expect(name);
        let active = &lines[at("alpha")];
        let inactive = &lines[at("beta")];
        assert!(rendered(active).starts_with("▸ "), "active focus glyph: {:?}", rendered(active));
        assert!(rendered(inactive).starts_with("○ "), "inactive focus glyph: {:?}", rendered(inactive));
        let fg = |line: &Line<'static>, text: &str| {
            line.spans
                .iter()
                .find(|span| span.content.as_ref() == text)
                .and_then(|span| span.style.fg)
                .unwrap_or_else(|| panic!("a {text} span: {line:?}"))
        };
        assert_eq!(fg(active, "▸"), ratatui::style::Color::Rgb(0xff, 0x6a, 0x1f));
        assert_eq!(fg(inactive, "○"), theme::MUTED);
        assert_eq!(fg(active, "alpha"), theme::TEXT);
        assert_eq!(fg(inactive, "beta"), theme::TEXT);
        assert!(active.spans.iter().any(|span| span.content.as_ref() == "alpha" && span.style.add_modifier.contains(Modifier::BOLD)));
        assert!(inactive.spans.iter().any(|span| span.content.as_ref() == "beta" && !span.style.add_modifier.contains(Modifier::BOLD)));
    }

    #[test]
    fn agent_elapsed_comes_from_the_agent_in_app_and_is_otherwise_omitted() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "agent-one", "status": { "state": "idle", "app": "pi" } },
                { "address": "t2", "name": "agent-two", "status": { "state": "working", "app": "claude" } }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        app.model.set_word("t1", Some("idle".to_string()), Some("pi".to_string()), true);
        let (lines, _, _) = panel_content(&app, 38, false);
        let texts = lines.iter().map(rendered).collect::<Vec<_>>();
        let agents = texts.iter().position(|line| line.starts_with("01 AGENTS ")).expect("the agents panel");
        let find = |name: &str| {
            texts
                .iter()
                .enumerate()
                .skip(agents + 1)
                .find(|(_, line)| line.contains(name))
                .map(|(at, _)| at + 1)
                .unwrap_or_else(|| panic!("the {name} agent: {texts:?}"))
        };
        let first = find("agent-one");
        let first_detail = rendered(&lines[first]);
        let parts = first_detail.trim_start().split(" · ").collect::<Vec<_>>();
        assert_eq!(parts.len(), 3, "state, app and elapsed: {:?}", rendered(&lines[first]));
        assert_eq!(parts[0], "idle");
        assert_eq!(parts[1], "pi");
        assert!(parts[2].chars().next().is_some_and(|c| c.is_ascii_digit()), "elapsed begins with a number: {parts:?}");
        assert!(parts[2].ends_with(['s', 'm', 'h']), "elapsed has a unit: {parts:?}");

        let second = find("agent-two");
        assert_eq!(rendered(&lines[second]).trim_start(), "working · claude", "no invented elapsed");
    }

    #[test]
    fn elapsed_labels_scale_from_seconds_to_hours() {
        assert_eq!(elapsed_label(0, 10_000), None, "a missing since is omitted");
        assert_eq!(elapsed_label(1_000, 59_999), Some("58s".to_string()));
        assert_eq!(elapsed_label(1_000, 61_000), Some("1m".to_string()));
        assert_eq!(elapsed_label(1_000, 3_601_000), Some("1h".to_string()));
    }

    #[test]
    fn taps_route_flat_blocks_without_a_nesting_indent() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [
                { "id": "s1", "name": "alpha", "folder": "/a", "tabs": [
                    { "id": "tb1", "name": "one", "root": { "id": "tb1", "type": "row", "children": [
                        { "id": "p1", "type": "block", "content": { "terminal": "t1" } }
                    ] } }
                ] },
                { "id": "s2", "name": "beta", "folder": "/b", "tabs": [
                    { "id": "tb2", "name": "two", "root": { "id": "tb2", "type": "row", "children": [] } }
                ] }
            ] }
        }));
        app.set_terminals(&json!({
            "terminals": [{ "address": "t1", "name": "pi", "status": { "state": "working", "app": "pi" } }],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        let panel = zones(Rect::new(0, 0, 40, 24), true).panel.expect("the phone sheet");
        let inner = panel_inner(panel);
        let view = panel_view(&app, inner.width, false, inner.height);
        let texts = view.lines.iter().map(rendered).collect::<Vec<_>>();
        let at = |text: &str| texts.iter().position(|line| line.contains(text)).expect(text);
        let beta = at("beta");
        let one = at("one");
        let agent = at("◐ pi");
        let y = |line: usize| inner.y + line as u16;
        for line in [beta, beta + 1] {
            assert_eq!(panel_space_at(&app, panel, inner.x + 2, y(line), false), Some("s2".to_string()));
        }
        for line in [one, one + 1] {
            assert_eq!(panel_row_at(&app, panel, inner.x + 2, y(line), false), Some(0));
        }
        for line in [agent, agent + 1] {
            assert_eq!(panel_row_at(&app, panel, inner.x + 2, y(line), false), Some(0), "an agent targets its tab");
        }
        assert!(texts[beta - 1].trim().is_empty(), "one blank row between spaces");
        assert_eq!(panel_space_at(&app, panel, inner.x + 2, y(beta - 1), false), None);
        assert_eq!(panel_row_at(&app, panel, inner.x + 2, y(beta - 1), false), None);
        let first_cell = |line: &str| line.chars().position(|c| !c.is_whitespace());
        assert_eq!(first_cell(&texts[beta]), first_cell(&texts[one]), "tab is not nested under a space");
        assert_eq!(first_cell(&texts[agent]), first_cell(&texts[one]), "agent shares the same inset");
        assert!(!texts.iter().any(|line| line.contains("t1")), "no child terminal row");
    }
}
