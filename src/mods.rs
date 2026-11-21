//! Stand-in for herdr-app's `mods` module.
//!
//! The copied `core_bin.rs` asks `crate::mods::home()` where the app keeps its
//! home. herdr-app's `mods` is a 1,876-line JavaScript mod host with 34 GPUI
//! references; only this one function is needed here, so this module provides
//! exactly that and nothing else. Keeping the name means the copied file needs
//! no edit.

use std::path::PathBuf;

/// The user's home, matching herdr-app's `mods::home()`.
pub fn home() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| "/".into())
}
