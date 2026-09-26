//! WASM bindings for acchordion-core.
//!
//! The browser can't do global key capture the way the native CLI does —
//! and shouldn't. Here the page's own textarea provides keydown/keyup
//! within a sandbox, JS tracks which keys are held, and on release it asks
//! this module which word the held letter-set resolves to. Same core
//! dictionary as the native tool, no OS access.

use acchordion_core::dictionary::Dictionary;
use wasm_bindgen::prelude::*;

/// The dictionaries are compiled into the WASM binary — a browser has no
/// filesystem to read the CSVs from at runtime.
const BASE_CSV: &str = include_str!("../../data/dictionary.en.csv");
const ABBREV_CSV: &str = include_str!("../../data/abbrev.en.csv");

#[wasm_bindgen]
pub struct Trainer {
    dict: Dictionary,
}

#[wasm_bindgen]
impl Trainer {
    /// Builds the English dictionary with the abbreviation tier merged on
    /// top, ready for the in-page demo.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Trainer, JsError> {
        let mut dict =
            Dictionary::from_csv_str(BASE_CSV).map_err(|e| JsError::new(&e.to_string()))?;
        dict.merge_overlay_str(ABBREV_CSV)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Trainer { dict })
    }

    /// Number of chords loaded (base + abbreviations).
    pub fn size(&self) -> usize {
        self.dict.len()
    }

    /// Resolves a set of simultaneously-held letters to a word, order- and
    /// duplicate-independent (the core folds them to a letter-set), or
    /// returns undefined if nothing matches. Non-letters are ignored.
    pub fn lookup(&self, letters: &str) -> Option<String> {
        let chars: Vec<char> = letters
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        self.dict.lookup(&chars).map(str::to_string)
    }
}
