//! The parts of herdr-app's `settings` that `vt.rs` reads.
//!
//! herdr-app's real settings module is 1,090 lines with nine GPUI references.
//! Only these two functions and one field are used here, so the copied `vt.rs`
//! keeps its call sites unchanged.

use std::collections::BTreeMap;

/// The one field `vt.rs` reads from the loaded settings.
pub struct Settings {
    pub minimum_contrast: f32,
}

impl Default for Settings {
    fn default() -> Settings {
        // herdr-app's shipped default (settings.json).
        Settings { minimum_contrast: 1.0 }
    }
}

/// Stand-in for herdr-app's settings store: the TUI does not load its JSON yet.
pub fn get() -> Settings {
    Settings::default()
}

/// The built-in theme colors, used when the app has no saved theme yet.
pub fn theme() -> BTreeMap<String, String> {
    let pairs = [
        ("background", "#11111b"),
        ("terminal_foreground", "#cdd6f4"),
        ("cursor", "#b4befe"),
        ("selection", "#313244"),
        ("ansi0", "#11111b"),
        ("ansi1", "#f38ba8"),
        ("ansi2", "#a6e3a1"),
        ("ansi3", "#f9e2af"),
        ("ansi4", "#b4befe"),
        ("ansi5", "#cba6f7"),
        ("ansi6", "#94e2d5"),
        ("ansi7", "#cdd6f4"),
        ("ansi8", "#585b70"),
        ("ansi9", "#f38ba8"),
        ("ansi10", "#a6e3a1"),
        ("ansi11", "#f9e2af"),
        ("ansi12", "#b4befe"),
        ("ansi13", "#cba6f7"),
        ("ansi14", "#94e2d5"),
        ("ansi15", "#eff1f5"),
    ];
    pairs.into_iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
}

/// Parse `#rrggbb`, as herdr-app's `rgb` does.
pub fn rgb(value: &str) -> super::vt::Rgb {
    let digits = value.trim_start_matches('#');
    let byte = |at: usize| {
        digits
            .get(at..at + 2)
            .and_then(|part| u8::from_str_radix(part, 16).ok())
            .unwrap_or(255)
    };
    super::vt::Rgb(byte(0), byte(2), byte(4))
}
