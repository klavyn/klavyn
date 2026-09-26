use std::time::Duration;

/// A default sample if the user doesn't supply their own --sentence. Chosen
/// to include a few chords likely to be in data/dictionary.en.csv's top few
/// thousand words, so a chorded run actually exercises chording.
pub const DEFAULT_SENTENCE: &str =
    "the quick fox and the lazy dog went to find a different life together";

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
}
