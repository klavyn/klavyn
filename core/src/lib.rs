//! Pure chord-matching core for typhony.
//!
//! Everything here is OS-independent — no global key capture, no input
//! injection — so the same logic drives both the native CLI (behind
//! rdev/enigo) and the in-browser WASM demo (behind a textarea).

pub mod dictionary;
pub mod matcher;
