//! A terminal UI client for herdr-core, and the herdr-app layers it reuses.
//!
//! The client, core discovery and model modules are copied from herdr-app
//! (Apache-2.0) and keep their full public surface: the TUI uses part of it
//! today, and the terminal attach stream uses the rest. Keeping them in a
//! library target is what lets that surface exist without dead-code warnings,
//! so no lint is suppressed.

pub mod core_bin;
pub mod hc;
pub mod layout;
pub mod links;
pub mod mirror;
pub mod mods;
pub mod model;
pub mod persist;
pub mod settings;
pub mod spawn;
pub mod sprites;
pub mod vt;
pub mod win32_keys;
pub mod tui;

/// Scratch folder for copied tests, matching herdr-app's own helper.
/// D8 in UPSTREAM.md: only test code calls this.
#[cfg(test)]
pub fn test_tmp() -> std::path::PathBuf {
    std::env::var_os("HERDR_TEST_TMPDIR").map_or_else(|| "/var/tmp".into(), Into::into)
}
