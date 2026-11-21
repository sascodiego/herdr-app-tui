//! Core-authoritative writes to the shared herdr-app workspace document.
//!
//! The TUI never writes a document directly. It calls `doc.get`/`doc.set`,
//! follows herdr-app's compare-and-swap contract, and re-applies a mutation
//! once after a conflict, exactly like herdr-app's `mirror_sync`.

use crate::mirror::Document;
use crate::model::{Model, Tab};
use anyhow::{Context, Result};
use serde_json::{Value, json};

/// The subset of `hc::Core` this module needs, kept small for deterministic tests.
pub trait CoreCalls {
    fn call(&self, method: &str, params: Value) -> Result<Value>;

    fn run(&self, params: Value) -> Result<Value> {
        self.call("terminal.run", params)
    }

    fn list(&self) -> Result<Value> {
        self.call("terminal.list", json!({}))
    }
}

impl CoreCalls for std::sync::Arc<crate::hc::Core> {
    fn call(&self, method: &str, params: Value) -> Result<Value> {
        crate::hc::Core::call(self, method, params)
    }
}

/// Commands available from the sidebar action row and the `:` palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    NewSpace,
    NewTab,
    RenameSpace,
    RenameTab,
    CloseTab,
    CloseSpace,
    SelectNextSpace,
    SelectPrevSpace,
    SelectNextTab,
    SelectPrevTab,
    MoveSpaceUp,
    MoveSpaceDown,
    MoveTabUp,
    MoveTabDown,
}

impl Command {
    pub fn parse(line: &str) -> Option<Self> {
        match line.trim().to_ascii_lowercase().as_str() {
            "new space" | "space new" => Some(Self::NewSpace),
            "new tab" | "tab new" => Some(Self::NewTab),
            "rename space" | "space rename" => Some(Self::RenameSpace),
            "rename tab" | "tab rename" => Some(Self::RenameTab),
            "close tab" | "tab close" => Some(Self::CloseTab),
            "close space" | "space close" => Some(Self::CloseSpace),
            "space next" => Some(Self::SelectNextSpace),
            "space prev" => Some(Self::SelectPrevSpace),
            "tab next" => Some(Self::SelectNextTab),
            "tab prev" => Some(Self::SelectPrevTab),
            "move space up" => Some(Self::MoveSpaceUp),
            "move space down" => Some(Self::MoveSpaceDown),
            "move tab up" => Some(Self::MoveTabUp),
            "move tab down" => Some(Self::MoveTabDown),
            _ => None,
        }
    }
}

/// A user-visible change to the shared workspace model.
///
/// Terminal addresses are returned by close operations so the caller can ask
/// the core to end them. This type never touches a terminal itself.
#[derive(Clone, Debug, PartialEq)]
pub enum Mutation {
    NewSpace { id: String, tab_id: String, pane_id: String, name: String, folder: String, tab_name: String, address: String },
    NewTab { space: String, id: String, pane_id: String, name: String, address: String },
    RenameSpace { id: String, name: String },
    RenameTab { id: String, name: String },
    SelectSpace { id: String },
    SelectTab { id: String },
    MoveSpace { id: String, index: usize },
    MoveTab { id: String, index: usize },
    CloseTab { id: String },
    CloseSpace { id: String },
}

impl Mutation {
    /// Allocate ids for a space with one tab and its first terminal.
    pub fn new_space(model: &mut Model, name: String, folder: String, tab_name: String, address: String) -> Self {
        Self::NewSpace {
            id: model.fresh_id_on("sp", None),
            tab_id: model.fresh_id_on("tb", None),
            pane_id: model.fresh_id_on("p", None),
            name,
            folder,
            tab_name,
            address,
        }
    }

    /// Allocate ids for a tab, or create a space when none is active.
    pub fn new_tab(model: &mut Model, space: Option<String>, name: String, address: String) -> Self {
        let Some(space) = space.or_else(|| model.active.clone()) else {
            return Self::new_space(model, String::new(), String::new(), name, address);
        };
        Self::NewTab {
            space,
            id: model.fresh_id_on("tb", None),
            pane_id: model.fresh_id_on("p", None),
            name,
            address,
        }
    }

    /// Fill in the terminal address returned by the core.
    pub fn with_address(self, address: String) -> Self {
        match self {
            Self::NewSpace { id, tab_id, pane_id, name, folder, tab_name, .. } => {
                Self::NewSpace { id, tab_id, pane_id, name, folder, tab_name, address }
            }
            Self::NewTab { space, id, pane_id, name, .. } => {
                Self::NewTab { space, id, pane_id, name, address }
            }
            other => other,
        }
    }

    /// Apply once to a model. The same closure can be replayed after a CAS conflict.
    pub fn apply(&self, model: &mut Model) -> Vec<String> {
        match self {
            Self::NewSpace { id, tab_id, pane_id, name, folder, tab_name, address } => {
                model.add_space(id, name, folder, Tab::new(tab_id, tab_name, pane_id, address));
                Vec::new()
            }
            Self::NewTab { space, id, pane_id, name, address } => {
                model.add_tab(space, Tab::new(id, name, pane_id, address));
                Vec::new()
            }
            Self::RenameSpace { id, name } => {
                if let Some(space) = model.space_mut(id) {
                    space.name = name.clone();
                }
                Vec::new()
            }
            Self::RenameTab { id, name } => {
                if let Some(tab) = model.tab_mut(id) {
                    tab.name = name.clone();
                }
                Vec::new()
            }
            Self::SelectSpace { id } => {
                if model.space(id).is_some() {
                    model.active = Some(id.clone());
                }
                Vec::new()
            }
            Self::SelectTab { id } => {
                model.select_tab(id);
                Vec::new()
            }
            Self::MoveSpace { id, index } => {
                model.move_space(id, *index);
                Vec::new()
            }
            Self::MoveTab { id, index } => {
                model.move_tab(id, *index);
                Vec::new()
            }
            Self::CloseTab { id } => model.remove_tab(id).0,
            Self::CloseSpace { id } => model.remove_space(id),
        }
    }
}

/// Ask the core to start a shell for a new space or tab.
pub fn launch<C: CoreCalls + ?Sized>(
    core: &C,
    folder: Option<&str>,
    cols: u16,
    rows: u16,
    space: &str,
    tab: &str,
) -> Result<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let reply = core.run(json!({
        "argv": [shell],
        "cwd": folder,
        "env": { "TERM": "xterm-256color", "COLORTERM": "truecolor", "TERM_PROGRAM": "herdr" },
        "labels": { "herdr.space": space, "herdr.tab": tab },
        "on": null,
        "keep": true,
        "display": {},
        "size": { "cols": cols, "rows": rows },
    }))?;
    reply
        .get("address")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .context("terminal.run returned no address")
}

/// End a terminal and its child tree through the core, sparing kept agents.
pub fn kill_tree<C: CoreCalls + ?Sized>(core: &C, addresses: &[String]) -> Result<()> {
    let list = core.list()?;
    let terminals = list.get("terminals").and_then(Value::as_array);
    let mut doomed = addresses.to_vec();
    let mut index = 0;
    while index < doomed.len() {
        if let Some(terminals) = terminals {
            for terminal in terminals {
                let Some(address) = terminal.get("address").and_then(Value::as_str) else { continue };
                let kept = terminal
                    .get("labels")
                    .and_then(|labels| labels.get("herdr.space"))
                    .and_then(Value::as_str)
                    .is_some_and(|space| !space.is_empty());
                let child = terminal
                    .get("parent")
                    .and_then(Value::as_str)
                    .is_some_and(|parent| doomed.iter().any(|doomed| doomed == parent));
                if child && !kept && !doomed.iter().any(|doomed| doomed == address) {
                    doomed.push(address.to_string());
                }
            }
        }
        index += 1;
    }

    let mut first_error = None;
    for address in doomed {
        if let Err(error) = core.call("terminal.kill", json!({ "terminal": address }))
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    match first_error {
        Some(error) => Err(error.context("cannot end a terminal tree")),
        None => Ok(()),
    }
}

fn write_params(document: &Document, version: u64, machine: Option<&str>) -> Result<Value> {
    Ok(json!({
        "name": "herdr-app",
        "on": machine,
        "version": version,
        "value": serde_json::to_value(document).context("cannot serialize the workspace")?,
    }))
}

fn version_of(reply: &Value) -> Result<u64> {
    reply
        .get("version")
        .and_then(Value::as_u64)
        .context("layout write returned no version")
}

fn read<C: CoreCalls + ?Sized>(core: &C, machine: Option<&str>) -> Result<(Document, u64)> {
    let reply = core.call("doc.get", json!({ "name": "herdr-app", "on": machine }))?;
    let version = reply.get("version").and_then(Value::as_u64).context("layout document has no version")?;
    let document = match reply.get("value") {
        Some(value) if !value.is_null() => serde_json::from_value(value.clone()).context("cannot read the workspace document")?,
        _ => Document::default(),
    };
    Ok((document, version))
}

/// Apply a mutation to the shared document with herdr-app's CAS contract.
///
/// The change is replayed once on a freshly read document after a conflict.
/// On success `model` holds the accepted document; on a later conflict or any
/// other write error the caller should refresh from the core before retrying.
pub fn apply<C, F>(core: &C, model: &mut Model, version: u64, machine: Option<&str>, change: F) -> Result<u64>
where
    C: CoreCalls + ?Sized,
    F: Fn(&mut Model) + Clone,
{
    change(model);
    let params = write_params(&Document::capture(model, machine), version, machine)?;
    match core.call("doc.set", params) {
        Ok(reply) => return version_of(&reply),
        Err(error) if crate::hc::error_kind(&error) == Some("conflict") => {}
        Err(error) => return Err(error.context("cannot write the workspace document")),
    }

    let (fresh, version) = read(core, machine)?;
    let mut replayed = Model::default();
    fresh.replace(&mut replayed, machine);
    change(&mut replayed);
    let params = write_params(&Document::capture(&replayed, machine), version, machine)?;
    let accepted = core
        .call("doc.set", params)
        .context("cannot write the reloaded workspace document")?;
    let version = version_of(&accepted)?;
    *model = replayed;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Model;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    struct Fake {
        calls: RefCell<Vec<(String, Value)>>,
        replies: RefCell<VecDeque<Result<Value>>>,
    }

    impl Fake {
        fn new(replies: Vec<Result<Value>>) -> Self {
            Self { calls: RefCell::new(Vec::new()), replies: RefCell::new(replies.into()) }
        }
    }

    impl CoreCalls for Fake {
        fn call(&self, method: &str, params: Value) -> Result<Value> {
            self.calls.borrow_mut().push((method.to_string(), params));
            self.replies
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok(json!({ "version": 2 })))
        }
    }

    fn conflict() -> anyhow::Error {
        anyhow::Error::new(crate::hc::RpcError(json!({ "data": { "kind": "conflict" } })))
    }

    #[test]
    fn a_conflict_reloads_and_reapplies_once() {
        let core = Fake::new(vec![
            Err(conflict()),
            Ok(json!({ "value": { "spaces": [] }, "version": 7 })),
            Ok(json!({ "version": 8 })),
        ]);
        let mut model = Model::default();
        let applied = Cell::new(0);
        let change = |_: &mut Model| applied.set(applied.get() + 1);

        let version = apply(&core, &mut model, 3, None, change).expect("retry succeeds");

        let calls = core.calls.borrow();
        assert_eq!(calls[0].0, "doc.set");
        assert_eq!(calls[0].1["version"], 3);
        assert_eq!(calls[1].0, "doc.get");
        assert_eq!(calls[2].0, "doc.set");
        assert_eq!(calls[2].1["version"], 7);
        assert_eq!(version, 8);
        assert_eq!(applied.get(), 2, "the mutation is replayed on the fresh model");
    }

    #[test]
    fn palette_commands_parse_from_typed_text() {
        assert_eq!(Command::parse("  New Space "), Some(Command::NewSpace));
        assert_eq!(Command::parse("tab close"), Some(Command::CloseTab));
        assert_eq!(Command::parse("move tab down"), Some(Command::MoveTabDown));
        assert_eq!(Command::parse("delete everything"), None);
    }

    #[test]
    fn a_new_space_opens_with_one_named_tab() {
        let mut model = Model::default();
        Mutation::new_space(&mut model, "research".into(), "/src".into(), "review".into(), "t1".into())
            .apply(&mut model);

        assert_eq!(model.spaces.len(), 1);
        let space = &model.spaces[0];
        assert_eq!(space.name, "research");
        assert_eq!(space.folder, "/src");
        assert_eq!(space.tabs.len(), 1);
        assert_eq!(space.tabs[0].name, "review");
        assert_eq!(model.active.as_deref(), Some(space.id.as_str()));
    }

    #[test]
    fn close_returns_the_terminal_addresses_for_the_core_to_end() {
        let mut model = Model::default();
        Mutation::new_space(&mut model, "s".into(), "/tmp".into(), "a".into(), "t1".into())
            .apply(&mut model);
        let space = model.spaces[0].id.clone();
        Mutation::new_tab(&mut model, Some(space.clone()), "b".into(), "t2".into())
            .apply(&mut model);

        let tab = model.spaces[0].tabs[1].id.clone();
        assert_eq!(Mutation::CloseTab { id: tab }.apply(&mut model), vec!["t2".to_string()]);
        assert_eq!(Mutation::CloseSpace { id: space }.apply(&mut model), vec!["t1".to_string()]);
        assert!(model.spaces.is_empty());
    }

    #[test]
    fn rename_select_and_move_operate_on_the_shared_model() {
        let mut model = Model::default();
        Mutation::new_space(&mut model, "first".into(), "/tmp".into(), "a".into(), "t1".into())
            .apply(&mut model);
        let space = model.spaces[0].id.clone();
        let tab = model.spaces[0].tabs[0].id.clone();

        Mutation::RenameSpace { id: space.clone(), name: "renamed".into() }.apply(&mut model);
        Mutation::RenameTab { id: tab.clone(), name: "work".into() }.apply(&mut model);
        Mutation::SelectTab { id: tab }.apply(&mut model);
        Mutation::MoveSpace { id: space.clone(), index: 0 }.apply(&mut model);

        assert_eq!(model.spaces[0].name, "renamed");
        assert_eq!(model.spaces[0].tabs[0].name, "work");
        assert_eq!(model.active.as_deref(), Some(space.as_str()));
    }

    #[test]
    fn launch_asks_the_core_for_a_terminal() {
        let core = Fake::new(vec![Ok(json!({ "address": "t9" }))]);
        let address = launch(&core, Some("/src"), 40, 11, "sp1", "tb1")
            .expect("the core starts the terminal");
        assert_eq!(address, "t9");
        let calls = core.calls.borrow();
        assert_eq!(calls[0].0, "terminal.run");
        assert_eq!(calls[0].1["cwd"], "/src");
        assert_eq!(calls[0].1["size"]["cols"], 40);
        assert_eq!(calls[0].1["size"]["rows"], 11);
        assert_eq!(calls[0].1["labels"]["herdr.space"], "sp1");
        assert_eq!(calls[0].1["labels"]["herdr.tab"], "tb1");
    }

    #[test]
    fn kill_tree_follows_children_and_spares_kept_agents() {
        let core = Fake::new(vec![
            Ok(json!({ "terminals": [
                { "address": "t1" },
                { "address": "t2", "parent": "t1" },
                { "address": "t3", "parent": "t1", "labels": { "herdr.space": "kept" } },
                { "address": "t4", "parent": "t2" },
            ] })),
            Ok(json!({})), Ok(json!({})), Ok(json!({})),
        ]);
        kill_tree(&core, &["t1".to_string()]).expect("kill asks the core");
        let calls = core.calls.borrow();
        assert_eq!(calls[0].0, "terminal.list");
        let killed: Vec<&str> = calls[1..].iter().map(|call| call.1["terminal"].as_str().unwrap_or_default()).collect();
        assert_eq!(killed, ["t1", "t2", "t4"]);
    }

    #[test]
    fn other_errors_do_not_retry() {
        let core = Fake::new(vec![Err(anyhow::anyhow!("offline"))]);
        let mut model = Model::default();
        let applied = Cell::new(0);
        let error = apply(&core, &mut model, 1, None, |_: &mut Model| applied.set(applied.get() + 1))
            .expect_err("offline is not a conflict");

        assert!(format!("{error:#}").contains("offline"), "{error:#}");
        assert_eq!(core.calls.borrow().len(), 1);
        assert_eq!(applied.get(), 1);
    }
}
