use std::time::Duration;

/// A default sample if the user doesn't supply their own --sentence. Chosen
/// to include a few chords likely to be in data/dictionary.en.csv's top few
/// thousand words, so a chorded run actually exercises chording.
pub const DEFAULT_SENTENCE: &str =
    "the quick fox and the lazy dog went to find a different life together";

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
