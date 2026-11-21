//! Keys to actions.
//!
//! A live terminal owns the keyboard while the panel is closed. Only the
//! prefix keys are consumed here; everything else is encoded and sent to the
//! attached program by `attach::encode_key`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// What a key asks the screen to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Leave the screen. Ctrl+Q, or Ctrl+\ as crossterm decodes it.
    Quit,
    TogglePanel,
    ClosePanel,
    /// Attach to the highlighted tab and give it the keyboard.
    Attach,
    Up,
    Down,
    Refresh,
    Ignore,
}

/// Read one key press.
pub fn action(key: KeyEvent) -> Action {
    match (key.code, key.modifiers) {
        // crossterm maps byte 0x1c (Ctrl+\) to Char('4') + CONTROL.
        (KeyCode::Char('\\'), KeyModifiers::CONTROL)
        | (KeyCode::Char('4'), KeyModifiers::CONTROL)
        | (KeyCode::Char('q'), KeyModifiers::CONTROL) => Action::Quit,
        (KeyCode::Tab, _) => Action::TogglePanel,
        (KeyCode::Esc, _) => Action::ClosePanel,
        (KeyCode::Enter, _) => Action::Attach,
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => Action::Up,
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => Action::Down,
        (KeyCode::Char('r'), KeyModifiers::NONE) => Action::Refresh,
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn tab_opens_the_panel_and_escape_closes_it() {
        assert_eq!(action(key(KeyCode::Tab)), Action::TogglePanel);
        assert_eq!(action(key(KeyCode::Esc)), Action::ClosePanel);
    }

    #[test]
    fn the_arrows_and_j_k_move_the_cursor() {
        for code in [KeyCode::Up, KeyCode::Char('k')] {
            assert_eq!(action(key(code)), Action::Up);
        }
        for code in [KeyCode::Down, KeyCode::Char('j')] {
            assert_eq!(action(key(code)), Action::Down);
        }
    }

    #[test]
    fn only_the_reserved_ctrl_keys_quit() {
        for code in [KeyCode::Char('\\'), KeyCode::Char('4'), KeyCode::Char('q')] {
            assert_eq!(action(KeyEvent::new(code, KeyModifiers::CONTROL)), Action::Quit, "{code:?}");
        }
        assert_eq!(action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), Action::Ignore);
        assert_eq!(action(key(KeyCode::Char('q'))), Action::Ignore);
    }

    #[test]
    fn enter_attaches_to_the_highlighted_tab() {
        assert_eq!(action(key(KeyCode::Enter)), Action::Attach);
    }

    #[test]
    fn ordinary_keys_are_not_consumed_by_the_panel() {
        // They must reach the attached program, including q and Ctrl+C.
        for code in [KeyCode::Char('q'), KeyCode::Char('z'), KeyCode::Esc] {
            assert_ne!(action(key(code)), Action::Quit, "{code:?}");
        }
        assert_eq!(action(key(KeyCode::F(5))), Action::Ignore);
    }
}
