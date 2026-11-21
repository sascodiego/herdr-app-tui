//! The terminal UI: connect, refresh, draw, read keys.
//!
//! This is the only layer that is not copied from herdr-app. It replaces the
//! GPUI window with ratatui; the protocol, core discovery and model come from
//! herdr-app unchanged (see `src/hc.rs` and friends).

pub mod app;
pub mod attach;
pub mod glyph;
pub mod input;
pub mod theme;
pub mod view;
pub mod workspace;

use anyhow::{Context, anyhow};
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_json::{Value, json};
use std::io::{IsTerminal, Write};
use std::time::Duration;

use crate::hc;
use app::App;
use attach::Session;
use input::Action;

/// The mouse modes Termux, tmux and mosh all agree on: press/release,
/// button-drag and SGR coordinates. Deliberately no `?1003h` (any-motion,
/// which Termux ignores) and no `?1015h` (RXVT, superseded by SGR).
const MOUSE_ON: &[u8] = b"\x1b[?1000h\x1b[?1002h\x1b[?1006h";
const MOUSE_OFF: &[u8] = b"\x1b[?1006l\x1b[?1002l\x1b[?1000l";

/// Enable or disable the minimal mouse reporting ourselves. crossterm's
/// `EnableMouseCapture` emits `?1003h` too, which mosh collapses into its one
/// mode and Termux does not implement, turning swipes into arrow keys.
fn write_mouse_reporting(out: &mut impl Write, on: bool) -> anyhow::Result<()> {
    out.write_all(if on { MOUSE_ON } else { MOUSE_OFF })
        .context("cannot set mouse reporting")?;
    out.flush().context("cannot flush mouse reporting")
}

/// One screen frame. Terminal output does not produce a crossterm event, so a
/// long poll would leave the attach stream waiting: bytes would arrive in
/// 250 ms bursts instead of as they are written. ~60 fps keeps it fluid.
const FRAME: Duration = Duration::from_millis(16);
/// Full resync even if no event arrived, in case the stream went quiet.
const HEARTBEAT_FRAMES: u32 = 250;

/// Run the screen. Returns the process exit code.
pub fn run() -> anyhow::Result<i32> {
    let core = hc::Core::connect().context("cannot reach herdr-core")?;
    hc::check(&core.hello).map_err(describe_mismatch)?;

    let session = std::env::var("HC_SESSION")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "default".to_string());
    let core_name = core
        .hello
        .pointer("/core/name")
        .and_then(Value::as_str)
        .unwrap_or("core")
        .to_string();

    let mut app = App::new(&session, &core_name);
    refresh(&core, &mut app, true)?;

    // Not a terminal (a pipe, a test): print once and leave, so the binary stays
    // usable from a script.
    if !std::io::stdout().is_terminal() {
        print_plain(&app);
        return Ok(0);
    }

    let (channel, events, snapshot) = core.subscribe().context("events.subscribe")?;
    let mut terminal = ratatui::init();
    write_mouse_reporting(&mut std::io::stdout().lock(), true)?;
    let outcome = event_loop(&core, &mut app, &mut terminal, (channel, events), snapshot);
    let _ = write_mouse_reporting(&mut std::io::stdout().lock(), false);
    ratatui::restore();
    outcome
}

/// Read keys, drain the streams, redraw. The core pushes changes; a key never
/// waits for a round trip to list terminals or read the document.
fn event_loop(
    core: &std::sync::Arc<hc::Core>,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    subscription: (u32, async_channel::Receiver<hc::Stream>),
    snapshot: Value,
) -> anyhow::Result<i32> {
    let mut attached: Option<Session> = None;
    let mut attached_address: Option<String> = None;
    let mut subscription = Some(subscription);
    let mut lines = hc::EventLines::new(&snapshot);
    let mut ticks = 0u32;
    let mut pointer_down: Option<(u16, u16)> = None;
    let mut drag_anchor: Option<u16> = None;

    loop {
        if subscription.is_none()
            && let Ok((channel, events, snapshot)) = core.subscribe()
        {
            subscription = Some((channel, events));
            lines = hc::EventLines::new(&snapshot);
            refresh(core, app, attached.is_none())?;
        }

        let mut sub = subscription.take();
        let mut docs_changed = false;
        let mut terminals_changed = false;
        if let Some((channel, events)) = &sub {
            while let Ok(stream) = events.try_recv() {
                match stream {
                    hc::Stream::Data(bytes) => {
                        let pushed = lines.push(&bytes);
                        docs_changed |= pushed.docs;
                        terminals_changed |= pushed.terminals;
                        core.credit(*channel, bytes.len());
                    }
                    _ => {
                        sub = None;
                        break;
                    }
                }
            }
        }
        subscription = sub;

        // A tab selected in the panel owns the pane. Attach as soon as it is
        // selected, including on the first frame, so there is a live prompt
        // without waiting for another key press.
        sync_attachment(core, app, terminal, &mut attached, &mut attached_address)?;

        // Keep the viewer's grid the same size as the terminal pane.
        if let Some(session) = attached.as_mut() {
            let area = terminal.get_frame().area();
            let (cols, rows) = pane_size(app, area.width, area.height);
            session.resize(core, cols, rows);
            session.drain(core)?;
            // In copy mode this frame is libghostty's scrolled viewport, so
            // the program's colors and attributes come along with the text.
            app.screen = Some(view::ScreenState::from_frame(session.frame()));
        }

        terminal.draw(|frame| view::render(app, frame)).context("cannot draw")?;

        if event::poll(FRAME).context("cannot wait for an event")? {
            match event::read().context("cannot read an event")? {
                Event::Key(key) => {
                    if handle_key(core, app, key, &mut attached)? {
                        detach(attached.take(), core);
                        return Ok(0);
                    }
                }
                Event::Mouse(mouse) => {
                    handle_pointer(
                        app, terminal, mouse, &mut attached, &mut pointer_down, &mut drag_anchor,
                    )?;
                }
                _ => {}
            }
        }

        let heartbeat = ticks.is_multiple_of(HEARTBEAT_FRAMES);
        ticks = ticks.saturating_add(1);
        if docs_changed || terminals_changed || heartbeat {
            refresh(core, app, attached.is_none())?;
        }
    }
}

/// Handle one key. Returns true when the screen must quit. Copy mode owns the
/// keyboard while it is open, just as a live program does otherwise.
fn handle_key(
    core: &std::sync::Arc<hc::Core>,
    app: &mut App,
    key: KeyEvent,
    attached: &mut Option<Session>,
) -> anyhow::Result<bool> {
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }
    let action = input::action(key);
    if action == Action::Quit {
        return Ok(true);
    }

    match action {
        Action::ScrollUp => {
            scroll_history(app, attached, true);
            return Ok(false);
        }
        Action::ScrollDown => {
            scroll_history(app, attached, false);
            return Ok(false);
        }
        _ => {}
    }

    if app.copy
        && (action == Action::ClosePanel
            || (key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE))
    {
        resume_live(app, attached);
        return Ok(false);
    }

    if app.panel {
        match action {
            Action::ClosePanel | Action::TogglePanel | Action::Attach => app.panel = false,
            Action::Up | Action::ScrollUp => app.move_cursor(-1),
            Action::Down | Action::ScrollDown => app.move_cursor(1),
            Action::Refresh | Action::Command | Action::Ignore => {}
            Action::Quit => return Ok(true),
        }
        return Ok(false);
    }

    match action {
        Action::TogglePanel => {
            app.panel = true;
            app.exit_copy();
        }
        // With nothing attached, Esc leaves; with a live pane it is a key for
        // the program, like q, r and Ctrl+C.
        Action::ClosePanel if attached.is_none() => return Ok(true),
        Action::ScrollUp | Action::ScrollDown | Action::Command => {
            // Both were handled before the panel/following branches.
        }
        _ => {
            if let Some(session) = attached.as_mut() {
                session.key(core, key);
            }
        }
    }
    Ok(false)
}

/// Move through the same styled scrollback the live pane renders.
fn scroll_history(app: &mut App, attached: &mut Option<Session>, up: bool) {
    let Some(session) = attached.as_mut() else {
        return;
    };
    if !app.copy {
        app.enter_copy();
    }
    if up {
        session.scroll_up(1);
    } else {
        session.scroll_down(1);
    }
}

/// Follow live output again, leaving the scrolled viewport.
fn resume_live(app: &mut App, attached: &mut Option<Session>) {
    app.exit_copy();
    if let Some(session) = attached.as_mut() {
        session.scroll_to_live();
    }
}

/// A pointer's zone, resolved from the same rectangles the renderer uses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Zone {
    Header,
    Panel,
    Terminal,
}

fn zone_at(z: view::Zones, x: u16, y: u16) -> Option<Zone> {
    if view::Zones::contains(z.header, x, y) {
        Some(Zone::Header)
    } else if view::Zones::contains(z.panel, x, y) {
        Some(Zone::Panel)
    } else if view::Zones::contains(Some(z.terminal), x, y) {
        Some(Zone::Terminal)
    } else {
        None
    }
}

/// Route a touch or pointer event by zone. A tap keeps its old meaning: open
/// the sheet, select a row, or resume the live pane. Movement becomes scroll.
fn handle_pointer(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mouse: MouseEvent,
    attached: &mut Option<Session>,
    pointer_down: &mut Option<(u16, u16)>,
    drag_anchor: &mut Option<u16>,
) -> anyhow::Result<()> {
    let Some(gesture) = input::pointer(mouse.kind) else {
        return Ok(());
    };
    let (x, y) = (mouse.column, mouse.row);
    let z = view::zones(terminal.get_frame().area(), app.panel);
    let zone = zone_at(z, x, y);

    match gesture {
        input::Pointer::Tap => match mouse.kind {
            MouseEventKind::Down(_) => {
                *pointer_down = Some((x, y));
                *drag_anchor = Some(y);
            }
            MouseEventKind::Up(_) => {
                let is_tap = pointer_down
                    .take()
                    .is_some_and(|(sx, sy)| sx.abs_diff(x) <= 1 && sy.abs_diff(y) <= 1);
                *drag_anchor = None;
                if !is_tap {
                    return Ok(());
                }
                match zone {
                    Some(Zone::Header) => {
                        resume_live(app, attached);
                        app.panel = !app.panel;
                    }
                    Some(Zone::Panel) => {
                        if let Some(index) = z.panel.and_then(|p| view::panel_row_at(app, p, x, y, z.wide)) {
                            app.cursor = index;
                            if !z.wide {
                                app.panel = false;
                            }
                        }
                    }
                    Some(Zone::Terminal) => resume_live(app, attached),
                    None => {}
                }
            }
            _ => {}
        },
        input::Pointer::ScrollUp | input::Pointer::ScrollDown => {
            let up = gesture == input::Pointer::ScrollUp;
            match zone {
                Some(Zone::Panel) => app.move_cursor(if up { -1 } else { 1 }),
                Some(Zone::Terminal) => scroll_history(app, attached, up),
                None | Some(Zone::Header) => {}
            }
        }
        input::Pointer::Drag => {
            let Some(anchor) = drag_anchor.as_mut() else {
                *drag_anchor = Some(y);
                return Ok(());
            };
            let delta = i32::from(y) - i32::from(*anchor);
            if delta.abs() < 2 {
                return Ok(());
            }
            let up = delta < 0;
            match zone {
                Some(Zone::Panel) => app.move_cursor(if up { -1 } else { 1 }),
                Some(Zone::Terminal) => scroll_history(app, attached, up),
                None | Some(Zone::Header) => {}
            }
            *anchor = y;
        }
    }
    Ok(())
}

/// Leave the terminal attached on the core side; it keeps running and another
/// client can pick it up.
fn detach(session: Option<Session>, core: &std::sync::Arc<hc::Core>) {
    if let Some(session) = session {
        session.detach(core);
    }
}

/// Attach to the tab the panel highlights. Detach from the previous one when
/// the selection changes; a failed attach is remembered so a broken terminal
/// does not retry every tick.
fn sync_attachment(
    core: &std::sync::Arc<hc::Core>,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    attached: &mut Option<Session>,
    attached_address: &mut Option<String>,
) -> anyhow::Result<()> {
    let desired = app.selected().and_then(|row| row.address.clone());
    if desired == *attached_address {
        return Ok(());
    }
    // Frozen history belongs to the previous terminal.
    app.exit_copy();
    detach(attached.take(), core);
    let Some(address) = desired else {
        app.screen = None;
        *attached_address = None;
        return Ok(());
    };
    let area = terminal.get_frame().area();
    let (cols, rows) = pane_size(app, area.width, area.height);
    match Session::attach(core, &address, cols, rows) {
        Ok(session) => {
            *attached = Some(session);
            *attached_address = Some(address);
        }
        Err(error) => {
            app.doc_error = Some(format!("cannot attach {address}: {error:#}"));
            app.screen = None;
            *attached_address = Some(address);
        }
    }
    Ok(())
}

/// The terminal pane's size in cells, accounting for the rail and header.
fn pane_size(_app: &App, width: u16, height: u16) -> (u16, u16) {
    // The phone sheet covers the pane but does not resize it: use the closed
    // face's geometry so opening the sidebar cannot reflow a live program.
    let zone = view::zones(ratatui::layout::Rect::new(0, 0, width, height), false);
    (zone.terminal.width.max(1), zone.terminal.height.max(1))
}

/// Why a running core cannot serve this screen, in words rather than a `Debug` dump.
fn describe_mismatch(mismatch: hc::Mismatch) -> anyhow::Error {
    match mismatch {
        hc::Mismatch::TooOld { have, need } => {
            anyhow!("the core at {have} is older than the {need} this screen needs: update the core")
        }
        hc::Mismatch::Newer { generation } => {
            anyhow!("the core speaks generation {generation}, newer than this screen: update herdr-app-tui")
        }
        hc::Mismatch::Missing(methods) => {
            anyhow!("the core lacks {} it needs: update the core", methods.join(", "))
        }
    }
}

/// Pull what changed. The preview is only needed while no terminal is
/// attached: a live pane already shows the output.
fn refresh(core: &std::sync::Arc<hc::Core>, app: &mut App, preview: bool) -> anyhow::Result<()> {
    let list = core.call("terminal.list", json!({})).context("terminal.list")?;
    app.set_terminals(&list);

    match core.call("doc.get", json!({ "name": "herdr-app" })) {
        Ok(document) => app.set_workspace(&document),
        Err(error) => app.doc_error = Some(format!("cannot read the workspace document: {error:#}")),
    }

    if preview
        && let Some(address) = app.selected().and_then(|row| row.address.clone())
    {
        let read = core
            .call("terminal.read", json!({ "terminal": address, "lines": app::PREVIEW_LINES }))
            .context("terminal.read")?;
        if let Some(text) = read.get("text").and_then(Value::as_str) {
            app.set_preview(&address, text);
        }
    }

    app.connected = true;
    Ok(())
}

/// The screen as plain text, for a pipe.
fn print_plain(app: &App) {
    println!("{} · {} — {} terminals", app.session, app.core, app.terminals.len());
    if let Some(error) = &app.doc_error {
        println!("workspace: {error}");
    }
    for row in app.rows() {
        let state = format!("{} {}", row.state.glyph(), row.state.word());
        println!("  {:<24} {state}", row.name);
    }
    let (working, needs_you, idle) = app.counts();
    println!("{working} working, {needs_you} need you, {idle} idle");
}


/// Attach once, send a line, print the resulting screen. A scriptable way to
/// prove the round trip without a TTY.
pub fn attach_once(terminal: &str, text: Option<&str>) -> i32 {
    let core = match hc::Core::connect() {
        Ok(core) => std::sync::Arc::new(core),
        Err(error) => {
            eprintln!("herdr-app-tui: cannot reach the core: {error:#}");
            return 1;
        }
    };
    let mut session = match Session::attach(&core, terminal, 80, 24) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("herdr-app-tui: {error:#}");
            return 1;
        }
    };
    if let Some(text) = text {
        session.type_text(&core, text);
        session.type_text(&core, "\r");
    }
    // Drain whatever arrives, with or without a message to send.
    for _ in 0..40 {
        let _ = session.drain(&core);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let frame = session.frame();
    for row in frame.rows {
        let line: String = row.iter().map(|cell| if cell.text.is_empty() { " " } else { &cell.text }).collect();
        println!("{}", line.trim_end());
    }
    session.detach(&core);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_reporting_uses_only_the_modes_everyone_understands() {
        let mut out = Vec::new();
        write_mouse_reporting(&mut out, true).expect("enable");
        let on = String::from_utf8(out).expect("utf8");
        assert_eq!(on, "\x1b[?1000h\x1b[?1002h\x1b[?1006h");
        assert!(!on.contains("1003"), "Termux ignores any-motion");
        assert!(!on.contains("1015"), "SGR supersedes RXVT coordinates");

        let mut out = Vec::new();
        write_mouse_reporting(&mut out, false).expect("disable");
        let off = String::from_utf8(out).expect("utf8");
        assert_eq!(off, "\x1b[?1006l\x1b[?1002l\x1b[?1000l");
        assert!(!off.contains("1003"));
        assert!(!off.contains("1015"));
    }

    #[test]
    fn the_mosh_wrapper_has_nothing_to_rewrite() {
        let mut out = Vec::new();
        write_mouse_reporting(&mut out, true).expect("enable");
        write_mouse_reporting(&mut out, false).expect("disable");
        let original = String::from_utf8(out).expect("utf8");
        let rewritten = original
            .replace("\x1b[?1003h", "\x1b[?1002h")
            .replace("\x1b[?1003l", "\x1b[?1002l");
        assert_eq!(rewritten, original, "a wrapper must be a no-op");
    }

    #[test]
    fn the_core_pane_matches_the_phone_terminal_zone() {
        let mut app = App::new("default", "local");
        for panel in [false, true] {
            app.panel = panel;
            let (cols, rows) = pane_size(&app, 40, 14);
            let zone = view::zones(ratatui::layout::Rect::new(0, 0, 40, 14), false);
            assert_eq!((cols, rows), (zone.terminal.width, zone.terminal.height), "panel={panel}");
            assert_eq!(rows, 14 - view::BAND_ROWS, "the prompt must not be clipped");
        }
    }
}
