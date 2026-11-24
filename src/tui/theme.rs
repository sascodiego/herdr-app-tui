//! Colors, from `design/system.md`.
//!
//! The canvas roles are Catppuccin Mocha — the theme the herdr TUI uses by
//! default, and the lavender the mobile captures show. The app chrome roles are
//! herdr-app's neutral tokens. Nothing outside this module names a color.

use ratatui::style::Color;

/// Terminal canvas.
pub const BG: Color = Color::Rgb(0x11, 0x11, 0x1b);
/// Sheet and rail surface, one step up from the canvas.
pub const PANEL: Color = Color::Rgb(0x18, 0x18, 0x25);
/// The selected row band.
pub const SURFACE: Color = Color::Rgb(0x31, 0x32, 0x44);
/// Primary text: labels, values, agent names.
pub const TEXT: Color = Color::Rgb(0xcd, 0xd6, 0xf4);
/// Secondary text: section headers, counts, paths.
pub const MUTED: Color = Color::Rgb(0x7f, 0x84, 0x9c);
/// Disabled text, separators, footer actions at rest.
pub const DIM: Color = Color::Rgb(0x58, 0x5b, 0x70);
/// Selection marker, focus, active tab.
pub const ACCENT: Color = Color::Rgb(0xb4, 0xbe, 0xfe);
/// Agent idle, connected.
pub const GREEN: Color = Color::Rgb(0xa6, 0xe3, 0xa1);
/// Agent working, warning.
pub const YELLOW: Color = Color::Rgb(0xf9, 0xe2, 0xaf);
/// Agent blocked, error, disconnected.
pub const RED: Color = Color::Rgb(0xf3, 0x8b, 0xa8);

#[cfg(test)]
mod tests {
    use super::*;

    /// `design/system.md` pins these; a change here is a change to the design.
    #[test]
    fn the_palette_matches_the_design_system() {
        assert_eq!(BG, Color::Rgb(0x11, 0x11, 0x1b));
        assert_eq!(PANEL, Color::Rgb(0x18, 0x18, 0x25));
        assert_eq!(SURFACE, Color::Rgb(0x31, 0x32, 0x44));
        assert_eq!(TEXT, Color::Rgb(0xcd, 0xd6, 0xf4));
        assert_eq!(MUTED, Color::Rgb(0x7f, 0x84, 0x9c));
        assert_eq!(DIM, Color::Rgb(0x58, 0x5b, 0x70));
        assert_eq!(ACCENT, Color::Rgb(0xb4, 0xbe, 0xfe));
        assert_eq!(GREEN, Color::Rgb(0xa6, 0xe3, 0xa1));
        assert_eq!(YELLOW, Color::Rgb(0xf9, 0xe2, 0xaf));
        assert_eq!(RED, Color::Rgb(0xf3, 0x8b, 0xa8));
    }
}
