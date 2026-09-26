use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks a chord attempt and decides when it's done.
///
/// A chord can be entered two ways, and this buffer treats them uniformly:
///
/// - **Held together**: press several keys, all overlapping, release them.
///   No timing check needed — while any key is still held, new presses
///   obviously belong to the same chord.
/// - **Arpeggiated / rolled**: press and release keys one at a time (or a
///   couple at once), as long as each new press starts within `roll_gap` of
///   the moment the keyboard last went idle (nothing held). This is the
///   only way to enter a chord bigger than your keyboard's simultaneous-key
///   ceiling (commonly 6 on non-NKRO keyboards) — you never need more than
///   a couple of keys down at once.
///
/// A burst commits (is handed back to the caller) when the keyboard goes
/// idle and *stays* idle past `roll_gap` (see `flush_if_idle`), or
/// immediately when the caller knows the word boundary has been reached —
/// e.g. a space or punctuation key was pressed (see `flush_now`).
pub struct ChordBuffer {
    roll_gap: Duration,
    held: HashMap<char, Instant>,
    burst_letters: Vec<char>,
    press_count: usize,
    /// When `held` most recently became empty. `None` before the first
    /// burst starts.
    last_release: Option<Instant>,
    /// Whether `burst_letters` holds an uncommitted attempt (as opposed to
    /// being merely empty because nothing has happened yet).
    pending: bool,
}

pub struct BurstResult {
    pub letters: Vec<char>,
    pub press_count: usize,
}

impl ChordBuffer {
    pub fn new(roll_gap: Duration) -> Self {
        Self {
            roll_gap,
            held: HashMap::new(),
            burst_letters: Vec::new(),
            press_count: 0,
            last_release: None,
            pending: false,
        }
    }

    /// Call on every KeyPress of a letter. Ignores OS auto-repeat (a letter
    /// still held generates repeated KeyPress events if you keep it down).
    pub fn key_down(&mut self, letter: char, at: Instant) {
        if self.held.contains_key(&letter) {
            return; // auto-repeat, not a new press
        }
        if self.held.is_empty() {
            let continuing_roll = self.pending
                && self
                    .last_release
                    .is_some_and(|t| at.saturating_duration_since(t) <= self.roll_gap);
            if !continuing_roll {
                self.reset();
            }
        }
        self.held.insert(letter, at);
        self.burst_letters.push(letter);
        self.press_count += 1;
        self.pending = true;
    }

    /// Call on every KeyRelease of a letter. Never commits by itself — only
    /// records when the keyboard went idle, so a roll continuation can
    /// still arrive. Use `flush_now`/`flush_if_idle` to actually commit.
    pub fn key_up(&mut self, letter: char, at: Instant) {
        self.held.remove(&letter);
        if self.held.is_empty() {
            self.last_release = Some(at);
        }
    }

    /// Force-commits a pending burst immediately, regardless of timing —
    /// call this when a non-letter key (space, punctuation, enter...) is
    /// pressed, since that unambiguously marks a word boundary.
    /// No-ops if keys are still held (shouldn't normally happen) or
    /// nothing is pending.
    pub fn flush_now(&mut self) -> Option<BurstResult> {
        if !self.pending || !self.held.is_empty() {
            return None;
        }
        self.commit()
    }

    /// Commits a pending burst once it's been idle (nothing held, no new
    /// press) for longer than `roll_gap` — i.e. the roll window lapsed with
    /// no continuation. Call this periodically (e.g. every 50ms) to handle
    /// the case where the user stops typing without hitting a delimiter.
    pub fn flush_if_idle(&mut self, now: Instant) -> Option<BurstResult> {
        if !self.pending || !self.held.is_empty() {
            return None;
        }
        let idle_long_enough = self
            .last_release
            .is_some_and(|t| now.saturating_duration_since(t) > self.roll_gap);
        if !idle_long_enough {
            return None;
        }
        self.commit()
    }

    fn commit(&mut self) -> Option<BurstResult> {
        let press_count = self.press_count;
        let letters = std::mem::take(&mut self.burst_letters);
        self.pending = false;
        self.press_count = 0;
        if press_count < 2 {
            return None; // a single key is just a letter, never a chord
        }
        Some(BurstResult {
            letters,
            press_count,
        })
    }

    fn reset(&mut self) {
        self.burst_letters.clear();
        self.press_count = 0;
        self.pending = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_together_forms_a_chord_on_flush() {
        let mut buf = ChordBuffer::new(Duration::from_millis(150));
        let t0 = Instant::now();
        buf.key_down('l', t0);
        buf.key_down('o', t0);
        buf.key_down('k', t0);
        buf.key_up('l', t0);
        buf.key_up('o', t0);
        buf.key_up('k', t0);
        assert!(buf.flush_if_idle(t0).is_none()); // roll_gap hasn't elapsed yet
        let result = buf.flush_now().expect("space/punct should force commit");
        assert_eq!(result.press_count, 3);
        let mut letters = result.letters.clone();
        letters.sort_unstable();
        assert_eq!(letters, vec!['k', 'l', 'o']);
    }

    #[test]
    fn arpeggiated_roll_beyond_hardware_rollover_still_forms_one_chord() {
        // "different" has 7 distinct letters (d,e,f,i,n,r,t) — more than a
        // typical 6-key-rollover keyboard can hold at once. Roll them one
        // at a time, well inside the roll gap between each.
        let mut buf = ChordBuffer::new(Duration::from_millis(200));
        let mut t = Instant::now();
        for letter in ['d', 'e', 'f', 'i', 'n', 'r', 't'] {
            buf.key_down(letter, t);
            buf.key_up(letter, t);
            assert!(buf.flush_if_idle(t).is_none(), "gap hasn't elapsed yet");
            t += Duration::from_millis(50); // well under the 200ms roll gap
        }
        let result = buf.flush_now().expect("rolled chord should commit");
        assert_eq!(result.press_count, 7);
        let mut letters = result.letters.clone();
        letters.sort_unstable();
        assert_eq!(letters, vec!['d', 'e', 'f', 'i', 'n', 'r', 't']);
    }

    #[test]
    fn a_pause_beyond_roll_gap_starts_a_new_burst() {
        let mut buf = ChordBuffer::new(Duration::from_millis(100));
        let t0 = Instant::now();
        buf.key_down('a', t0);
        buf.key_up('a', t0);
        let t1 = t0 + Duration::from_millis(500); // well past the roll gap
        buf.key_down('b', t1);
        buf.key_up('b', t1);
        // The dropped 'a' burst never committed (it was a single key
        // anyway); 'b' alone is also just a single key, so nothing commits.
        assert!(buf.flush_now().is_none());
    }

    #[test]
    fn single_key_never_forms_a_chord() {
        let mut buf = ChordBuffer::new(Duration::from_millis(150));
        let t0 = Instant::now();
        buf.key_down('a', t0);
        buf.key_up('a', t0);
        assert!(buf.flush_now().is_none());
    }

    #[test]
    fn idle_timeout_commits_without_an_explicit_delimiter() {
        let mut buf = ChordBuffer::new(Duration::from_millis(100));
        let t0 = Instant::now();
        buf.key_down('i', t0);
        buf.key_down('n', t0);
        buf.key_up('i', t0);
        buf.key_up('n', t0);
        let past_gap = t0 + Duration::from_millis(150);
        let result = buf
            .flush_if_idle(past_gap)
            .expect("should commit once idle past the roll gap");
        assert_eq!(result.press_count, 2);
    }
}
