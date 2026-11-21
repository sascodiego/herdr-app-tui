//! Stand-in for herdr-app's `sprites::is_sprite`.
//!
//! The real `sprites.rs` is a 516-line GPUI painter. `vt.rs` only asks it
//! whether one character is a sprite, so this keeps the decision without the
//! GPU paint path.

/// `true` for the box-drawing, block and braille characters herdr-app draws as
/// sprites rather than through the font.
pub fn is_sprite(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => {
            ('\u{2500}'..='\u{257f}').contains(&c) // box drawing
                || ('\u{2580}'..='\u{259f}').contains(&c) // block elements
                || ('\u{2800}'..='\u{28ff}').contains(&c) // braille
                || ('\u{e0b0}'..='\u{e0bf}').contains(&c) // powerline
                || c == '\u{23fa}' // ⏺, Claude Code's step marker
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_sprite;

    #[test]
    fn a_single_sprite_character_is_recognised() {
        assert!(is_sprite("│"));
        assert!(is_sprite("█"));
        assert!(is_sprite("⠋"));
        assert!(is_sprite("⏺"));
    }

    #[test]
    fn ordinary_text_is_not_a_sprite() {
        assert!(!is_sprite("a"));
        assert!(!is_sprite("ab"), "only one character can be a sprite");
        assert!(!is_sprite(""));
    }
}
