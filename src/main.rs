mod dictionary;
mod inject;
mod matcher;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clap::Parser;
use rdev::{Event, EventType, Key as RdevKey};

use dictionary::Dictionary;
use inject::Injector;
use matcher::ChordBuffer;

/// korder: a QWERTY chorded-typing trainer. Press a word's letters at once
/// (in any order) and korder replaces them with the whole word — the same
/// idea as CharaChorder, approximated on a normal keyboard.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to the chord dictionary CSV (letterset,word,frequency).
    #[arg(long, default_value = "data/dictionary.csv")]
    dictionary: PathBuf,

    /// Max spread (ms) between a chord's first and last key-press for it to
    /// count as "simultaneous" rather than sequential typing.
    #[arg(long, default_value_t = 150)]
    window_ms: u64,

    /// Log detected chords but don't actually backspace/replace anything.
    /// Still requires macOS Input Monitoring permission (to listen), but
    /// not Accessibility (to inject) — a safe way to try it first.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let dict = Dictionary::load(&args.dictionary)?;
    if dict.is_empty() {
        anyhow::bail!("dictionary at {:?} loaded 0 chords", args.dictionary);
    }
    eprintln!(
        "korder: loaded {} chords from {:?}",
        dict.len(),
        args.dictionary
    );
    if args.dry_run {
        eprintln!("korder: --dry-run, will only log detected chords");
    }
    eprintln!("korder: listening globally. Ctrl+C to quit.");

    let mut injector = if args.dry_run {
        None
    } else {
        Some(Injector::new()?)
    };
    let mut buffer = ChordBuffer::new(Duration::from_millis(args.window_ms));

    // rdev::listen's callback must be 'static and non-blocking-ish; do the
    // real work on the calling thread via a channel so we're not doing
    // dictionary lookups / enigo calls from inside the OS event tap itself.
    let (tx, rx) = mpsc::channel::<Event>();
    std::thread::spawn(move || {
        if let Err(e) = rdev::listen(move |event| {
            let _ = tx.send(event);
        }) {
            eprintln!("korder: listen error: {e:?}");
        }
    });

    for event in rx {
        let now = Instant::now();
        match event.event_type {
            EventType::KeyPress(key) => {
                if let Some(c) = key_to_letter(key) {
                    buffer.key_down(c, now);
                }
            }
            EventType::KeyRelease(key) => {
                if let Some(c) = key_to_letter(key) {
                    if let Some(burst) = buffer.key_up(c) {
                        if let Some(word) = dict.lookup(&burst.letters) {
                            eprintln!(
                                "korder: chord {:?} -> \"{word}\"",
                                sorted_display(&burst.letters)
                            );
                            if let Some(injector) = injector.as_mut() {
                                if let Err(e) = injector.replace(burst.press_count, word) {
                                    eprintln!("korder: injection failed: {e}");
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn sorted_display(letters: &[char]) -> String {
    let mut v = letters.to_vec();
    v.sort_unstable();
    v.into_iter().collect()
}

/// Maps rdev's key enum to a lowercase ascii letter, for the ~26 keys we
/// care about. Everything else (digits, punctuation, modifiers, arrows...)
/// is intentionally ignored by returning None.
fn key_to_letter(key: RdevKey) -> Option<char> {
    use RdevKey::*;
    let c = match key {
        KeyA => 'a',
        KeyB => 'b',
        KeyC => 'c',
        KeyD => 'd',
        KeyE => 'e',
        KeyF => 'f',
        KeyG => 'g',
        KeyH => 'h',
        KeyI => 'i',
        KeyJ => 'j',
        KeyK => 'k',
        KeyL => 'l',
        KeyM => 'm',
        KeyN => 'n',
        KeyO => 'o',
        KeyP => 'p',
        KeyQ => 'q',
        KeyR => 'r',
        KeyS => 's',
        KeyT => 't',
        KeyU => 'u',
        KeyV => 'v',
        KeyW => 'w',
        KeyX => 'x',
        KeyY => 'y',
        KeyZ => 'z',
        _ => return None,
    };
    Some(c)
}
