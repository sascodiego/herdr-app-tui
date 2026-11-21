//! The glyph family and the state vocabulary, from `design/system.md`.
//!
//! One geometric family, used everywhere. State is always glyph plus word, never
//! color alone. Selection is a surface band, never a glyph: a selection marker
//! and a state glyph in the same column read as the same kind of thing.

use ratatui::style::Color;

use super::theme;

/// What a core program's status word says about a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Working,
    NeedsYou,
    Done,
    Failed,
    /// It never reported a status.
    Unknown,
}

impl State {
    /// Read a herdr-core status word: `working`, `idle`, `done`, `blocked`,
    /// `error`. Anything else, and `None`, is unknown.
    pub fn from_word(word: Option<&str>) -> State {
        match word {
            Some("idle") => State::Idle,
            Some("working") => State::Working,
            Some("blocked") => State::NeedsYou,
            Some("done") => State::Done,
            Some("error") => State::Failed,
            _ => State::Unknown,
        }
    }

    /// The glyph that marks it.
    pub fn glyph(self) -> char {
        match self {
            State::Idle => '○',
            State::Working => '◐',
            State::NeedsYou => '▲',
            State::Done => '✓',
            State::Failed => '×',
            State::Unknown => '·',
        }
    }

    /// The word that goes with the glyph.
    pub fn word(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Working => "working",
            State::NeedsYou => "needs you",
            State::Done => "done",
            State::Failed => "error",
            State::Unknown => "unknown",
        }
    }

    /// Its color role. Never the only signal: always read with the glyph and word.
    pub fn color(self) -> Color {
        match self {
            State::Idle => theme::GREEN,
            State::Working => theme::YELLOW,
            State::NeedsYou | State::Failed => theme::RED,
            State::Done | State::Unknown => theme::DIM,
        }
    }

    /// Whether it wants the person's attention.
    pub fn needs_you(self) -> bool {
        matches!(self, State::NeedsYou | State::Failed)
    }

    /// The herdr order of attention: blocked > done > working > idle.
    pub fn priority(self) -> u8 {
        match self {
            State::NeedsYou | State::Failed => 0,
            State::Done => 1,
            State::Working => 2,
            State::Idle => 3,
            State::Unknown => 4,
        }
    }
}

/// Section collapse markers and the rest of the family.
pub const EXPANDED: char = '▾';
pub const COLLAPSED: char = '▸';
pub const CLOSE: char = '×';
pub const CONNECTED: char = '●';
pub const DISCONNECTED: char = '○';
pub const SEPARATOR: char = '·';

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_word_maps_to_one_state() {
        assert_eq!(State::from_word(Some("idle")), State::Idle);
        assert_eq!(State::from_word(Some("working")), State::Working);
        assert_eq!(State::from_word(Some("blocked")), State::NeedsYou);
        assert_eq!(State::from_word(Some("done")), State::Done);
        assert_eq!(State::from_word(Some("error")), State::Failed);
        assert_eq!(State::from_word(Some("nonsense")), State::Unknown);
        assert_eq!(State::from_word(None), State::Unknown);
    }

    #[test]
    fn every_state_has_a_distinct_glyph_and_word() {
        let all = [State::Idle, State::Working, State::NeedsYou, State::Done, State::Failed, State::Unknown];
        let glyphs: Vec<char> = all.iter().map(|state| state.glyph()).collect();
        let words: Vec<&str> = all.iter().map(|state| state.word()).collect();
        for (i, glyph) in glyphs.iter().enumerate() {
            assert!(!glyphs[i + 1..].contains(glyph), "glyph {glyph:?} repeats");
            assert!(!words[i + 1..].contains(&words[i]), "word {:?} repeats", words[i]);
        }
    }

    #[test]
    fn only_blocked_and_error_ask_for_the_person() {
        assert!(State::NeedsYou.needs_you());
        assert!(State::Failed.needs_you());
        assert!(!State::Working.needs_you());
        assert!(!State::Idle.needs_you());
        assert!(!State::Done.needs_you());
    }

    #[test]
    fn the_priority_puts_whoever_needs_you_first_and_unknown_last() {
        let mut states = [State::Unknown, State::Idle, State::Working, State::Done, State::NeedsYou];
        states.sort_by_key(|state| state.priority());
        assert_eq!(states, [State::NeedsYou, State::Done, State::Working, State::Idle, State::Unknown]);
    }
}
