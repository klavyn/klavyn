use std::collections::HashMap;
use std::path::Path;

/// Maps a chord's letter-set (sorted, de-duplicated letters, e.g. "klo") to
/// the word it should expand to (e.g. "look").
///
/// Plain QWERTY keys carry no press-order or press-direction information the
/// way CharaChorder's switches do, so a chord here is fundamentally a *set*:
/// "stop", "pots", "tops", and "spot" are indistinguishable chords. The
/// dictionary is built (see scripts/build_dictionary.py) by keeping only the
/// highest-frequency word per letter-set and dropping the rest.
pub struct Dictionary {
    by_letterset: HashMap<String, String>,
}

impl Dictionary {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let mut reader = csv::Reader::from_path(path)?;
        let mut by_letterset = HashMap::new();
        for record in reader.records() {
            let record = record?;
            let letterset = record.get(0).unwrap_or_default().to_string();
            let word = record.get(1).unwrap_or_default().to_string();
            if !letterset.is_empty() && !word.is_empty() {
                by_letterset.insert(letterset, word);
            }
        }
        Ok(Self { by_letterset })
    }

    pub fn len(&self) -> usize {
        self.by_letterset.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_letterset.is_empty()
    }

    /// Looks up the word for a burst of pressed letters, order-independent.
    pub fn lookup(&self, letters: &[char]) -> Option<&str> {
        let key = letterset(letters);
        self.by_letterset.get(&key).map(String::as_str)
    }
}

fn letterset(letters: &[char]) -> String {
    let mut chars: Vec<char> = letters.to_vec();
    chars.sort_unstable();
    chars.dedup();
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dict(rows: &[(&str, &str)]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "letterset,word,frequency").unwrap();
        for (ls, word) in rows {
            writeln!(f, "{ls},{word},1.0").unwrap();
        }
        f
    }

    #[test]
    fn looks_up_regardless_of_press_order() {
        let f = temp_dict(&[("klo", "look")]);
        let dict = Dictionary::load(f.path()).unwrap();
        assert_eq!(dict.lookup(&['l', 'o', 'k']), Some("look"));
        assert_eq!(dict.lookup(&['k', 'o', 'l']), Some("look"));
        assert_eq!(dict.lookup(&['o', 'o', 'l', 'k']), Some("look")); // repeated letter, still matches
    }

    #[test]
    fn unknown_chord_returns_none() {
        let f = temp_dict(&[("klo", "look")]);
        let dict = Dictionary::load(f.path()).unwrap();
        assert_eq!(dict.lookup(&['x', 'y', 'z']), None);
    }
}
