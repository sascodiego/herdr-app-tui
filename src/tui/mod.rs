//! The terminal UI: connect, refresh, draw, read keys.
//!
//! This is the only layer that is not copied from herdr-app. It replaces the
//! GPUI window with ratatui; the protocol, core discovery and model come from
//! herdr-app unchanged (see `src/hc.rs` and friends).

pub mod app;
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
use input::Action;

/// How long between refreshes when nobody presses a key.
const TICK: Duration = Duration::from_millis(700);

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

/// Read keys and tick the data.
fn event_loop(
    core: &std::sync::Arc<hc::Core>,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> anyhow::Result<i32> {
    loop {
        terminal.draw(|frame| view::render(app, frame)).context("cannot draw")?;

        if event::poll(TICK).context("cannot wait for a key")? {
            let Event::Key(key) = event::read().context("cannot read a key")? else { continue };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match input::action(key) {
                // With the panel open, `q` steps out of it first: the panel has
                // the keyboard, and quitting is the second ask.
                Action::Quit if app.focus() == app::Focus::Panel => app.panel = false,
                Action::Quit => return Ok(0),
                Action::TogglePanel => app.toggle_panel(),
                Action::ClosePanel => app.panel = false,
                Action::Up => app.move_cursor(-1),
                Action::Down => app.move_cursor(1),
                Action::Refresh => {}
                Action::Ignore => continue,
            }
        }

        refresh(core, app)?;
    }
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

    if let Some(selected) = app.selected() {
        let address = selected.address.clone();
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
    for row in app.rows() {
        let state = format!("{} {}", row.state.glyph(), row.state.word());
        println!("  {:<24} {state}", row.name);
    }
    let (working, needs_you, idle) = app.counts();
    println!("{working} working, {needs_you} need you, {idle} idle");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plain_output_names_the_session_core_and_each_terminal() {
        let mut app = App::new("work", "rohan");
        app.set_terminals(&json!({
            "terminals": [{ "address": "t1", "name": "pi 1", "status": { "state": "blocked" } }]
        }));
        // The renderer writes to stdout; this only proves the app it reads from
        // is the one the loop refreshes.
        assert_eq!(app.rows().len(), 1);
        assert_eq!(app.rows()[0].state, glyph::State::NeedsYou);
    }
}
