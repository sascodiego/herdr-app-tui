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
pub mod mirror;
pub mod mods;
pub mod model;
pub mod persist;
pub mod spawn;
pub mod tui;
