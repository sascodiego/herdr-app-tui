//! Keys to actions.
//!
//! A live terminal owns the keyboard while the panel is closed. Only the
//! prefix keys are consumed here; everything else is encoded and sent to the
//! attached program by `attach::encode_key`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};

/// What a key asks the screen to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Leave the screen. Ctrl+Q, or Ctrl+\ as crossterm decodes it.
    Quit,
    TogglePanel,
    ClosePanel,
    /// Attach to the highlighted tab and give it the keyboard.
    Attach,
    /// Move through the frozen history, entering copy mode if needed.
    ScrollUp,
    ScrollDown,
    Up,
    Down,
    Refresh,
    /// Open the workspace command palette from anywhere; reserved like Quit.
    Palette,
    /// Type a command while the panel is open.
    Command,
    Ignore,
}

/// A pointer gesture, before the screen decides which zone it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    Tap,
    ScrollUp,
    ScrollDown,
    /// A held drag: the caller measures how far it moved.
    Drag,
}

/// Translate a crossterm mouse event. Other buttons never steer the UI.
pub fn pointer(kind: MouseEventKind) -> Option<Pointer> {
    match kind {
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => Some(Pointer::Tap),
        MouseEventKind::ScrollUp => Some(Pointer::ScrollUp),
        MouseEventKind::ScrollDown => Some(Pointer::ScrollDown),
        MouseEventKind::Drag(MouseButton::Left) => Some(Pointer::Drag),
        _ => None,
    }
}

/// Read one key press.
pub fn action(key: KeyEvent) -> Action {
    match (key.code, key.modifiers) {
        // crossterm maps byte 0x1c (Ctrl+\) to Char('4') + CONTROL.
        (KeyCode::Char('\\'), KeyModifiers::CONTROL)
        | (KeyCode::Char('4'), KeyModifiers::CONTROL)
        | (KeyCode::Char('q'), KeyModifiers::CONTROL) => Action::Quit,
        (KeyCode::Char('o'), KeyModifiers::CONTROL) => Action::Palette,
        (KeyCode::Tab, _) => Action::TogglePanel,
        (KeyCode::Esc, _) => Action::ClosePanel,
        (KeyCode::Enter, _) => Action::Attach,
        (KeyCode::PageUp, _) | (KeyCode::Up, KeyModifiers::ALT) => Action::ScrollUp,
        (KeyCode::PageDown, _) | (KeyCode::Down, KeyModifiers::ALT) => Action::ScrollDown,
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => Action::Up,
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => Action::Down,
        (KeyCode::Char('r'), KeyModifiers::NONE) => Action::Refresh,
        (KeyCode::Char(':'), KeyModifiers::NONE | KeyModifiers::SHIFT) => Action::Command,
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
    fn page_keys_and_alt_arrows_scroll_without_touching_the_agent() {
        assert_eq!(action(key(KeyCode::PageUp)), Action::ScrollUp);
        assert_eq!(action(key(KeyCode::PageDown)), Action::ScrollDown);
        assert_eq!(action(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT)), Action::ScrollUp);
        assert_eq!(action(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT)), Action::ScrollDown);
        // A bare arrow still belongs to the panel or the agent.
        assert_eq!(action(key(KeyCode::Up)), Action::Up);
    }

    #[test]
    fn only_left_pointer_events_steer_the_ui() {
        assert_eq!(pointer(MouseEventKind::Down(MouseButton::Left)), Some(Pointer::Tap));
        assert_eq!(pointer(MouseEventKind::Up(MouseButton::Left)), Some(Pointer::Tap));
        assert_eq!(pointer(MouseEventKind::ScrollUp), Some(Pointer::ScrollUp));
        assert_eq!(pointer(MouseEventKind::ScrollDown), Some(Pointer::ScrollDown));
        assert_eq!(pointer(MouseEventKind::Drag(MouseButton::Left)), Some(Pointer::Drag));
        assert_eq!(pointer(MouseEventKind::Down(MouseButton::Right)), None);
        assert_eq!(pointer(MouseEventKind::Moved), None);
    }

    #[test]
    fn ctrl_o_opens_the_palette_without_typing_a_colon() {
        assert_eq!(action(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)), Action::Palette);
        assert_ne!(action(key(KeyCode::Char(':'))), Action::Palette);
    }

    #[test]
    fn colon_opens_the_command_palette() {
        assert_eq!(action(key(KeyCode::Char(':'))), Action::Command);
        assert_eq!(action(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::SHIFT)), Action::Command);
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
