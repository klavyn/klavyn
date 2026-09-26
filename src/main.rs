mod benchmark;
mod dictionary;
mod inject;
mod matcher;

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand};
use rdev::{Event, EventType, Key as RdevKey};

use dictionary::Dictionary;
use inject::Injector;
use matcher::ChordBuffer;

/// korder: a QWERTY chorded-typing trainer. Press a word's letters at once
/// or roll through them one at a time (in any order) and korder replaces
/// them with the whole word — the same idea as CharaChorder, approximated
/// on a normal keyboard.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    run_args: RunArgs,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the chord trainer (default if no subcommand is given).
    Run(RunArgs),
    /// Hold down as many letter keys as you can for a few seconds, to see
    /// how many your keyboard/OS can actually report at once. Purely
    /// informational — korder's roll/arpeggiate matching works regardless
    /// of this ceiling, but it's useful for knowing which words you'll
    /// need to roll rather than hold.
    DetectRollover,
    /// Type one sentence, timed, with chording on or off, and get WPM +
    /// accuracy. Run it once each way to compare.
    Benchmark(BenchmarkArgs),
}

#[derive(Args, Clone)]
struct BenchmarkArgs {
    /// Sentence to type. Defaults to a short built-in sample.
    #[arg(long)]
    sentence: Option<String>,

    /// Run korder's chord engine in the background during the timed
    /// attempt, so chording the sentence's words gets corrected live (same
    /// as `korder run`). Omit this to measure your normal, un-chorded
    /// typing as a baseline.
    #[arg(long)]
    chording: bool,

    /// Dictionary to use when --chording is set.
    #[arg(long, default_value = "data/dictionary.en.csv")]
    dictionary: PathBuf,

    /// Roll gap when --chording is set.
    #[arg(long, default_value_t = 200)]
    roll_gap_ms: u64,
}

#[derive(Args, Clone)]
struct RunArgs {
    /// Path to the chord dictionary CSV (letterset,word,frequency). See
    /// data/ for per-language files (dictionary.en.csv, dictionary.fr.csv,
    /// ...) built by scripts/build_dictionary.py.
    #[arg(long, default_value = "data/dictionary.en.csv")]
    dictionary: PathBuf,

    /// Max gap (ms), while nothing is held, before a new key press starts a
    /// *new* chord attempt instead of continuing the current one. This is
    /// what makes arpeggiation (rolling keys one at a time) work — set it
    /// higher if you're rolling long chords slowly, lower if short common
    /// words are getting falsely merged with the next word.
    #[arg(long, default_value_t = 200)]
    roll_gap_ms: u64,

    /// Log detected chords but don't actually backspace/replace anything.
    /// Still requires macOS Input Monitoring permission (to listen), but
    /// not Accessibility (to inject) — a safe way to try it first.
    #[arg(long)]
    dry_run: bool,

    /// Don't print recognized chords (the resulting word, not your raw
    /// keystrokes) to the terminal. See SECURITY.md: this output is never
    /// written to a file or sent anywhere by korder itself, but if you
    /// want zero terminal echo at all — e.g. on a shared or logged
    /// terminal session — this suppresses it.
    #[arg(long)]
    quiet: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::DetectRollover) => detect_rollover(),
        Some(Commands::Run(args)) => run(args),
        Some(Commands::Benchmark(args)) => cmd_benchmark(args),
        None => run(cli.run_args),
    }
}

fn run(args: RunArgs) -> anyhow::Result<()> {
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
    eprintln!(
        "korder: listening globally (roll gap {}ms). Ctrl+C to quit.",
        args.roll_gap_ms
    );

    let injector = if args.dry_run {
        None
    } else {
        Some(Injector::new()?)
    };
    chord_loop(dict, args.roll_gap_ms, injector, args.quiet)
}

/// The core listen -> buffer -> match -> (optionally) correct loop, shared
/// by `run` (blocks the main thread, for interactive use) and
/// `cmd_benchmark`'s chorded mode (spawned on a background thread — the
/// benchmark's own timing/capture happens on the main thread via stdin,
/// same as it would for any other focused app).
fn chord_loop(
    dict: Dictionary,
    roll_gap_ms: u64,
    mut injector: Option<Injector>,
    quiet: bool,
) -> anyhow::Result<()> {
    let mut buffer = ChordBuffer::new(Duration::from_millis(roll_gap_ms));
    let rx = spawn_listener();
    // A short recv timeout doubles as our idle-poll tick, so a chord commits
    // even if the user never presses another key (no trailing space, etc.).
    let tick = Duration::from_millis(50);
    loop {
        match rx.recv_timeout(tick) {
            Ok(event) => {
                let now = Instant::now();
                match event.event_type {
                    EventType::KeyPress(key) => match key_to_letter(key) {
                        Some(c) => buffer.key_down(c, now),
                        None => {
                            if let Some(burst) = buffer.flush_now() {
                                handle_burst(&burst, &dict, injector.as_mut(), quiet);
                            }
                        }
                    },
                    EventType::KeyRelease(key) => {
                        if let Some(c) = key_to_letter(key) {
                            buffer.key_up(c, now);
                        }
                    }
                    _ => {}
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(burst) = buffer.flush_if_idle(Instant::now()) {
                    handle_burst(&burst, &dict, injector.as_mut(), quiet);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("listener thread ended unexpectedly");
            }
        }
    }
}

/// Times one attempt at typing `args.sentence` (or the built-in default),
/// optionally with the chord engine running in the background.
///
/// This works by reusing the terminal's own line input rather than
/// reimplementing raw keystroke capture: whether or not chording is on,
/// whatever ends up in the submitted line — post-correction, if chording
/// fired — is exactly what `chord_loop`'s injector would have sent to any
/// other focused app, since a terminal is just another focused app to it.
fn cmd_benchmark(args: BenchmarkArgs) -> anyhow::Result<()> {
    let sentence = args
        .sentence
        .unwrap_or_else(|| benchmark::DEFAULT_SENTENCE.to_string());

    if args.chording {
        let dict = Dictionary::load(&args.dictionary)?;
        if dict.is_empty() {
            anyhow::bail!("dictionary at {:?} loaded 0 chords", args.dictionary);
        }
        let injector = Injector::new()?;
        let roll_gap_ms = args.roll_gap_ms;
        std::thread::spawn(move || {
            // Always quiet: printing "chord X -> Y" mid-benchmark would
            // clutter the timed prompt and give away words as they land.
            if let Err(e) = chord_loop(dict, roll_gap_ms, Some(injector), true) {
                eprintln!("korder: chord engine stopped: {e}");
            }
        });
        println!("(chording ON)");
    } else {
        println!("(chording OFF — baseline)");
    }

    println!("\nType this sentence exactly, then press Enter:\n");
    println!("  {sentence}\n");
    print!("Press Enter when you're ready to start... ");
    io::stdout().flush()?;
    let mut throwaway = String::new();
    io::stdin().read_line(&mut throwaway)?;

    let start = Instant::now();
    let mut typed = String::new();
    io::stdin().read_line(&mut typed)?;
    let elapsed = start.elapsed();
    let typed = typed.trim_end_matches(['\n', '\r']);

    let report = benchmark::score(&sentence, typed, elapsed);
    report.print(if args.chording { "chorded" } else { "raw" });
    Ok(())
}

fn handle_burst(
    burst: &matcher::BurstResult,
    dict: &Dictionary,
    injector: Option<&mut Injector>,
    quiet: bool,
) {
    let Some(word) = dict.lookup(&burst.letters) else {
        return;
    };
    if !quiet {
        eprintln!(
            "korder: chord {:?} -> \"{word}\"",
            sorted_display(&burst.letters)
        );
    }
    if let Some(injector) = injector {
        if let Err(e) = injector.replace(burst.press_count, word) {
            eprintln!("korder: injection failed: {e}");
        }
    }
}

fn detect_rollover() -> anyhow::Result<()> {
    eprintln!("korder: rollover self-test.");
    eprintln!("Hold down as many DIFFERENT letter keys as you comfortably can at once,");
    eprintln!("then release them. You have 6 seconds. Go:");

    let rx = spawn_listener();
    let deadline = Instant::now() + Duration::from_secs(6);
    let mut held: HashSet<char> = HashSet::new();
    let mut max_simultaneous = 0usize;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(event) => match event.event_type {
                EventType::KeyPress(key) => {
                    if let Some(c) = key_to_letter(key) {
                        held.insert(c);
                        max_simultaneous = max_simultaneous.max(held.len());
                    }
                }
                EventType::KeyRelease(key) => {
                    if let Some(c) = key_to_letter(key) {
                        held.remove(&c);
                    }
                }
                _ => {}
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    eprintln!();
    eprintln!("korder: max simultaneous letter keys observed: {max_simultaneous}");
    if max_simultaneous <= 6 {
        eprintln!(
            "This looks like a typical 6-key-rollover (or lower) keyboard. Words needing more \
             unique letters than that will need to be rolled (pressed one/two at a time, \
             quickly) rather than held all at once — korder's --roll-gap-ms handles this \
             automatically, no configuration needed."
        );
    } else {
        eprintln!(
            "That's a real NKRO-capable rollover — you can likely hold most chords fully \
             simultaneously without needing to roll them."
        );
    }
    eprintln!(
        "Note: this measures what your OS/keyboard combo actually delivers end-to-end, which \
         is what matters for korder — not a theoretical spec-sheet number."
    );
    Ok(())
}

fn spawn_listener() -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel::<Event>();
    std::thread::spawn(move || {
        if let Err(e) = rdev::listen(move |event| {
            let _ = tx.send(event);
        }) {
            eprintln!("korder: listen error: {e:?}");
        }
    });
    rx
}

fn sorted_display(letters: &[char]) -> String {
    let mut v = letters.to_vec();
    v.sort_unstable();
    v.into_iter().collect()
}

/// Maps rdev's key enum to a lowercase ascii letter, for the ~26 keys we
/// care about. Everything else (digits, punctuation, modifiers, arrows...)
/// is intentionally ignored by returning None. rdev's `Key` enum is
/// already OS-abstracted (same variants on macOS/Linux/Windows), so this
/// mapping needs no per-platform branches.
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
