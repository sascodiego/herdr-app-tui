//! The screen's state, with no terminal and no I/O: everything here is testable.

use serde_json::Value;

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

/// Where the keyboard is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// Typing goes to the terminal, as usual.
    Terminal,
    /// The panel is open and the arrows move through its rows.
    Panel,
}

/// Everything the view draws.
#[derive(Debug)]
pub struct App {
    /// The herdr-core session this screen is attached to.
    pub session: String,
    /// What the core calls itself.
    pub core: String,
    pub connected: bool,
    /// Whether the panel is open.
    pub panel: bool,
    /// The row the panel highlights, as an index into [`App::rows`].
    pub cursor: usize,
    pub terminals: Vec<Terminal>,
    pub hidden_finished: u64,
}

impl App {
    pub fn new(session: &str, core: &str) -> App {
        App {
            session: session.to_string(),
            core: core.to_string(),
            connected: true,
            panel: false,
            cursor: 0,
            terminals: Vec::new(),
            hidden_finished: 0,
        }
    }

    /// Replace the terminal list from a `terminal.list` result.
    pub fn set_terminals(&mut self, list: &Value) {
        let entries = list.get("terminals").and_then(Value::as_array).cloned().unwrap_or_default();
        self.terminals = entries.iter().map(Terminal::from_value).collect();
        self.hidden_finished = list.get("hidden_finished").and_then(Value::as_u64).unwrap_or(0);
        self.clamp_cursor();
    }

    /// Keep the preview of one terminal, trimmed to [`PREVIEW_LINES`].
    pub fn set_preview(&mut self, address: &str, text: &str) {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let start = lines.len().saturating_sub(PREVIEW_LINES);
        let tail = lines[start..].to_vec();
        if let Some(terminal) = self.terminals.iter_mut().find(|terminal| terminal.address == address) {
            terminal.preview = tail;
        }
    }

    /// The rows the panel lists: terminals with no parent here, most in need of
    /// the person first.
    pub fn rows(&self) -> Vec<&Terminal> {
        let mut roots: Vec<&Terminal> = self
            .terminals
            .iter()
            .filter(|terminal| match &terminal.parent {
                Some(parent) => !self.terminals.iter().any(|other| &other.address == parent),
                None => true,
            })
            .collect();
        roots.sort_by_key(|terminal| (terminal.state.priority(), terminal.address.clone()));
        roots
    }

    /// The terminals started by `address`, in the order they were created.
    pub fn children_of(&self, address: &str) -> Vec<&Terminal> {
        self.terminals.iter().filter(|terminal| terminal.parent.as_deref() == Some(address)).collect()
    }

    /// The highlighted row.
    pub fn selected(&self) -> Option<&Terminal> {
        self.rows().get(self.cursor).copied()
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
        let last = self.rows().len().saturating_sub(1);
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
        let order: Vec<&str> = app.rows().iter().map(|row| row.address.as_str()).collect();
        assert_eq!(order, ["t2", "t3", "t1"], "blocked, then working, then idle");
    }

    #[test]
    fn a_helper_nests_under_the_terminal_that_started_it() {
        let app = app();
        let rows: Vec<&str> = app.rows().iter().map(|row| row.address.as_str()).collect();
        assert!(!rows.contains(&"t4"), "a helper is not a top-level row: {rows:?}");
        let children: Vec<&str> = app.children_of("t3").iter().map(|row| row.address.as_str()).collect();
        assert_eq!(children, ["t4"]);
    }

    #[test]
    fn a_child_whose_parent_is_gone_becomes_a_top_level_row() {
        let mut app = App::new("default", "local");
        app.set_terminals(&list(json!([{ "address": "t9", "parent": "gone" }])));
        let rows: Vec<&str> = app.rows().iter().map(|row| row.address.as_str()).collect();
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
        assert_eq!(app.selected().map(|row| row.address.as_str()), Some("t1"));
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
}
