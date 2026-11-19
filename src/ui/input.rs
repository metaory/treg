//! Minimal single-line editor; `cursor` counts chars, not bytes.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
pub struct Input {
    pub text: String,
    pub cursor: usize,
}

impl Input {
    pub fn new(text: &str) -> Self {
        Self { text: text.to_owned(), cursor: text.chars().count() }
    }

    fn byte(&self, at: usize) -> usize {
        self.text.char_indices().nth(at).map_or(self.text.len(), |(b, _)| b)
    }

    /// Applies an editing key; returns whether the text changed.
    pub fn handle(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let len = self.text.chars().count();
        match key.code {
            KeyCode::Char('u') if ctrl => {
                self.text.drain(..self.byte(self.cursor));
                self.cursor = 0;
                true
            }
            KeyCode::Char(c) if !ctrl => {
                self.text.insert(self.byte(self.cursor), c);
                self.cursor += 1;
                true
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.text.remove(self.byte(self.cursor));
                true
            }
            KeyCode::Left => self.step(self.cursor.saturating_sub(1)),
            KeyCode::Right => self.step((self.cursor + 1).min(len)),
            KeyCode::Home => self.step(0),
            KeyCode::End => self.step(len),
            _ => false,
        }
    }

    fn step(&mut self, to: usize) -> bool {
        self.cursor = to;
        false
    }
}
