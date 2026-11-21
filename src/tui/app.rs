//! The screen's state, with no terminal and no I/O: everything here is testable.

use crate::mirror::Document;
use crate::model::Model;
use serde_json::Value;

use super::view::ScreenState;

use super::glyph::State;

/// How many lines of a terminal's output the preview keeps.
pub const PREVIEW_LINES: usize = 6;

/// One terminal, as the panel shows it.
#[derive(Clone, Debug)]
pub struct Terminal {
    pub address: String,
    pub name: String,
    pub state: State,
    /// The terminal that started it, when there is one: it nests under that row.
    pub parent: Option<String>,
    /// The tail of its output, for the preview under the list.
    pub preview: Vec<String>,
}

impl Terminal {
    /// Read one entry of `terminal.list`, tolerating fields this build doesn't know.
    pub fn from_value(value: &Value) -> Terminal {
        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
        let address = text("address").or_else(|| text("id")).unwrap_or_else(|| "?".to_string());
        let name = text("name").unwrap_or_else(|| address.clone());
        let word = value
            .get("status")
            .and_then(|status| status.get("state"))
            .or_else(|| value.get("state"))
            .and_then(Value::as_str);
        Terminal {
            state: State::from_word(word),
            parent: text("parent"),
            preview: Vec::new(),
            address,
            name,
        }
    }
}

/// One tab of the active space, as the panel lists it. A tab owns its
/// terminals through its layout, so its address is the first one for preview
/// and attach; a tab with no terminal yet has none.
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub state: State,
    pub preview: Vec<String>,
}

/// Where the keyboard is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// Typing goes to the terminal, as usual.
    Terminal,
    /// The panel is open and the arrows move through its rows.
    Panel,
}

/// Everything the view draws.
pub struct App {
    /// The herdr-core session this screen is attached to.
    pub session: String,
    /// What the core calls itself.
    pub core: String,
    /// The directory the TUI was launched from, shown in the mobile band.
    pub path: String,
    pub connected: bool,
    /// Whether the panel is open.
    pub panel: bool,
    /// The row the panel highlights, as an index into [`App::rows`].
    pub cursor: usize,
    pub terminals: Vec<Terminal>,
    pub hidden_finished: u64,
    /// The shared workspace document, the same data herdr-app reads.
    pub model: Model,
    /// Whether the core answered `doc.get`, as opposed to falling back to the
    /// terminal list because the call failed.
    pub doc_available: bool,
    /// The document's compare-and-swap version, kept for a future write.
    pub doc_version: Option<u64>,
    /// Why the document could not be read, when it could not.
    pub doc_error: Option<String>,
    /// The tabs the panel lists, derived from the document's active space.
    pub rows: Vec<Row>,
    /// The attached screen, when a terminal is being viewed live.
    pub screen: Option<ScreenState>,
    /// Whether the screen shows libghostty's scrollback instead of following.
    pub copy: bool,
}

impl App {
    pub fn new(session: &str, core: &str) -> App {
        App {
            session: session.to_string(),
            core: core.to_string(),
            path: std::env::current_dir()
                .map(|directory| directory.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            connected: true,
            panel: false,
            cursor: 0,
            terminals: Vec::new(),
            hidden_finished: 0,
            model: Model::default(),
            doc_available: false,
            doc_version: None,
            doc_error: None,
            rows: Vec::new(),
            screen: None,
            copy: false,
        }
    }

    /// Enter copy mode. The scrollback itself lives in the attached `Vt`.
    pub fn enter_copy(&mut self) {
        self.copy = true;
    }

    /// Resume the live pane.
    pub fn exit_copy(&mut self) {
        self.copy = false;
    }

    /// Replace the terminal list from a `terminal.list` result.
    pub fn set_terminals(&mut self, list: &Value) {
        let entries = list.get("terminals").and_then(Value::as_array).cloned().unwrap_or_default();
        self.terminals = entries.iter().map(Terminal::from_value).collect();
        self.hidden_finished = list.get("hidden_finished").and_then(Value::as_u64).unwrap_or(0);
        self.rebuild_rows();
    }

    /// Replace the workspace from a `doc.get` result. The document is the
    /// source of the panel's rows; the terminal list only supplies their live
    /// state, exactly as it does in herdr-app.
    pub fn set_workspace(&mut self, result: &Value) {
        let version = result.get("version").and_then(Value::as_u64);
        if self.doc_available && version.is_some() && version == self.doc_version {
            return;
        }
        self.doc_available = result.get("value").is_some_and(|value| !value.is_null());
        let value = result.get("value").cloned().unwrap_or(Value::Null);
        let document: Document = if value.is_null() {
            Document::default()
        } else {
            match serde_json::from_value(value) {
                Ok(document) => document,
                Err(error) => {
                    self.doc_error = Some(format!("cannot read the workspace document: {error}"));
                    return;
                }
            }
        };
        document.replace(&mut self.model, None);
        if self.model.active.is_none() {
            self.model.active = self.model.spaces.first().map(|space| space.id.clone());
        }
        self.doc_version = version;
        self.doc_error = None;
        self.rebuild_rows();
    }

    /// Rebuild the panel's rows from the active space's tabs. With no
    /// workspace yet, fall back to the core's top-level terminals so a fresh
    /// core is still usable.
    fn rebuild_rows(&mut self) {
        let from_document = self.doc_available;
        let tabs: Vec<(String, String, Option<String>)> = match self.model.active_space() {
            Some(space) if from_document => space
                .tabs
                .iter()
                .map(|tab| {
                    let address = tab
                        .root
                        .terminals()
                        .first()
                        .and_then(|node| node.terminal())
                        .map(str::to_owned);
                    (tab.id.clone(), Model::tab_name(space, tab), address)
                })
                .collect(),
            _ if !from_document => self
                .terminals
                .iter()
                .filter(|terminal| match &terminal.parent {
                    Some(parent) => !self.terminals.iter().any(|other| &other.address == parent),
                    None => true,
                })
                .map(|terminal| {
                    (
                        terminal.address.clone(),
                        terminal.name.clone(),
                        Some(terminal.address.clone()),
                    )
                })
                .collect(),
            _ => Vec::new(),
        };

        self.rows = tabs
            .into_iter()
            .map(|(id, name, address)| {
                let state = address
                    .as_deref()
                    .and_then(|address| {
                        self.terminals
                            .iter()
                            .find(|terminal| terminal.address == address)
                            .map(|terminal| terminal.state)
                    })
                    .unwrap_or(State::Unknown);
                Row { id, name, address, state, preview: Vec::new() }
            })
            .collect();
        if !from_document {
            // Without a workspace, rank the core's terminals by attention.
            // Tabs keep their document order, as they do in herdr-app.
            self.rows.sort_by_key(|row| (row.state.priority(), row.name.clone()));
        }
        self.copy_previews();
        self.clamp_cursor();
    }

    /// A rebuild drops previews; carry over what the core already told us.
    fn copy_previews(&mut self) {
        for row in &mut self.rows {
            let Some(address) = row.address.as_deref() else { continue };
            if let Some(terminal) = self.terminals.iter().find(|terminal| terminal.address == address) {
                row.preview = terminal.preview.clone();
                if row.state == State::Unknown {
                    row.state = terminal.state;
                }
            }
        }
    }

    /// Keep the preview of one terminal, trimmed to [`PREVIEW_LINES`].
    pub fn set_preview(&mut self, address: &str, text: &str) {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let start = lines.len().saturating_sub(PREVIEW_LINES);
        let tail = lines[start..].to_vec();
        if let Some(terminal) = self.terminals.iter_mut().find(|terminal| terminal.address == address) {
            terminal.preview = tail.clone();
        }
        if let Some(row) = self.rows.iter_mut().find(|row| row.address.as_deref() == Some(address)) {
            row.preview = tail;
        }
    }

    /// The tabs of the active workspace, as the panel lists them.
    pub fn rows(&self) -> Vec<&Row> {
        self.rows.iter().collect()
    }

    /// The terminals started by `address`, in the order they were created.
    pub fn children_of(&self, address: &str) -> Vec<&Terminal> {
        self.terminals.iter().filter(|terminal| terminal.parent.as_deref() == Some(address)).collect()
    }

    /// The workspace named in the band: the highlighted tab, or the session.
    pub fn workspace(&self) -> &str {
        self.selected().map_or(self.session.as_str(), |row| row.name.as_str())
    }

    /// The highlighted row.
    pub fn selected(&self) -> Option<&Row> {
        self.rows.get(self.cursor)
    }

    /// Move the highlight by `delta` rows, staying inside the list.
    pub fn move_cursor(&mut self, delta: isize) {
        let last = self.rows().len().saturating_sub(1);
        let moved = self.cursor as isize + delta;
        self.cursor = moved.clamp(0, last as isize) as usize;
    }

    /// Open or close the panel. Opening it takes the highlight from the terminal.
    pub fn toggle_panel(&mut self) {
        self.panel = !self.panel;
        if self.panel {
            self.clamp_cursor();
        }
    }

    /// Where the keyboard is right now.
    pub fn focus(&self) -> Focus {
        if self.panel { Focus::Panel } else { Focus::Terminal }
    }

    /// `(working, needs you, idle)` for the agents rollup.
    pub fn counts(&self) -> (usize, usize, usize) {
        let count = |state: State| self.terminals.iter().filter(|terminal| terminal.state == state).count();
        (count(State::Working), count(State::NeedsYou) + count(State::Failed), count(State::Idle))
    }

    fn clamp_cursor(&mut self) {
        let last = self.rows.len().saturating_sub(1);
        if self.cursor > last {
            self.cursor = last;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn list(terminals: Value) -> Value {
        json!({ "terminals": terminals, "hidden_finished": 2, "machines": [] })
    }

    fn app() -> App {
        let mut app = App::new("default", "local");
        app.set_terminals(&list(json!([
            { "address": "t1", "name": "pi 1", "status": { "state": "idle" } },
            { "address": "t2", "name": "pi 2", "status": { "state": "blocked" } },
            { "address": "t3", "name": "pi 3", "status": { "state": "working" } },
            { "address": "t4", "name": "helper", "status": { "state": "idle" }, "parent": "t3" }
        ])));
        app
    }

    #[test]
    fn a_terminal_reads_its_address_name_and_state() {
        let terminal = Terminal::from_value(&json!({
            "address": "t7", "name": "pi", "status": { "state": "working" }
        }));
        assert_eq!((terminal.address.as_str(), terminal.name.as_str()), ("t7", "pi"));
        assert_eq!(terminal.state, State::Working);
    }

    #[test]
    fn a_terminal_without_a_name_is_known_by_its_address() {
        let terminal = Terminal::from_value(&json!({ "address": "t7" }));
        assert_eq!(terminal.name, "t7");
        assert_eq!(terminal.state, State::Unknown);
    }

    #[test]
    fn whoever_needs_the_person_is_listed_first() {
        let app = app();
        let order: Vec<&str> = app.rows().iter().map(|row| row.address.as_deref().unwrap_or("?")).collect();
        assert_eq!(order, ["t2", "t3", "t1"], "blocked, then working, then idle");
    }

    #[test]
    fn a_helper_nests_under_the_terminal_that_started_it() {
        let app = app();
        let rows: Vec<&str> = app.rows().iter().map(|row| row.address.as_deref().unwrap_or("?")).collect();
        assert!(!rows.contains(&"t4"), "a helper is not a top-level row: {rows:?}");
        let children: Vec<&str> = app.children_of("t3").iter().map(|row| row.address.as_str()).collect();
        assert_eq!(children, ["t4"]);
    }

    #[test]
    fn a_child_whose_parent_is_gone_becomes_a_top_level_row() {
        let mut app = App::new("default", "local");
        app.set_terminals(&list(json!([{ "address": "t9", "parent": "gone" }])));
        let rows: Vec<&str> = app.rows().iter().map(|row| row.address.as_deref().unwrap_or("?")).collect();
        assert_eq!(rows, ["t9"]);
    }

    #[test]
    fn the_cursor_stays_inside_the_list() {
        let mut app = app();
        app.move_cursor(-5);
        assert_eq!(app.cursor, 0);
        app.move_cursor(99);
        assert_eq!(app.cursor, 2, "three top-level rows");
    }

    #[test]
    fn the_cursor_follows_a_shorter_list_without_pointing_past_it() {
        let mut app = app();
        app.move_cursor(9);
        app.set_terminals(&list(json!([{ "address": "t1" }])));
        assert_eq!(app.cursor, 0);
        assert_eq!(app.selected().and_then(|row| row.address.as_deref()), Some("t1"));
    }

    #[test]
    fn the_rollup_counts_working_needs_you_and_idle() {
        assert_eq!(app().counts(), (1, 1, 2), "t3 working, t2 blocked, t1 and t4 idle");
    }

    #[test]
    fn the_panel_holds_the_keyboard_only_while_it_is_open() {
        let mut app = app();
        assert_eq!(app.focus(), Focus::Terminal);
        app.toggle_panel();
        assert_eq!(app.focus(), Focus::Panel);
        app.toggle_panel();
        assert_eq!(app.focus(), Focus::Terminal);
    }

    #[test]
    fn a_preview_keeps_only_its_tail() {
        let mut app = app();
        let text: String = (1..=20).map(|n| format!("line {n:02}\n")).collect();
        app.set_preview("t1", &text);
        let preview = &app.terminals[0].preview;
        assert_eq!(preview.len(), PREVIEW_LINES);
        assert_eq!(preview.first().map(String::as_str), Some("line 15"));
        assert_eq!(preview.last().map(String::as_str), Some("line 20"));
    }

    #[test]
    fn the_hidden_finished_count_comes_from_the_core() {
        assert_eq!(app().hidden_finished, 2);
    }

    fn workspace(tabs: Value) -> Value {
        json!({
            "version": 5,
            "value": { "spaces": [{
                "id": "s1", "name": "work", "folder": "", "tabs": tabs
            }] }
        })
    }

    #[test]
    fn a_tab_in_the_workspace_document_becomes_a_row() {
        let mut app = app();
        app.set_workspace(&workspace(json!([
            { "id": "tb1", "name": "review", "root": { "id": "tb1", "type": "row", "children": [
                { "id": "pdf2", "type": "block", "content": { "terminal": "t10" } }
            ] } }
        ])));
        let rows: Vec<(&str, Option<&str>)> = app
            .rows()
            .iter()
            .map(|row| (row.name.as_str(), row.address.as_deref()))
            .collect();
        assert_eq!(rows, [("review", Some("t10"))]);
    }

    #[test]
    fn a_terminal_outside_the_workspace_is_not_a_row() {
        let mut app = app();
        app.set_workspace(&workspace(json!([
            { "id": "tb1", "root": { "id": "tb1", "type": "row", "children": [
                { "id": "b1", "type": "block", "content": { "terminal": "t1" } }
            ] } }
        ])));
        let rows: Vec<Option<&str>> = app.rows().iter().map(|row| row.address.as_deref()).collect();
        assert_eq!(rows, [Some("t1")], "t2 and t3 are not in any tab");
    }

    #[test]
    fn a_document_row_takes_its_state_from_its_terminal() {
        let mut app = app();
        app.set_workspace(&workspace(json!([
            { "id": "tb1", "root": { "id": "tb1", "type": "row", "children": [
                { "id": "b1", "type": "block", "content": { "terminal": "t2" } }
            ] } }
        ])));
        assert_eq!(app.rows()[0].state, State::NeedsYou, "t2 is blocked in the core");
    }

    #[test]
    fn an_unchanged_document_version_does_not_rebuild_the_workspace() {
        let mut app = app();
        app.set_workspace(&workspace(json!([
            { "id": "tb1", "name": "first" }
        ])));
        app.set_workspace(&json!({
            "version": 5,
            "value": { "spaces": [{ "tabs": [{ "id": "tb2", "name": "second" }] }] }
        }));
        let names: Vec<&str> = app.rows().iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["first"], "the same version cannot carry new content");
    }

    #[test]
    fn a_new_document_version_rebuilds_the_workspace() {
        let mut app = app();
        app.set_workspace(&workspace(json!([{ "id": "tb1", "name": "first" }])));
        app.set_workspace(&json!({
            "version": 6,
            "value": { "spaces": [{ "tabs": [{ "id": "tb2", "name": "second" }] }] }
        }));
        let names: Vec<&str> = app.rows().iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["second"]);
    }

    #[test]
    fn an_empty_workspace_has_no_rows() {
        let mut app = app();
        app.set_workspace(&json!({ "version": 6, "value": { "spaces": [] } }));
        assert!(app.rows().is_empty());
        assert_eq!(app.doc_version, Some(6));
    }

    #[test]
    fn copy_mode_only_tracks_whether_to_follow_the_live_pane() {
        let mut app = app();
        assert!(!app.copy);
        app.enter_copy();
        assert!(app.copy, "the scrollback lives in Vt, not in App");
        app.exit_copy();
        assert!(!app.copy);
    }

    #[test]
    fn only_the_active_space_tabs_become_rows() {
        let mut app = app();
        app.set_workspace(&json!({
            "version": 5,
            "value": { "spaces": [
                { "id": "s1", "tabs": [{ "id": "tb1", "name": "first" }] },
                { "id": "s2", "tabs": [{ "id": "tb2", "name": "second" }] }
            ] }
        }));
        let names: Vec<&str> = app.rows().iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["first"], "the other space is not shown");
        assert_eq!(app.doc_version, Some(5));
    }
}
