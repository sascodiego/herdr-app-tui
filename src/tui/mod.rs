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
use app::{App, Prompt};
use attach::Session;
use input::Action;
use workspace::{Command, Mutation};

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

        // A tab whose terminal vanished cannot own the pane: follow another
        // one, or open a fresh shell at home when nothing is left.
        ensure_live_selection(core, app)?;

        // A tab selected in the panel owns the pane. Attach as soon as it is
        // selected, including on the first frame, so there is a live prompt
        // without waiting for another key press.
        sync_attachment(core, app, terminal, &mut attached, &mut attached_address)?;

        // Keep the viewer's grid the same size as the terminal pane. A closed
        // stream is a normal event: detach and let the next frame select
        // another tab, or open a shell at home when none is left.
        let mut closed = false;
        if let Some(session) = attached.as_mut() {
            let area = terminal.get_frame().area();
            let (cols, rows) = pane_size(app, area.width, area.height);
            app.pane = (cols, rows);
            session.resize(core, cols, rows);
            match session.drain(core) {
                Ok(_) => {
                    // In copy mode this frame is libghostty's scrolled viewport.
                    app.screen = Some(view::ScreenState::from_frame(session.frame()));
                }
                Err(error) => {
                    app.doc_error = Some(format!("terminal closed: {error:#}"));
                    closed = true;
                }
            }
        }
        if closed {
            detach(attached.take(), core);
            attached_address = None;
            app.screen = None;
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
                        core, app, terminal, mouse, &mut attached, &mut pointer_down, &mut drag_anchor,
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
    if app.panel && app.prompt.is_some() {
        return handle_prompt_key(core, app, key);
    }
    let action = input::action(key);
    if action == Action::Quit {
        return Ok(true);
    }
    if action == Action::Prefix {
        app.prefix = !app.prefix;
        return Ok(false);
    }
    if app.prefix {
        return handle_prefix_key(core, app, key);
    }
    if let Action::SelectTab(index) = action {
        select_tab_index(app, index);
        return Ok(false);
    }
    if action == Action::AgentUp || action == Action::AgentDown {
        move_agent(app, action == Action::AgentUp);
        return Ok(false);
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
            Action::Up | Action::ScrollUp => move_panel_block(app, false),
            Action::Down | Action::ScrollDown => move_panel_block(app, true),
            Action::Refresh
            | Action::Ignore
            | Action::Prefix
            | Action::SelectTab(_)
            | Action::AgentUp
            | Action::AgentDown => {}
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
        Action::ScrollUp
        | Action::ScrollDown
        | Action::Prefix
        | Action::SelectTab(_)
        | Action::AgentUp
        | Action::AgentDown => {
            // Handled before the panel/following branches.
        }
        _ => {
            if let Some(session) = attached.as_mut() {
                session.key(core, key);
            }
        }
    }
    Ok(false)
}

fn commit(core: &std::sync::Arc<hc::Core>, app: &mut App, mutation: Mutation) -> anyhow::Result<()> {
    let version = app.doc_version.unwrap_or(0);
    let version = workspace::apply(core, &mut app.model, version, None, |model| {
        mutation.apply(model);
    })?;
    app.doc_version = Some(version);
    app.rebuild_rows();
    Ok(())
}

/// Select a space from the sidebar. The shared document has no `active`
/// field, so this choice stays in this screen's model, like herdr's view state.
fn select_space(app: &mut App, id: &str) {
    if app.model.active.as_deref() == Some(id) {
        return;
    }
    Mutation::SelectSpace { id: id.to_string() }.apply(&mut app.model);
    app.rebuild_rows();
}

fn commit_close(core: &std::sync::Arc<hc::Core>, app: &mut App, mutation: Mutation) -> anyhow::Result<()> {
    let doomed = mutation.apply(&mut app.model);
    workspace::kill_tree(core, &doomed)?;
    let version = app.doc_version.unwrap_or(0);
    let version = workspace::apply(core, &mut app.model, version, None, |model| {
        mutation.apply(model);
    })?;
    app.doc_version = Some(version);
    app.rebuild_rows();
    Ok(())
}

fn remember_terminal(app: &mut App, address: &str, name: &str) {
    if app.terminals.iter().any(|terminal| terminal.address == address) {
        return;
    }
    app.terminals.push(app::Terminal {
        address: address.to_string(),
        name: name.to_string(),
        state: crate::tui::glyph::State::Unknown,
        is_agent: false,
        parent: None,
        preview: Vec::new(),
        app: None,
    });
}

fn select_current(app: &mut App) {
    let Some(id) = app.model.active_tab().map(|tab| tab.id.clone()) else { return };
    if let Some(index) = app.rows.iter().position(|row| row.id == id) {
        app.cursor = index;
    }
}

fn ensure_open_workspace(core: &std::sync::Arc<hc::Core>, app: &mut App) -> anyhow::Result<()> {
    let live = |address: &str| app.terminals.iter().any(|terminal| terminal.address == address);
    if app
        .rows
        .iter()
        .any(|row| row.address.as_deref().is_some_and(live))
    {
        return Ok(());
    }

    // Drop spaces whose terminals the core has already removed, otherwise a
    // stale tab would keep looking like a live choice in the panel.
    let stale: Vec<String> = app
        .model
        .spaces
        .iter()
        .filter(|space| {
            !space.tabs.iter().any(|tab| {
                tab.root
                    .terminals()
                    .iter()
                    .filter_map(|node| node.terminal())
                    .any(live)
            })
        })
        .map(|space| space.id.clone())
        .collect();
    for id in &stale {
        app.model.remove_space(id);
    }

    let home = crate::mods::home().to_string_lossy().to_string();
    let mutation = Mutation::new_space(
        &mut app.model,
        String::new(),
        home.clone(),
        "tab".to_string(),
        String::new(),
    );
    let (space, tab) = match &mutation {
        Mutation::NewSpace { id, tab_id, .. } => (id.clone(), tab_id.clone()),
        _ => return Ok(()),
    };
    let address = workspace::launch(core, Some(&home), app.pane.0, app.pane.1, &space, &tab)?;
    remember_terminal(app, &address, "tab");
    let version = app.doc_version.unwrap_or(0);
    let version = workspace::apply(
        core,
        &mut app.model,
        version,
        None,
        |model| {
            for id in &stale {
                model.remove_space(id);
            }
            mutation.clone().with_address(address.clone()).apply(model);
        },
    )?;
    app.doc_version = Some(version);
    app.rebuild_rows();
    app.cursor = 0;
    Ok(())
}

fn ensure_live_selection(core: &std::sync::Arc<hc::Core>, app: &mut App) -> anyhow::Result<()> {
    let Some(address) = app.selected().and_then(|row| row.address.clone()) else {
        return ensure_open_workspace(core, app);
    };
    if app.terminals.iter().any(|terminal| terminal.address == address) {
        return Ok(());
    }
    let live = app.rows.iter().position(|row| {
        row.address
            .as_deref()
            .is_some_and(|address| app.terminals.iter().any(|terminal| terminal.address == address))
    });
    if let Some(index) = live {
        app.cursor = index;
        app.rebuild_rows();
        return Ok(());
    }
    ensure_open_workspace(core, app)
}

fn close_and_fallback(
    core: &std::sync::Arc<hc::Core>,
    app: &mut App,
    mutation: Mutation,
) -> anyhow::Result<()> {
    commit_close(core, app, mutation)?;
    select_current(app);
    ensure_open_workspace(core, app)
}

fn current_space(app: &App) -> Option<String> {
    app.model.active.clone()
}

fn execute_command(core: &std::sync::Arc<hc::Core>, app: &mut App, command: Command) -> anyhow::Result<()> {
    match command {
        Command::CloseTab | Command::CloseSpace => {
            app.panel = true;
            let label = if command == Command::CloseTab { "Close tab?" } else { "Close space?" };
            app.prompt = Some(Prompt::ConfirmClose { label: label.to_string(), command });
            Ok(())
        }
        other => run_command(core, app, other),
    }
}

fn run_command(core: &std::sync::Arc<hc::Core>, app: &mut App, command: Command) -> anyhow::Result<()> {
    match command {
        Command::TogglePanel => {
            app.panel = !app.panel;
            if app.panel {
                app.exit_copy();
            }
            Ok(())
        }
        Command::NewSpace => {
            let name = format!("space {}", app.model.spaces.len() + 1);
            let mutation = Mutation::new_space(
                &mut app.model,
                name.clone(),
                app.path.clone(),
                "tab".to_string(),
                String::new(),
            );
            let (space, tab) = match &mutation {
                Mutation::NewSpace { id, tab_id, .. } => (id.clone(), tab_id.clone()),
                _ => return Ok(()),
            };
            let address = workspace::launch(core, Some(&app.path), app.pane.0, app.pane.1, &space, &tab)?;
            remember_terminal(app, &address, &name);
            commit(core, app, mutation.with_address(address))
        }
        Command::NewTab => {
            let space = current_space(app).ok_or_else(|| anyhow!("no space yet: make one first"))?;
            let folder = app
                .model
                .space(&space)
                .map(|space| space.folder.clone())
                .unwrap_or_else(|| app.path.clone());
            let mutation = Mutation::new_tab(
                &mut app.model,
                Some(space.clone()),
                "tab".to_string(),
                String::new(),
            );
            let tab = match &mutation {
                Mutation::NewTab { id, .. } => id.clone(),
                Mutation::NewSpace { tab_id, .. } => tab_id.clone(),
                _ => return Ok(()),
            };
            let address = workspace::launch(core, Some(&folder), app.pane.0, app.pane.1, &space, &tab)?;
            remember_terminal(app, &address, "tab");
            commit(core, app, mutation.with_address(address))
        }
        Command::RenameSpace => {
            let Some(id) = current_space(app) else { return Ok(()) };
            app.panel = true;
            let buffer = app.model.space(&id).map_or(String::new(), |space| space.name.clone());
            app.prompt = Some(Prompt::RenameSpace { id, buffer });
            Ok(())
        }
        Command::RenameTab => {
            let Some(tab) = app.model.active_tab().map(|tab| tab.id.clone()) else { return Ok(()) };
            app.panel = true;
            let buffer = app.model.tab(&tab).map_or(String::new(), |tab| tab.name.clone());
            app.prompt = Some(Prompt::RenameTab { id: tab, buffer });
            Ok(())
        }
        Command::CloseTab => {
            let Some(tab) = app.model.active_tab().map(|tab| tab.id.clone()) else { return Ok(()) };
            close_and_fallback(core, app, Mutation::CloseTab { id: tab })
        }
        Command::CloseSpace => {
            let Some(space) = current_space(app) else { return Ok(()) };
            close_and_fallback(core, app, Mutation::CloseSpace { id: space })
        }
        Command::SelectNextSpace | Command::SelectPrevSpace => {
            let count = app.model.spaces.len();
            if count == 0 {
                return Ok(());
            }
            let at = app.model.active_index().unwrap_or(0);
            let step = if command == Command::SelectNextSpace { 1 } else { count - 1 };
            let id = app.model.spaces[(at + step) % count].id.clone();
            app.model.active = Some(id);
            app.rebuild_rows();
            Ok(())
        }
        Command::SelectNextTab | Command::SelectPrevTab => {
            let Some(space) = current_space(app) else { return Ok(()) };
            let Some(space_mut) = app.model.space_mut(&space) else { return Ok(()) };
            let count = space_mut.tabs.len();
            if count == 0 {
                return Ok(());
            }
            let at = space_mut.tabs.iter().position(|tab| Some(tab.id.as_str()) == space_mut.active.as_deref()).unwrap_or(0);
            let step = if command == Command::SelectNextTab { 1 } else { count - 1 };
            space_mut.active = Some(space_mut.tabs[(at + step) % count].id.clone());
            app.rebuild_rows();
            Ok(())
        }
        Command::MoveSpaceUp | Command::MoveSpaceDown => {
            let Some(id) = current_space(app) else { return Ok(()) };
            let at = app.model.active_index().unwrap_or(0);
            let index = if command == Command::MoveSpaceUp { at.saturating_sub(1) } else { at + 1 };
            commit(core, app, Mutation::MoveSpace { id, index })
        }
        Command::MoveTabUp | Command::MoveTabDown => {
            let Some(tab) = app.model.active_tab().map(|tab| tab.id.clone()) else { return Ok(()) };
            let Some(space) = current_space(app) else { return Ok(()) };
            let at = app.model.space(&space).and_then(|space| space.tabs.iter().position(|item| item.id == tab)).unwrap_or(0);
            let index = if command == Command::MoveTabUp { at.saturating_sub(1) } else { at + 1 };
            commit(core, app, Mutation::MoveTab { id: tab, index })
        }
    }
}

fn handle_prefix_key(core: &std::sync::Arc<hc::Core>, app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    if key.code == KeyCode::Esc {
        app.prefix = false;
        return Ok(false);
    }
    app.prefix = false;
    let command = match (key.code, key.modifiers) {
        (KeyCode::Char('o'), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(Command::TogglePanel),
        (KeyCode::Char('c'), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(Command::NewTab),
        (KeyCode::Char('w'), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(Command::NewSpace),
        (KeyCode::Char('r'), KeyModifiers::NONE) => Some(Command::RenameTab),
        (KeyCode::Char('R'), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(Command::RenameSpace),
        (KeyCode::Char('x'), KeyModifiers::NONE) => Some(Command::CloseTab),
        (KeyCode::Char('X'), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(Command::CloseSpace),
        _ => None,
    };
    if let Some(command) = command {
        execute_command(core, app, command)?;
    }
    Ok(false)
}

fn select_tab_index(app: &mut App, index: usize) {
    let Some(row) = app.rows.get(index) else { return };
    let id = row.id.clone();
    app.model.select_tab(&id);
    app.cursor = index;
    app.rebuild_rows();
}

fn move_agent(app: &mut App, up: bool) {
    let targets = view::agent_targets(app);
    let Some(last) = targets.len().checked_sub(1) else { return };
    let current = app
        .selected()
        .and_then(|row| row.address.as_deref())
        .and_then(|address| targets.iter().position(|target| target == address));
    let next = match current {
        None if up => last,
        None => 0,
        Some(at) if up => at.saturating_sub(1),
        Some(at) => (at + 1).min(last),
    };
    app.activate_agent(&targets[next]);
}

fn handle_prompt_key(core: &std::sync::Arc<hc::Core>, app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    let confirmation = match app.prompt.as_ref() {
        Some(Prompt::ConfirmClose { command, .. }) => Some(*command),
        _ => None,
    };
    if let Some(command) = confirmation {
        match key.code {
            KeyCode::Char('y' | 'Y') => {
                app.prompt = None;
                return run_command(core, app, command).map(|()| false);
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                app.prompt = None;
                return Ok(false);
            }
            _ => return Ok(false),
        }
    }
    match key.code {
        KeyCode::Esc => app.prompt = None,
        KeyCode::Enter => {
            let Some(prompt) = app.prompt.take() else { return Ok(false) };
            match prompt {
                Prompt::RenameSpace { id, buffer } => {
                    commit(core, app, Mutation::RenameSpace { id, name: buffer })?
                }
                Prompt::RenameTab { id, buffer } => {
                    commit(core, app, Mutation::RenameTab { id, name: buffer })?
                }
                Prompt::ConfirmClose { .. } => {}
            }
        }
        KeyCode::Backspace => {
            if let Some(buffer) = app.prompt_buffer() {
                buffer.pop();
            }
        }
        KeyCode::Char(character)
            if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
        {
            if let Some(buffer) = app.prompt_buffer() {
                buffer.push(character);
            }
        }
        _ => {}
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

/// Move the panel highlight by one whole two-row item block. A block is the
/// unit of both selection and clipping, so a wheel notch or an arrow key can
/// never stop halfway through an item.
fn move_panel_block(app: &mut App, down: bool) {
    app.move_cursor(if down { 1 } else { -1 });
}

/// Route a touch or pointer event by zone. A tap keeps its old meaning: open
/// the sheet, select a row, or resume the live pane. Movement becomes scroll.
fn handle_pointer(
    core: &std::sync::Arc<hc::Core>,
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
                        if let Some(address) = z.panel.and_then(|p| view::panel_agent_at(app, p, x, y, z.wide)) {
                            app.activate_agent(&address);
                            if !z.wide {
                                app.panel = false;
                                resume_live(app, attached);
                            }
                            return Ok(());
                        }
                        if let Some(space) = z.panel.and_then(|p| view::panel_space_at(app, p, x, y, z.wide)) {
                            select_space(app, &space);
                            return Ok(());
                        }
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
                Some(Zone::Panel) => move_panel_block(app, !up),
                Some(Zone::Terminal) => {
                    // A program that tracks the mouse owns the wheel: forward the
                    // notch instead of stealing it for herdr's copy mode.
                    let forwarded = attached.as_mut().is_some_and(|session| {
                        session.wheel(core, up, x.saturating_sub(z.terminal.x), y.saturating_sub(z.terminal.y))
                    });
                    if !forwarded {
                        scroll_history(app, attached, up);
                    }
                }
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
                Some(Zone::Panel) => move_panel_block(app, !up),
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
        // A terminal can disappear between `terminal.list` and this preview.
        // That is a normal race, not a reason to leave the screen.
        if let Ok(read) = core.call(
            "terminal.read",
            json!({ "terminal": address, "lines": app::PREVIEW_LINES }),
        ) && let Some(text) = read.get("text").and_then(Value::as_str)
        {
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
    fn selecting_a_space_is_local_and_rebuilds_the_tabs() {
        let mut app = App::new("default", "local");
        app.set_workspace(&json!({
            "version": 7,
            "value": { "spaces": [
                { "id": "s1", "name": "alpha", "folder": "", "tabs": [
                    { "id": "tb1", "name": "one", "root": { "id": "tb1", "type": "row", "children": [] } }
                ] },
                { "id": "s2", "name": "beta", "folder": "", "tabs": [
                    { "id": "tb2", "name": "two", "root": { "id": "tb2", "type": "row", "children": [] } },
                    { "id": "tb3", "name": "three", "root": { "id": "tb3", "type": "row", "children": [] } }
                ] }
            ] }
        }));
        assert_eq!(app.doc_version, Some(7));
        assert_eq!(app.model.active.as_deref(), Some("s1"));

        select_space(&mut app, "s2");

        assert_eq!(app.model.active.as_deref(), Some("s2"));
        assert_eq!(app.doc_version, Some(7), "no core write for a view choice");
        let tabs: Vec<&str> = app.rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(tabs, ["two", "three"]);
    }

    #[test]
    fn panel_movement_steps_one_whole_block_at_a_time() {
        let mut app = App::new("default", "local");
        app.set_terminals(&json!({
            "terminals": [
                { "address": "t1", "name": "one", "status": { "state": "idle" } },
                { "address": "t2", "name": "two", "status": { "state": "idle" } },
                { "address": "t3", "name": "three", "status": { "state": "idle" } }
            ],
            "hidden_finished": 0,
            "machines": [],
        }));
        app.toggle_panel();
        assert_eq!(app.cursor, 0);
        move_panel_block(&mut app, true);
        assert_eq!(app.cursor, 1, "one wheel notch or key moves one two-row block");
        move_panel_block(&mut app, true);
        assert_eq!(app.cursor, 2);
        move_panel_block(&mut app, false);
        assert_eq!(app.cursor, 1);
        move_panel_block(&mut app, false);
        move_panel_block(&mut app, false);
        assert_eq!(app.cursor, 0, "never lands between blocks");
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
