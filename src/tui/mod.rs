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

use anyhow::{Context, anyhow};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_json::{Value, json};
use std::io::IsTerminal;
use std::time::Duration;

use crate::hc;
use app::App;
use attach::Session;
use input::Action;

/// How long between refreshes when nobody presses a key.
const TICK: Duration = Duration::from_millis(250);

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
    refresh(&core, &mut app)?;

    // Not a terminal (a pipe, a test): print once and leave, so the binary stays
    // usable from a script.
    if !std::io::stdout().is_terminal() {
        print_plain(&app);
        return Ok(0);
    }

    let mut terminal = ratatui::init();
    let outcome = event_loop(&core, &mut app, &mut terminal);
    ratatui::restore();
    outcome
}

/// Read keys, drain the stream, tick the data.
fn event_loop(
    core: &std::sync::Arc<hc::Core>,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> anyhow::Result<i32> {
    let mut attached: Option<Session> = None;
    let mut attached_address: Option<String> = None;

    loop {
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
            app.screen = Some(view::ScreenState::from_frame(session.frame()));
        }

        terminal.draw(|frame| view::render(app, frame)).context("cannot draw")?;

        if event::poll(TICK).context("cannot wait for a key")? {
            let Event::Key(key) = event::read().context("cannot read a key")? else { continue };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let action = input::action(key);

            // The panel owns the keyboard only while it is open. Ctrl+\ is the
            // one way out; Enter attaches to the highlighted tab and closes.
            if app.panel {
                match action {
                    Action::Quit => {
                        detach(attached.take(), core);
                        return Ok(0);
                    }
                    Action::ClosePanel | Action::TogglePanel | Action::Attach => app.panel = false,
                    Action::Up => app.move_cursor(-1),
                    Action::Down => app.move_cursor(1),
                    Action::Refresh => {}
                    Action::Ignore => continue,
                }
                continue;
            }

            match action {
                Action::Quit => {
                    detach(attached.take(), core);
                    return Ok(0);
                }
                Action::TogglePanel => app.panel = true,
                // With nothing attached, Esc leaves; with a live pane it is a
                // key for the program, like q, r and Ctrl+C.
                Action::ClosePanel if attached.is_none() => {
                    detach(attached.take(), core);
                    return Ok(0);
                }
                _ => {
                    if let Some(session) = attached.as_mut() {
                        session.key(core, key);
                    }
                }
            }
        }

        refresh(core, app)?;
    }
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
    let (cols, rows) = if width >= view::WIDE_AT {
        (width.saturating_sub(view::RAIL_WIDTH + 1), height)
    } else {
        (width, height.saturating_sub(1))
    };
    (cols.max(1), rows.max(1))
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

/// Pull the terminal list, and the highlighted terminal's tail.
fn refresh(core: &std::sync::Arc<hc::Core>, app: &mut App) -> anyhow::Result<()> {
    let list = core.call("terminal.list", json!({})).context("terminal.list")?;
    app.set_terminals(&list);

    match core.call("doc.get", json!({ "name": "herdr-app" })) {
        Ok(document) => app.set_workspace(&document),
        Err(error) => app.doc_error = Some(format!("cannot read the workspace document: {error:#}")),
    }

    if let Some(address) = app.selected().and_then(|row| row.address.clone()) {
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
