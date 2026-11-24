//! Colors, from `design/system.md`.
//!
//! The canvas roles are Gotham Default — the terminal palette the herdr TUI
//! uses by default. The app chrome roles are herdr-app's neutral tokens.
//! Nothing outside this module names a color.

use ratatui::style::Color;

/// Terminal canvas.
pub const BG: Color = Color::Rgb(0x0c, 0x10, 0x14);
/// Sheet and rail surface, one step up from the canvas.
pub const PANEL: Color = Color::Rgb(0x11, 0x18, 0x20);
/// The selected row band.
pub const SURFACE: Color = Color::Rgb(0x0a, 0x37, 0x49);
/// Primary text: labels, values, agent names.
pub const TEXT: Color = Color::Rgb(0x99, 0xd1, 0xce);
/// Secondary text: section headers, counts, paths.
pub const MUTED: Color = Color::Rgb(0x99, 0xd1, 0xce);
/// Disabled text, separators, footer actions at rest.
pub const DIM: Color = Color::Rgb(0x99, 0xd1, 0xce);
/// Selection marker, focus, active tab.
pub const ACCENT: Color = Color::Rgb(0x33, 0x85, 0x9e);
/// Agent idle, connected.
pub const GREEN: Color = Color::Rgb(0x2a, 0xa8, 0x89);
/// Agent working, warning.
pub const YELLOW: Color = Color::Rgb(0xed, 0xb4, 0x43);
/// Agent blocked, error, disconnected.
pub const RED: Color = Color::Rgb(0xc2, 0x31, 0x27);

#[cfg(test)]
mod tests {
    use super::*;

    /// `design/system.md` pins these; a change here is a change to the design.
    #[test]
    fn the_palette_matches_the_design_system() {
        assert_eq!(BG, Color::Rgb(0x0c, 0x10, 0x14));
        assert_eq!(PANEL, Color::Rgb(0x11, 0x18, 0x20));
        assert_eq!(SURFACE, Color::Rgb(0x0a, 0x37, 0x49));
        assert_eq!(TEXT, Color::Rgb(0x99, 0xd1, 0xce));
        assert_eq!(MUTED, Color::Rgb(0x99, 0xd1, 0xce));
        assert_eq!(DIM, Color::Rgb(0x99, 0xd1, 0xce));
        assert_eq!(ACCENT, Color::Rgb(0x33, 0x85, 0x9e));
        assert_eq!(GREEN, Color::Rgb(0x2a, 0xa8, 0x89));
        assert_eq!(YELLOW, Color::Rgb(0xed, 0xb4, 0x43));
        assert_eq!(RED, Color::Rgb(0xc2, 0x31, 0x27));
    }
}
