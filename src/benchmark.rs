use std::time::{Duration, Instant};

/// A default sample if the user doesn't supply their own --sentence. Chosen
/// to include a few chords likely to be in data/dictionary.en.csv's top few
/// thousand words, so a chorded run actually exercises chording.
pub const DEFAULT_SENTENCE: &str =
    "the quick fox and the lazy dog went to find a different life together";

/// One isolated trial: type a single word once, first keystroke to Enter,
/// then the line is naturally "cleared" — the next trial starts from a
/// fresh empty prompt. Exists because a continuous rapid-fire drill (see
/// RepeatReport) has a failure mode: switching between short chords faster
/// than the roll-gap merges separate attempts into one unresolvable burst.
/// Discrete trials sidestep that by construction, and let each rep start
/// from a real pause (e.g. hands back at home row) instead of momentum.
pub struct Trial {
    pub elapsed: Duration,
    /// Final submitted text matched the target word exactly.
    pub matched: bool,
    /// Backspace presses observed during this trial. Matters even when
    /// `matched` is true: typing "baout", backspacing, and fixing it to
    /// "about" submits a correct final string but was still a real error
    /// — treating it as identical to a clean attempt would hide that.
    pub backspaces: usize,
}

impl Trial {
    /// No mismatch and no self-correction — the only trials whose timing
    /// is a meaningful "how fast is this chord/word" data point.
    pub fn clean(&self) -> bool {
        self.matched && self.backspaces == 0
    }
}

pub struct TrialsReport {
    pub word: String,
    pub trials: Vec<Trial>,
}

impl TrialsReport {
    pub fn print(&self, label: &str) {
        let n = self.trials.len();
        let clean_count = self.trials.iter().filter(|t| t.clean()).count();
        let clean_durations: Vec<Duration> = self
            .trials
            .iter()
            .filter(|t| t.clean())
            .map(|t| t.elapsed)
            .collect();

        println!();
        println!("=== {label}: \"{}\", {n} trials ===", self.word);
        for (i, t) in self.trials.iter().enumerate() {
            let mark = match (t.matched, t.backspaces) {
                (true, 0) => "ok".to_string(),
                (true, n) => format!("ok (but {n} backspace{})", if n == 1 { "" } else { "s" }),
                (false, 0) => "MISS".to_string(),
                (false, n) => format!("MISS ({n} backspace{})", if n == 1 { "" } else { "s" }),
            };
            println!("  {:>2}: {:>6.3}s  {mark}", i + 1, t.elapsed.as_secs_f64());
        }
        println!("accuracy (clean, no typo or backspace): {clean_count}/{n}");
        if !clean_durations.is_empty() {
            let total: Duration = clean_durations.iter().sum();
            let avg = total / clean_durations.len() as u32;
            let min = clean_durations.iter().min().unwrap();
            let max = clean_durations.iter().max().unwrap();
            println!("avg time (clean trials): {:.3}s", avg.as_secs_f64());
            println!(
                "min/max:                 {:.3}s / {:.3}s",
                min.as_secs_f64(),
                max.as_secs_f64()
            );
        } else {
            println!("no clean trials to average");
        }
    }
}

/// A short phrase (one or more words) typed/chorded on repeat for a
/// self-timed window (e.g. "type 'about me' over and over for ~15s"),
/// rather than one fixed sentence. Useful for A/B-ing a specific chord's
/// (or small set of chords') real comfort in isolation — does it actually
/// beat typing normally, repetition for repetition — rather than
/// averaging across a whole sentence's mix of chords. A multi-word unit
/// also forces your fingers to actually move between reps, instead of
/// repeatedly landing the exact same hand shape.
pub struct RepeatReport {
    pub phrase: String,
    pub attempts: usize,
    pub correct: usize,
    pub elapsed: Duration,
    pub gross_wpm: f64,
    /// Correct repetitions per minute — a more direct "how many did I
    /// actually land" number than the chars/5 WPM convention, which blends
    /// in whatever extra characters a botched chord attempt typed.
    pub repetitions_per_minute: f64,
    pub accuracy: f64,
}

/// Scores a repeat drill: `typed` is whatever ended up submitted, split on
/// whitespace and compared token-by-token against `unit` cycled
/// (`unit[i % unit.len()]`), so "about me" repeated scores "about", "me",
/// "about", "me", ... positionally rather than requiring exact wraparound.
/// Works identically whether chording was on or off — a correctly-fired
/// chord's injected replacement is indistinguishable from having typed the
/// word normally, which is exactly the point.
pub fn score_repeated(unit: &[&str], typed: &str, elapsed: Duration) -> RepeatReport {
    let tokens: Vec<&str> = typed.split_whitespace().collect();
    let attempts = tokens.len();
    let correct = tokens
        .iter()
        .enumerate()
        .filter(|(i, t)| **t == unit[i % unit.len()])
        .count();

    let minutes = (elapsed.as_secs_f64() / 60.0).max(1.0 / 3600.0);
    let typed_chars = typed.chars().count();
    let gross_wpm = (typed_chars as f64 / 5.0) / minutes;
    let repetitions_per_minute = correct as f64 / minutes;
    let accuracy = if attempts == 0 {
        0.0
    } else {
        100.0 * correct as f64 / attempts as f64
    };

    RepeatReport {
        phrase: unit.join(" "),
        attempts,
        correct,
        elapsed,
        gross_wpm,
        repetitions_per_minute,
        accuracy,
    }
}

impl RepeatReport {
    pub fn print(&self, label: &str) {
        println!();
        println!("=== {label}: \"{}\" repeated ===", self.phrase);
        println!("time:        {:.2}s", self.elapsed.as_secs_f64());
        println!("attempts:    {}", self.attempts);
        println!("correct:     {}", self.correct);
        println!("accuracy:    {:.1}%", self.accuracy);
        println!("reps/min:    {:.1}", self.repetitions_per_minute);
        println!(
            "WPM:         {:.1}  (gross, chars/5 convention)",
            self.gross_wpm
        );
    }
}

pub struct Report {
    pub sentence: String,
    pub typed: String,
    pub elapsed: Duration,
    pub gross_wpm: f64,
    pub word_accuracy: f64,
    pub char_accuracy: f64,
}

/// Scores a completed attempt: `sentence` is what they were asked to type,
/// `typed` is what actually ended up submitted (after any chord
/// corrections have already landed in it), `elapsed` is the time from
/// "start typing" to pressing Enter.
pub fn score(sentence: &str, typed: &str, elapsed: Duration) -> Report {
    let typed_chars = typed.chars().count();
    // The standard typing-test convention: 5 characters = 1 "word", so WPM
    // is comparable across sentences of different average word length.
    let minutes = (elapsed.as_secs_f64() / 60.0).max(1.0 / 3600.0);
    let gross_wpm = (typed_chars as f64 / 5.0) / minutes;

    let target_words: Vec<&str> = sentence.split_whitespace().collect();
    let typed_words: Vec<&str> = typed.split_whitespace().collect();
    let matched = target_words
        .iter()
        .zip(typed_words.iter())
        .filter(|(a, b)| a == b)
        .count();
    let word_accuracy = if target_words.is_empty() {
        100.0
    } else {
        100.0 * matched as f64 / target_words.len() as f64
    };

    let dist = levenshtein(sentence, typed);
    let denom = sentence.chars().count().max(typed_chars).max(1);
    let char_accuracy = 100.0 * (1.0 - dist as f64 / denom as f64).max(0.0);

    Report {
        sentence: sentence.to_string(),
        typed: typed.to_string(),
        elapsed,
        gross_wpm,
        word_accuracy,
        char_accuracy,
    }
}

impl Report {
    pub fn print(&self, label: &str) {
        println!();
        println!("=== {label} ===");
        println!("target: {}", self.sentence);
        println!("typed:  {}", self.typed);
        println!("time:     {:.2}s", self.elapsed.as_secs_f64());
        println!(
            "WPM:      {:.1}  (gross, chars/5 convention)",
            self.gross_wpm
        );
        println!("word acc: {:.1}%", self.word_accuracy);
        println!("char acc: {:.1}%", self.char_accuracy);
    }
}

/// A per-segment timing breakdown of producing a two-word phrase
/// ("about me"), split into: S1 execute word 1; S2/S3 the space handling
/// (typing only — n/a when chording, which emits its own spaces); S4 the
/// full word-1-to-word-2 transition (= S2 + S3 when typing); S5 execute
/// word 2 plus the terminating Return.
pub struct SegmentReport {
    pub word1: String,
    pub word2: String,
    pub s1: Duration,
    pub s2: Option<Duration>,
    pub s3: Option<Duration>,
    pub s4: Duration,
    pub s5: Duration,
}

impl SegmentReport {
    /// Typing: word 1's first-letter press → its last-letter press (S1);
    /// last letter → space (S2); space → word 2's first letter (S3);
    /// their sum (S4); word 2's first letter → Return (S5).
    pub fn typing(
        word1: &str,
        word2: &str,
        w1_first: Instant,
        w1_last: Instant,
        space: Instant,
        w2_first: Instant,
        ret: Instant,
    ) -> Self {
        let s2 = space.saturating_duration_since(w1_last);
        let s3 = w2_first.saturating_duration_since(space);
        Self {
            word1: word1.to_string(),
            word2: word2.to_string(),
            s1: w1_last.saturating_duration_since(w1_first),
            s2: Some(s2),
            s3: Some(s3),
            s4: s2 + s3,
            s5: ret.saturating_duration_since(w2_first),
        }
    }

    /// Chording: word 1's first key-press → its last key-release (S1);
    /// that release → word 2's first key-press (S4, the transition);
    /// word 2's first key-press → Return (S5). No space segments.
    pub fn chording(
        word1: &str,
        word2: &str,
        w1_first_press: Instant,
        w1_last_release: Instant,
        w2_first_press: Instant,
        ret: Instant,
    ) -> Self {
        Self {
            word1: word1.to_string(),
            word2: word2.to_string(),
            s1: w1_last_release.saturating_duration_since(w1_first_press),
            s2: None,
            s3: None,
            s4: w2_first_press.saturating_duration_since(w1_last_release),
            s5: ret.saturating_duration_since(w2_first_press),
        }
    }

    pub fn total(&self) -> Duration {
        self.s1 + self.s4 + self.s5
    }

    pub fn print(&self, label: &str) {
        let opt = |d: Option<Duration>| match d {
            Some(d) => format!("{:.3}s", d.as_secs_f64()),
            None => "n/a".to_string(),
        };
        println!();
        println!(
            "=== {label}: \"{}\" + \"{}\" segments ===",
            self.word1, self.word2
        );
        println!(
            "S1 execute \"{}\"        : {:.3}s",
            self.word1,
            self.s1.as_secs_f64()
        );
        println!("S2 last letter -> space  : {}", opt(self.s2));
        println!("S3 space -> next word    : {}", opt(self.s3));
        println!("S4 transition            : {:.3}s", self.s4.as_secs_f64());
        println!(
            "S5 execute \"{}\" + Return : {:.3}s",
            self.word2,
            self.s5.as_secs_f64()
        );
        println!(
            "total (S1+S4+S5)         : {:.3}s",
            self.total().as_secs_f64()
        );
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_segments_split_the_transition() {
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        // a@0 ... t@400 (S1=400), space@500 (S2=100), m@640 (S3=140), Return@850 (S5=210)
        let r = SegmentReport::typing("about", "me", at(0), at(400), at(500), at(640), at(850));
        assert_eq!(r.s1, Duration::from_millis(400));
        assert_eq!(r.s2, Some(Duration::from_millis(100)));
        assert_eq!(r.s3, Some(Duration::from_millis(140)));
        assert_eq!(r.s4, Duration::from_millis(240)); // S2 + S3
        assert_eq!(r.s5, Duration::from_millis(210));
        assert_eq!(r.total(), Duration::from_millis(850)); // S1 + S4 + S5
    }

    #[test]
    fn chording_segments_have_no_space_parts() {
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        // about: first press@0, last release@40 (S1=40); me first press@300
        // (S4=260); Return@360 (S5=60)
        let r = SegmentReport::chording("about", "me", at(0), at(40), at(300), at(360));
        assert_eq!(r.s1, Duration::from_millis(40));
        assert_eq!(r.s2, None);
        assert_eq!(r.s3, None);
        assert_eq!(r.s4, Duration::from_millis(260));
        assert_eq!(r.s5, Duration::from_millis(60));
    }

    #[test]
    fn perfect_typing_scores_100_percent_both_ways() {
        let r = score("the quick fox", "the quick fox", Duration::from_secs(3));
        assert_eq!(r.word_accuracy, 100.0);
        assert_eq!(r.char_accuracy, 100.0);
    }

    #[test]
    fn one_wrong_word_reduces_accuracy_proportionally() {
        let r = score("the quick fox", "the quikc fox", Duration::from_secs(3));
        assert!((r.word_accuracy - (200.0 / 3.0)).abs() < 0.01);
        assert!(r.char_accuracy > 0.0 && r.char_accuracy < 100.0);
    }

    #[test]
    fn wpm_uses_the_five_char_convention() {
        // 50 chars typed in exactly 1 minute -> 10 WPM.
        let typed = "a".repeat(50);
        let r = score(&typed, &typed, Duration::from_secs(60));
        assert!((r.gross_wpm - 10.0).abs() < 0.01);
    }

    #[test]
    fn repeat_drill_counts_only_exact_matches() {
        let r = score_repeated(
            &["about"],
            "about about abuot about",
            Duration::from_secs(30),
        );
        assert_eq!(r.attempts, 4);
        assert_eq!(r.correct, 3);
        assert!((r.accuracy - 75.0).abs() < 0.01);
    }

    #[test]
    fn repeat_drill_reps_per_minute() {
        // 10 correct repetitions in 30 seconds -> 20 reps/min.
        let typed = "about ".repeat(10);
        let r = score_repeated(&["about"], typed.trim(), Duration::from_secs(30));
        assert!((r.repetitions_per_minute - 20.0).abs() < 0.01);
    }

    #[test]
    fn repeat_drill_scores_multi_word_unit_positionally() {
        let r = score_repeated(
            &["about", "me"],
            "about me about me about",
            Duration::from_secs(15),
        );
        assert_eq!(r.attempts, 5);
        assert_eq!(r.correct, 5);
    }
}
