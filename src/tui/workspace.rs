//! Core-authoritative writes to the shared herdr-app workspace document.
//!
//! The TUI never writes a document directly. It calls `doc.get`/`doc.set`,
//! follows herdr-app's compare-and-swap contract, and re-applies a mutation
//! once after a conflict, exactly like herdr-app's `mirror_sync`.

use crate::mirror::Document;
use crate::model::Model;
use anyhow::{Context, Result};
use serde_json::{Value, json};

/// The subset of `hc::Core` this module needs, kept small for deterministic tests.
pub trait CoreCalls {
    fn call(&self, method: &str, params: Value) -> Result<Value>;
}

impl CoreCalls for std::sync::Arc<crate::hc::Core> {
    fn call(&self, method: &str, params: Value) -> Result<Value> {
        crate::hc::Core::call(self, method, params)
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
