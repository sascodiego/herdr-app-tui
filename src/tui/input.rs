//! Keys to actions.
//!
//! Until T5 attaches a terminal, this screen is a viewer: keys drive the UI and
//! are never forwarded to an agent. `q` therefore quits without asking.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// What a key asks the screen to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,
    TogglePanel,
    ClosePanel,
    Up,
    Down,
    Refresh,
    Ignore,
}

/// Read one key press.
pub fn action(key: KeyEvent) -> Action {
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Quit,
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Tab, _) => Action::TogglePanel,
        (KeyCode::Esc, _) => Action::ClosePanel,
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
    fn q_and_ctrl_c_quit() {
        assert_eq!(action(key(KeyCode::Char('q'))), Action::Quit);
        assert_eq!(action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), Action::Quit);
    }

    #[test]
    fn a_modified_q_is_not_a_quit() {
        // Ctrl+Q is not the quit key; only plain q and Ctrl+C are.
        assert_eq!(action(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)), Action::Ignore);
    }

    #[test]
    fn anything_else_is_ignored() {
        assert_eq!(action(key(KeyCode::Char('z'))), Action::Ignore);
        assert_eq!(action(key(KeyCode::F(5))), Action::Ignore);
    }
}
