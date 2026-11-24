//! Colors, from `design/system.md`.
//!
//! The canvas roles are the sealed warm system: charcoal ground, bone-white
//! text, warm stone detail and a single rust accent reserved for the active
//! marker. Nothing outside this module names a color.

use ratatui::style::Color;

/// Terminal canvas.
pub const BG: Color = Color::Rgb(0x0d, 0x0b, 0x0a);
/// Rail surface, one step up from the canvas.
pub const PANEL: Color = Color::Rgb(0x14, 0x12, 0x10);
/// Phone sheet and header band, two steps up from the canvas.
pub const ELEVATED: Color = Color::Rgb(0x1a, 0x17, 0x14);
/// Hairline separators, the rail rule, section rules.
pub const LINE: Color = Color::Rgb(0x26, 0x22, 0x20);
/// Primary text: item names, values, emphasis.
pub const TEXT: Color = Color::Rgb(0xec, 0xe6, 0xe0);
/// Second-line detail: paths, state words, metadata.
pub const BODY: Color = Color::Rgb(0xb0, 0xa6, 0x9e);
/// Section labels, counts, caption text.
pub const MUTED: Color = Color::Rgb(0x9c, 0x92, 0x8a);
/// Discreet text: overflow indicators, actions at rest.
pub const DIM: Color = Color::Rgb(0x6f, 0x66, 0x5f);
/// Numbered panel numerals and connection markers, not a rail.
pub const ACCENT: Color = Color::Rgb(0xf7, 0x4c, 0x00);
/// The active space's focus glyph and the active tab's shortcut digit.
pub const ACCENT_BRIGHT: Color = Color::Rgb(0xff, 0x6a, 0x1f);
/// The one-cell edge on the left of an active two-row block.
pub const RUST_LINE: Color = Color::Rgb(0x5c, 0x28, 0x10);
/// Framing for panels and their separation from the terminal.
pub const PANEL_BORDER: Color = Color::Rgb(0x33, 0x2e, 0x2a);
/// Active item fill: a warm rust tint across both rows.
pub const WASH: Color = Color::Rgb(0x2a, 0x16, 0x0c);
/// Legacy name for the active fill; the renderer never paints a hard band.
pub const SURFACE: Color = WASH;
/// Agent state `idle`, connected.
pub const GREEN: Color = Color::Rgb(0x4a, 0xde, 0x80);
/// Agent state `working`, warning.
pub const YELLOW: Color = Color::Rgb(0xf0, 0xb4, 0x29);
/// Agent state `blocked`, error, disconnected.
pub const RED: Color = Color::Rgb(0xff, 0x63, 0x69);

#[cfg(test)]
mod tests {
    use super::*;

    /// `design/system.md` pins these; a change here is a change to the design.
    #[test]
    fn the_palette_matches_the_design_system() {
        assert_eq!(BG, Color::Rgb(0x0d, 0x0b, 0x0a));
        assert_eq!(PANEL, Color::Rgb(0x14, 0x12, 0x10));
        assert_eq!(ELEVATED, Color::Rgb(0x1a, 0x17, 0x14));
        assert_eq!(LINE, Color::Rgb(0x26, 0x22, 0x20));
        assert_eq!(TEXT, Color::Rgb(0xec, 0xe6, 0xe0));
        assert_eq!(BODY, Color::Rgb(0xb0, 0xa6, 0x9e));
        assert_eq!(MUTED, Color::Rgb(0x9c, 0x92, 0x8a));
        assert_eq!(DIM, Color::Rgb(0x6f, 0x66, 0x5f));
        assert_eq!(ACCENT, Color::Rgb(0xf7, 0x4c, 0x00));
        assert_eq!(ACCENT_BRIGHT, Color::Rgb(0xff, 0x6a, 0x1f));
        assert_eq!(RUST_LINE, Color::Rgb(0x5c, 0x28, 0x10));
        assert_eq!(PANEL_BORDER, Color::Rgb(0x33, 0x2e, 0x2a));
        assert_eq!(WASH, Color::Rgb(0x2a, 0x16, 0x0c), "the active wash is the sealed rust tint");
        assert_eq!(SURFACE, WASH, "the legacy name has no hard band of its own");
        assert_eq!(GREEN, Color::Rgb(0x4a, 0xde, 0x80));
        assert_eq!(YELLOW, Color::Rgb(0xf0, 0xb4, 0x29));
        assert_eq!(RED, Color::Rgb(0xff, 0x63, 0x69));
    }
}
