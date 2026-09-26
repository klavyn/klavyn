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
        let mut dict = Self {
            by_letterset: HashMap::new(),
        };
        dict.merge_overlay(path)?;
        Ok(dict)
    }

    /// Loads a second CSV (same letterset,word,... format) on top of this
    /// one. Entries here win on a letterset collision — meant for a
    /// hand-curated abbreviation tier (e.g. "bc" -> "because") sitting on
    /// top of the auto-generated spelling dictionary, since a mnemonic
    /// code isn't derived from a word's letters the way the base
    /// dictionary's entries are, so it can't be generated the same way.
    pub fn merge_overlay(&mut self, path: &Path) -> anyhow::Result<()> {
        let mut reader = csv::Reader::from_path(path)?;
        for record in reader.records() {
            let record = record?;
            let letterset = record.get(0).unwrap_or_default().to_string();
            let word = record.get(1).unwrap_or_default().to_string();
            if !letterset.is_empty() && !word.is_empty() {
                self.by_letterset.insert(letterset, word);
            }
        }
        Ok(())
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

    #[test]
    fn overlay_wins_on_collision_but_leaves_other_entries_alone() {
        let base = temp_dict(&[("klo", "look"), ("eht", "the")]);
        let mut dict = Dictionary::load(base.path()).unwrap();
        let abbrev = temp_dict(&[("klo", "kilo")]); // arbitrary curated code, unrelated spelling
        dict.merge_overlay(abbrev.path()).unwrap();
        assert_eq!(dict.lookup(&['l', 'o', 'k']), Some("kilo"));
        assert_eq!(dict.lookup(&['e', 'h', 't']), Some("the"));
    }
}
