use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks a burst of held letter keys and decides, once they're all
/// released, whether the burst looked like a genuine simultaneous chord
/// attempt (as opposed to normal sequential typing).
pub struct ChordBuffer {
    window: Duration,
    held: HashMap<char, Instant>,
    first_press: Option<Instant>,
    last_press: Option<Instant>,
    /// All distinct letters seen since the burst began (union across the
    /// whole hold, even letters already released before the last one goes
    /// up) — this is the chord's letter-set candidate.
    burst_letters: Vec<char>,
    /// Number of individual key-press events in this burst. This is what
    /// must be backspaced, since it's how many literal characters the OS
    /// already typed (not the matched word's length — see dictionary.rs).
    press_count: usize,
}

/// The outcome of a burst ending: either it looked like a real chord
/// attempt (worth checking against the dictionary) or it didn't.
pub struct BurstResult {
    pub letters: Vec<char>,
    pub press_count: usize,
}

impl ChordBuffer {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            held: HashMap::new(),
            first_press: None,
            last_press: None,
            burst_letters: Vec::new(),
            press_count: 0,
        }
    }

    /// Call on every KeyPress of a letter. Ignores OS auto-repeat (a letter
    /// already held generates repeated KeyPress events on macOS if you keep
    /// it down).
    pub fn key_down(&mut self, letter: char, at: Instant) {
        if self.held.contains_key(&letter) {
            return; // auto-repeat, not a new press
        }
        if self.held.is_empty() {
            self.first_press = Some(at);
            self.burst_letters.clear();
            self.press_count = 0;
        }
        self.last_press = Some(at);
        self.held.insert(letter, at);
        self.burst_letters.push(letter);
        self.press_count += 1;
    }

    /// Call on every KeyRelease of a letter. Returns `Some` once the last
    /// held letter key is released, ending the burst.
    pub fn key_up(&mut self, letter: char) -> Option<BurstResult> {
        self.held.remove(&letter);
        if !self.held.is_empty() {
            return None;
        }
        let first = self.first_press.take()?;
        let last = self.last_press.take().unwrap_or(first);
        // Simultaneity is judged by the spread between the first and last
        // *press* (how tight the chord was struck), not by when it was
        // released — you can hold a clean chord for a while before letting go.
        let within_window = last.saturating_duration_since(first) <= self.window;
        let result = if within_window && self.press_count >= 2 {
            Some(BurstResult {
                letters: std::mem::take(&mut self.burst_letters),
                press_count: self.press_count,
            })
        } else {
            self.burst_letters.clear();
            None
        };
        self.press_count = 0;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simultaneous_presses_within_window_form_a_burst() {
        let mut buf = ChordBuffer::new(Duration::from_millis(150));
        let t0 = Instant::now();
        buf.key_down('l', t0);
        buf.key_down('o', t0);
        buf.key_down('k', t0);
        assert!(buf.key_up('l').is_none());
        assert!(buf.key_up('o').is_none());
        let result = buf.key_up('k').expect("burst should end here");
        assert_eq!(result.press_count, 3);
        let mut letters = result.letters.clone();
        letters.sort_unstable();
        assert_eq!(letters, vec!['k', 'l', 'o']);
    }

    #[test]
    fn single_key_never_forms_a_chord() {
        let mut buf = ChordBuffer::new(Duration::from_millis(150));
        buf.key_down('a', Instant::now());
        assert!(buf.key_up('a').is_none());
    }
}
