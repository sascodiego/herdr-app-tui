//! A live, two-way terminal attached through herdr-core.
//!
//! `hc` already owns the framed channel: `attach` gives us a stream of
//! `Stream::Data` bytes, and `input`/`credit`/`resize` send back. This module
//! ties that channel to `vt::Vt`, which interprets the bytes as a screen.

use anyhow::{Context, anyhow};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::Arc;

use crate::hc;
use crate::vt::{Frame, Vt};

/// How many bytes to credit after draining a batch.
///
/// `hc::attach` credits 1 MiB up front; every consumed chunk must be credited
/// again or the core eventually stops sending.
const CREDIT_CHUNK: usize = 64 * 1024;

/// One attached terminal.
pub struct Session {
    attached: hc::Attached,
    pub vt: Vt,
    cols: u16,
    rows: u16,
}

impl Session {
    /// Attach to a terminal and take its first paint.
    pub fn attach(core: &Arc<hc::Core>, terminal: &str, cols: u16, rows: u16) -> anyhow::Result<Session> {
        let attached = core
            .attach(terminal, cols, rows, serde_json::json!({}))
            .with_context(|| format!("cannot attach to {terminal}"))?;
        let vt = Vt::new(cols, rows);
        if let Some(title) = attached.terminal.pointer("/status/state").and_then(serde_json::Value::as_str) {
            tracing::debug!("attached to {terminal} in state {title}");
        }
        Ok(Session { attached, vt, cols, rows })
    }

    pub fn channel(&self) -> u32 {
        self.attached.channel
    }

    /// Forward one wheel notch to the program when it tracks the mouse.
    /// Returns false when there is no tracking, so the caller can keep herdr's
    /// own copy-mode scrollback for that case.
    pub fn wheel(&mut self, core: &Arc<hc::Core>, up: bool, col: u16, row: u16) -> bool {
        let bytes = self.vt.wheel_bytes(up, col, row);
        if bytes.is_empty() {
            return false;
        }
        // Wheel input is a person acting, not a terminal-engine reply.
        core.input(self.attached.channel, &bytes);
        true
    }

    /// Drain everything the core has sent. Returns true if anything arrived.
    pub fn drain(&mut self, core: &Arc<hc::Core>) -> anyhow::Result<bool> {
        let mut consumed = 0usize;
        let mut saw_data = false;
        loop {
            match self.attached.stream.try_recv() {
                Ok(hc::Stream::Data(bytes)) => {
                    consumed += bytes.len();
                    self.vt.write(&bytes);
                    saw_data = true;
                }
                Ok(hc::Stream::Closed(reason)) => {
                    let why = reason.get("reason").and_then(serde_json::Value::as_str).unwrap_or("closed");
                    return Err(anyhow!("the terminal closed: {why}"));
                }
                Ok(hc::Stream::Lost) => return Err(anyhow!("the connection to the core was lost")),
                Err(_) => break,
            }
        }
        if consumed > 0 {
            // Credit in bounded pieces: one u32 per message, just like upstream.
            let mut left = consumed;
            while left > 0 {
                let n = left.min(CREDIT_CHUNK);
                core.credit(self.attached.channel, n);
                left -= n;
            }
        }
        // Terminal-engine answers (cursor reports, Kitty queries) go back as
        // automatic input, not as keystrokes.
        let replies = self.vt.take_replies();
        if !replies.is_empty() {
            core.automatic_input(self.attached.channel, &replies);
        }
        Ok(saw_data)
    }

    /// The screen to draw. In copy mode this is the scrolled viewport, with
    /// the program's own styles still on every cell.
    pub fn frame(&mut self) -> Frame {
        self.vt.frame()
    }

    /// Move through libghostty's scrollback. Positive lines show older output.
    pub fn scroll_up(&mut self, lines: usize) {
        self.vt.scroll_viewport(-(lines as isize));
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.vt.scroll_viewport(lines as isize);
    }

    /// Resume following live output.
    pub fn scroll_to_live(&mut self) {
        self.vt.scroll_to_bottom();
    }

    /// Keep the viewer's size in sync with the pane.
    pub fn resize(&mut self, core: &Arc<hc::Core>, cols: u16, rows: u16) -> bool {
        if cols == 0 || rows == 0 || (cols == self.cols && rows == self.rows) {
            return false;
        }
        self.cols = cols;
        self.rows = rows;
        self.vt.resize(cols, rows, 10, 20);
        core.resize(self.attached.channel, cols, rows);
        true
    }

    /// A key press, encoded as the bytes the program expects.
    pub fn key(&mut self, core: &Arc<hc::Core>, key: KeyEvent) {
        let bytes = encode_key(key);
        if !bytes.is_empty() {
            core.input(self.attached.channel, &bytes);
        }
    }

    /// Type a whole line (compose/paste), then Enter.
    pub fn type_text(&self, core: &Arc<hc::Core>, text: &str) {
        core.input(self.attached.channel, text.as_bytes());
    }

    /// Stop viewing without killing the program.
    pub fn detach(self, core: &Arc<hc::Core>) {
        core.detach(self.attached.channel);
    }
}

/// The key grammar used by terminals: control, alt, function keys and arrows.
pub fn encode_key(key: KeyEvent) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let mut base = match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                let c = c.to_ascii_lowercase();
                // Ctrl+@ .. Ctrl+_ as 0x00..0x1f; Ctrl+space is NUL.
                vec![if c == ' ' { 0 } else { c as u8 & 0x1f }]
            } else {
                c.to_string().into_bytes()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::F(n) => {
            let code = match n {
                1..=4 => format!("\x1b[{n}~"),
                5 => "\x1b[15~".to_string(),
                6 => "\x1b[17~".to_string(),
                7 => "\x1b[18~".to_string(),
                8 => "\x1b[19~".to_string(),
                9 => "\x1b[20~".to_string(),
                10 => "\x1b[21~".to_string(),
                11 => "\x1b[23~".to_string(),
                12 => "\x1b[24~".to_string(),
                _ => return Vec::new(),
            };
            code.into_bytes()
        }
        _ => return Vec::new(),
    };
    if alt && !base.is_empty() {
        base.insert(0, 0x1b);
    }
    if shift && key.modifiers != KeyModifiers::SHIFT {
        // Modified arrows: CSI 1;2 A and friends.
        if let (Some(b'A' | b'B' | b'C' | b'D' | b'H' | b'F'), true) = (base.last().copied(), base.starts_with(&[0x1b, b'['])) {
            let final_byte = base.pop().expect("checked above");
            base.extend_from_slice(b";2");
            base.push(final_byte);
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::encode_key;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn plain_characters_are_sent_as_is() {
        assert_eq!(encode_key(key(KeyCode::Char('a'), KeyModifiers::NONE)), b"a");
        assert_eq!(encode_key(key(KeyCode::Char('ñ'), KeyModifiers::NONE)), "ñ".as_bytes());
    }

    #[test]
    fn control_characters_become_control_bytes() {
        assert_eq!(encode_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)), vec![3]);
        assert_eq!(encode_key(key(KeyCode::Char(' '), KeyModifiers::CONTROL)), vec![0]);
    }

    #[test]
    fn alt_prefixes_with_escape() {
        assert_eq!(encode_key(key(KeyCode::Char('x'), KeyModifiers::ALT)), vec![0x1b, b'x']);
    }

    #[test]
    fn navigation_keys_use_the_standard_sequences() {
        assert_eq!(encode_key(key(KeyCode::Enter, KeyModifiers::NONE)), vec![b'\r']);
        assert_eq!(encode_key(key(KeyCode::Tab, KeyModifiers::NONE)), vec![b'\t']);
        assert_eq!(encode_key(key(KeyCode::Backspace, KeyModifiers::NONE)), vec![0x7f]);
        assert_eq!(encode_key(key(KeyCode::Up, KeyModifiers::NONE)), vec![0x1b, b'[', b'A']);
        assert_eq!(encode_key(key(KeyCode::F(5), KeyModifiers::NONE)), b"\x1b[15~");
    }

    #[test]
    fn an_unrepresentable_key_sends_nothing() {
        assert!(encode_key(key(KeyCode::Null, KeyModifiers::NONE)).is_empty());
        assert!(encode_key(key(KeyCode::F(20), KeyModifiers::NONE)).is_empty());
    }
}
