mod benchmark;
mod dictionary;
mod inject;
mod matcher;

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
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
    /// Produce a two-word phrase once ("about me") and get a per-segment
    /// timing breakdown (word 1, the word-to-word transition, word 2 +
    /// Return). Run with and without --chording to compare where the time
    /// actually goes.
    Segments(SegmentsArgs),
}

#[derive(Args, Clone)]
struct SegmentsArgs {
    /// The two words to produce, space-separated.
    #[arg(long, default_value = "about me")]
    phrase: String,

    /// Chord both words instead of typing them. Chord word 1 (press its
    /// keys together, release), immediately chord word 2, then Return — no
    /// pause, no manual spaces (korder emits those). Each word commits when
    /// you release its keys, so fully release word 1 before pressing word 2
    /// or the two merge into one unresolvable burst. Ignored with --compare.
    #[arg(long)]
    chording: bool,

    /// Run all three categories, --trials times each, and print a
    /// comparison table: (1) chord+chord, (2) chord word 1 + type word 2,
    /// (3) type+type. Directly shows whether chording a short second word
    /// actually beats typing it.
    #[arg(long)]
    compare: bool,

    /// Number of valid attempts to average each category over.
    #[arg(long, default_value_t = 1)]
    trials: u32,

    #[arg(long, default_value = "data/dictionary.en.csv")]
    dictionary: PathBuf,

    #[arg(long)]
    abbrev: Option<PathBuf>,

    #[arg(long, default_value_t = 200)]
    roll_gap_ms: u64,
}

#[derive(Args, Clone)]
struct BenchmarkArgs {
    /// Sentence to type. Defaults to a short built-in sample. Ignored if
    /// --repeat-phrase is set.
    #[arg(long)]
    sentence: Option<String>,

    /// Instead of one fixed sentence, type this phrase (one or more words)
    /// on repeat until the time limit hits. A multi-word phrase (e.g.
    /// "about me") forces your fingers to actually move between reps,
    /// rather than repeating one static hand shape. Scores correct
    /// repetitions and reps/minute, not a fixed-sentence match.
    #[arg(long)]
    repeat_phrase: Option<String>,

    /// Hard time limit in seconds for --repeat-phrase: korder injects a
    /// synthetic Enter at this deadline, so you don't have to self-time
    /// and remember to stop. Ignored for plain --sentence mode.
    #[arg(long, default_value_t = 15)]
    duration_secs: u64,

    /// Run this many isolated trials of a single word (use with --word)
    /// instead of one sentence or a continuous repeat drill. Each trial:
    /// type the word once, press Enter. Timed from your first keystroke
    /// (not the "ready" prompt) to that Enter. Avoids --repeat-phrase's
    /// failure mode of separate attempts bleeding into one unresolvable
    /// burst when you switch words faster than the roll gap.
    #[arg(long)]
    trials: Option<u32>,

    /// The word to use with --trials.
    #[arg(long)]
    word: Option<String>,

    /// Run korder's chord engine in the background during the timed
    /// attempt, so chording the sentence's words gets corrected live (same
    /// as `korder run`). Omit this to measure your normal, un-chorded
    /// typing as a baseline.
    #[arg(long)]
    chording: bool,

    /// Dictionary to use when --chording is set.
    #[arg(long, default_value = "data/dictionary.en.csv")]
    dictionary: PathBuf,

    /// Optional abbreviation overlay when --chording is set (see RunArgs).
    #[arg(long)]
    abbrev: Option<PathBuf>,

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

    /// Optional hand-curated abbreviation CSV (same letterset,word format),
    /// merged on top of --dictionary — its entries win on collision. For
    /// short mnemonic codes that aren't derived from a word's own letters
    /// (e.g. "bc" -> "because"), which the auto-generated dictionary can't
    /// produce. See data/abbrev.en.csv.
    #[arg(long)]
    abbrev: Option<PathBuf>,

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
        Some(Commands::Segments(args)) => cmd_segments(args),
        None => run(cli.run_args),
    }
}

fn run(args: RunArgs) -> anyhow::Result<()> {
    let mut dict = Dictionary::load(&args.dictionary)?;
    if dict.is_empty() {
        anyhow::bail!("dictionary at {:?} loaded 0 chords", args.dictionary);
    }
    eprintln!(
        "korder: loaded {} chords from {:?}",
        dict.len(),
        args.dictionary
    );
    if let Some(abbrev) = &args.abbrev {
        dict.merge_overlay(abbrev)?;
        eprintln!(
            "korder: merged abbreviations from {abbrev:?} (now {} chords)",
            dict.len()
        );
    }
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
    if args.trials.is_some() {
        return cmd_benchmark_trials(args);
    }

    let sentence = args
        .sentence
        .clone()
        .unwrap_or_else(|| benchmark::DEFAULT_SENTENCE.to_string());

    start_chord_engine_if_requested(&args)?;

    if let Some(phrase) = &args.repeat_phrase {
        println!(
            "\nType \"{phrase}\" repeatedly, separated by spaces. \
             Stops itself after {}s — just keep typing until it does:\n",
            args.duration_secs
        );
    } else {
        println!("\nType this sentence exactly, then press Enter:\n");
        println!("  {sentence}\n");
    }
    print!("Press Enter when you're ready to start... ");
    io::stdout().flush()?;
    let mut throwaway = String::new();
    io::stdin().read_line(&mut throwaway)?;

    // For repeat-phrase drills, force-submit at the deadline via a
    // synthetic Enter, rather than relying on the user to self-time and
    // remember to stop (which is exactly what produced 23s instead of 15s
    // in an earlier manual run).
    if args.repeat_phrase.is_some() {
        let duration_secs = args.duration_secs;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(duration_secs));
            match Injector::new() {
                Ok(mut injector) => {
                    if let Err(e) = injector.press_enter() {
                        eprintln!("korder: couldn't auto-submit: {e}");
                    }
                }
                Err(e) => eprintln!("korder: couldn't auto-submit: {e}"),
            }
        });
    }

    let start = Instant::now();
    let mut typed = String::new();
    io::stdin().read_line(&mut typed)?;
    let elapsed = start.elapsed();
    let typed = typed.trim_end_matches(['\n', '\r']);

    let label = if args.chording { "chorded" } else { "raw" };
    if let Some(phrase) = &args.repeat_phrase {
        let unit: Vec<&str> = phrase.split_whitespace().collect();
        benchmark::score_repeated(&unit, typed, elapsed).print(label);
    } else {
        benchmark::score(&sentence, typed, elapsed).print(label);
    }
    Ok(())
}

/// Starts the chord engine in the background if `--chording` was passed,
/// shared by both benchmark modes (one fixed sentence/repeat-phrase, and
/// discrete trials).
fn start_chord_engine_if_requested(args: &BenchmarkArgs) -> anyhow::Result<()> {
    if !args.chording {
        println!("(chording OFF — baseline)");
        return Ok(());
    }
    let mut dict = Dictionary::load(&args.dictionary)?;
    if let Some(abbrev) = &args.abbrev {
        dict.merge_overlay(abbrev)?;
    }
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
    Ok(())
}

/// A committed chord reported by the benchmark chord loop to the trial
/// runner: the word it resolved to, and how long the chord itself took to
/// execute (first letter-press to last letter-press of the burst, so it
/// excludes both pre-chord reaction time and the post-release roll-gap /
/// delimiter wait).
struct ChordSpan {
    word: String,
    span: Duration,
}

/// Runs `args.trials` isolated reps of producing `args.word` once each.
///
/// The two modes measure different mechanics but report the same thing —
/// per-trial time to produce the target word, plus a hit/miss — so raw
/// and chorded runs stay comparable:
///
/// - Raw: time from your first keystroke to Enter, matched against the
///   typed line. A private per-trial listener captures first-keystroke
///   timing and any backspaces.
/// - Chorded: the typed line can't be used — your Enter both ends the
///   trial and commits the chord, and korder's correction lands *after*
///   the line is submitted, unreachable via stdin. So the chord engine
///   itself reports what it resolved and how long the burst took, over a
///   channel; stdin is read only to consume your Enter and advance.
fn cmd_benchmark_trials(args: BenchmarkArgs) -> anyhow::Result<()> {
    let n = args.trials.unwrap();
    let word = args
        .word
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--trials requires --word"))?;

    if args.chording {
        run_chorded_trials(&args, &word, n)
    } else {
        run_raw_trials(&word, n)
    }
}

fn run_raw_trials(word: &str, n: u32) -> anyhow::Result<()> {
    println!("(chording OFF — baseline)");

    // One persistent listener, warmed up once before trial 1. A fresh
    // rdev::listen() per trial needs a moment to register with the OS, and
    // firing the first keystroke during that gap dropped it (trial 1's
    // timing anchored late, collapsing its span toward zero).
    let (tx, event_rx) = mpsc::channel::<(Instant, EventType)>();
    std::thread::spawn(move || {
        let rx = spawn_listener();
        for event in rx {
            let at = Instant::now();
            if tx.send((at, event.event_type)).is_err() {
                return;
            }
        }
    });
    warm_up_listener();

    let mut trials = Vec::with_capacity(n as usize);
    for i in 1..=n {
        while event_rx.try_recv().is_ok() {} // discard events from before this trial
        print!("\nTrial {i}/{n}: type \"{word}\", then Enter... ");
        io::stdout().flush()?;

        let mut typed = String::new();
        io::stdin().read_line(&mut typed)?;
        let end = Instant::now();
        std::thread::sleep(Duration::from_millis(30)); // let the last keystroke arrive

        let mut first_key = None;
        let mut backspaces = 0usize;
        while let Ok((at, event_type)) = event_rx.try_recv() {
            if let EventType::KeyPress(key) = event_type {
                first_key.get_or_insert(at);
                if key == RdevKey::Backspace {
                    backspaces += 1;
                }
            }
        }
        let elapsed = end.saturating_duration_since(first_key.unwrap_or(end));
        let typed = typed.trim_end_matches(['\n', '\r']).trim();
        trials.push(benchmark::Trial {
            elapsed,
            matched: typed == word,
            backspaces,
        });
    }
    benchmark::TrialsReport {
        word: word.to_string(),
        trials,
    }
    .print("raw");
    Ok(())
}

/// Give a freshly-spawned rdev listener time to register with the OS
/// before the first timed trial, so an eager first keystroke isn't
/// dropped. rdev exposes no readiness signal, so a short fixed wait is
/// the pragmatic guard.
fn warm_up_listener() {
    std::thread::sleep(Duration::from_millis(400));
}

fn run_chorded_trials(args: &BenchmarkArgs, word: &str, n: u32) -> anyhow::Result<()> {
    let mut dict = Dictionary::load(&args.dictionary)?;
    if let Some(abbrev) = &args.abbrev {
        dict.merge_overlay(abbrev)?;
    }
    if dict.is_empty() {
        anyhow::bail!("dictionary at {:?} loaded 0 chords", args.dictionary);
    }
    let injector = Injector::new()?;
    let roll_gap_ms = args.roll_gap_ms;
    let (span_tx, span_rx) = mpsc::channel::<ChordSpan>();
    // Set true at each trial boundary; the engine clears its burst timing
    // and buffer on the next event, so no keypress from a prior trial (or
    // the reset-to-home-row movement between them) can leak into this
    // trial's span, and two rapid trials can't bleed into one burst.
    let reset = Arc::new(AtomicBool::new(false));
    let engine_reset = Arc::clone(&reset);
    std::thread::spawn(move || {
        if let Err(e) = benchmark_chord_loop(dict, roll_gap_ms, injector, span_tx, engine_reset) {
            eprintln!("korder: chord engine stopped: {e}");
        }
    });
    println!("(chording ON)");
    warm_up_listener(); // don't let an eager trial-1 chord fire before the engine is live

    let mut trials = Vec::with_capacity(n as usize);
    for i in 1..=n {
        reset.store(true, Ordering::SeqCst);
        while span_rx.try_recv().is_ok() {} // drop any spans from before this trial

        print!("\nTrial {i}/{n}: chord \"{word}\", then Enter... ");
        io::stdout().flush()?;

        let mut typed = String::new();
        io::stdin().read_line(&mut typed)?;
        std::thread::sleep(Duration::from_millis(50)); // let the Enter-triggered commit report

        // The last span reported this trial is the chord that just
        // committed; earlier ones would be stray extra chords in the same
        // line (the user was asked for exactly one).
        let mut last_span: Option<ChordSpan> = None;
        while let Ok(span) = span_rx.try_recv() {
            last_span = Some(span);
        }
        let (elapsed, matched) = match last_span {
            Some(s) => (s.span, s.word == word),
            None => (Duration::ZERO, false),
        };
        trials.push(benchmark::Trial {
            elapsed,
            matched,
            backspaces: 0, // in chorded mode korder does the correcting, not the user
        });
    }
    benchmark::TrialsReport {
        word: word.to_string(),
        trials,
    }
    .print("chorded");
    Ok(())
}

/// Chord loop that reports each committed, matched chord over `span_tx`
/// with the burst's first-press-to-last-press duration. See
/// `cmd_benchmark_trials` for why chorded trials measure this rather than
/// the typed line.
fn benchmark_chord_loop(
    dict: Dictionary,
    roll_gap_ms: u64,
    mut injector: Injector,
    span_tx: mpsc::Sender<ChordSpan>,
    reset: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut buffer = ChordBuffer::new(Duration::from_millis(roll_gap_ms));
    let mut burst_first: Option<Instant> = None;
    let mut burst_last = Instant::now();
    let rx = spawn_listener();
    for event in rx {
        // Clear all carried state at a trial boundary before touching this
        // event, so the first letter of the new trial anchors burst_first
        // fresh and no prior-trial chord is still pending in the buffer.
        if reset.swap(false, Ordering::SeqCst) {
            buffer = ChordBuffer::new(Duration::from_millis(roll_gap_ms));
            burst_first = None;
        }
        let now = Instant::now();
        match event.event_type {
            EventType::KeyPress(key) => match key_to_letter(key) {
                Some(c) => {
                    burst_first.get_or_insert(now);
                    burst_last = now;
                    buffer.key_down(c, now);
                }
                None => {
                    if let Some(burst) = buffer.flush_now() {
                        if let Some(word) = handle_burst(&burst, &dict, Some(&mut injector), true) {
                            let span = burst_last
                                .saturating_duration_since(burst_first.unwrap_or(burst_last));
                            let _ = span_tx.send(ChordSpan { word, span });
                        }
                    }
                    burst_first = None;
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
    anyhow::bail!("listener thread ended unexpectedly")
}

fn cmd_segments(args: SegmentsArgs) -> anyhow::Result<()> {
    let words: Vec<String> = args.phrase.split_whitespace().map(str::to_string).collect();
    let [w1, w2] = <[String; 2]>::try_from(words)
        .map_err(|_| anyhow::anyhow!("--phrase must be exactly two words"))?;
    let n = args.trials.max(1);

    let mut dict = Dictionary::load(&args.dictionary)?;
    if let Some(abbrev) = &args.abbrev {
        dict.merge_overlay(abbrev)?;
    }
    if dict.is_empty() {
        anyhow::bail!("dictionary at {:?} loaded 0 chords", args.dictionary);
    }
    let roll_gap_ms = args.roll_gap_ms;
    let (tx, seg_rx) = mpsc::channel::<SegEvent>();
    // One unified listener for the whole session. It forwards every letter
    // and space press (for typed-word segmentation) and also emits a Chord
    // event whenever a released key-group resolves to a word (for chorded
    // segmentation). No injection — the benchmark only measures key timing,
    // it doesn't correct the screen.
    std::thread::spawn(move || {
        if let Err(e) = segments_event_loop(dict, roll_gap_ms, tx) {
            eprintln!("korder: chord engine stopped: {e}");
        }
    });
    warm_up_listener();

    let categories = if args.compare {
        vec![
            Category::ChordChord,
            Category::ChordType,
            Category::TypeType,
        ]
    } else if args.chording {
        vec![Category::ChordChord]
    } else {
        vec![Category::TypeType]
    };

    let compare = categories.len() > 1;
    let mut results: Vec<(Category, benchmark::AvgSegments)> = Vec::new();
    for category in categories {
        let reports = run_category(&seg_rx, category, &w1, &w2, n)?;
        if let Some(avg) = benchmark::AvgSegments::of(&reports) {
            results.push((category, avg));
        }
    }

    if compare {
        println!("\n=== segments (phrase \"{w1} {w2}\", N={n}, avg seconds) ===");
        println!(
            "{:<16} {:>7} {:>9} {:>10} {:>8}",
            "", "S1(w1)", "S4(tr)", "S5(w2+ret)", "total"
        );
        for (category, avg) in &results {
            avg.print_row(category.label());
        }
    } else if let Some((category, avg)) = results.first() {
        avg.print_block(category.label());
    }
    Ok(())
}

#[derive(Copy, Clone)]
enum Category {
    /// Chord word 1, chord word 2, Return.
    ChordChord,
    /// Chord word 1, type word 2, Return.
    ChordType,
    /// Type word 1, space, type word 2, Return.
    TypeType,
}

impl Category {
    fn label(&self) -> &'static str {
        match self {
            Category::ChordChord => "1 chord+chord",
            Category::ChordType => "2 chord+type",
            Category::TypeType => "3 type+type",
        }
    }

    fn prompt(&self, w1: &str, w2: &str) -> String {
        match self {
            Category::ChordChord => {
                format!("chord \"{w1}\", then chord \"{w2}\" (no pause), Enter")
            }
            Category::ChordType => format!("chord \"{w1}\", then TYPE \"{w2}\", Enter"),
            Category::TypeType => format!("type \"{w1} {w2}\", Enter"),
        }
    }

    /// Turns one attempt's captured events into a SegmentReport, or an Err
    /// message explaining why the attempt was invalid (for a retry).
    fn extract(
        &self,
        events: &[SegEvent],
        w1: &str,
        w2: &str,
    ) -> Result<benchmark::SegmentReport, String> {
        let ret = events
            .iter()
            .find_map(|e| match e {
                SegEvent::Return { at } => Some(*at),
                _ => None,
            })
            .ok_or_else(|| "no Return seen — press Enter to finish.".to_string())?;
        match self {
            Category::ChordChord => {
                let chords: Vec<&SegEvent> = events
                    .iter()
                    .filter(|e| matches!(e, SegEvent::Chord { .. }))
                    .collect();
                let got: Vec<&str> = chords.iter().map(|e| chord_parts(e).0).collect();
                if got != [w1, w2] {
                    return Err(format!(
                        "got chords {got:?}, expected [{w1}, {w2}] — press each word's keys together as one chord (a rolled 2-key word splits into misses)."
                    ));
                }
                let (_, f1, l1) = chord_parts(chords[0]);
                let (_, f2, _) = chord_parts(chords[1]);
                Ok(benchmark::SegmentReport::chording(f1, l1, f2, ret))
            }
            Category::ChordType => {
                let first_chord = events.iter().find(|e| matches!(e, SegEvent::Chord { .. }));
                let Some(chord) = first_chord else {
                    return Err(format!(
                        "no chord for \"{w1}\" — chord word 1 as one simultaneous press."
                    ));
                };
                let (word, f1, l1) = chord_parts(chord);
                if word != w1 {
                    return Err(format!("first chord was \"{word}\", expected \"{w1}\"."));
                }
                // Word 2 is typed: its letters are the letter presses after
                // word 1's chord released.
                let typed_w2: Vec<(char, Instant)> = events
                    .iter()
                    .filter_map(|e| match e {
                        SegEvent::Letter { c, at } if *at > l1 => Some((*c, *at)),
                        _ => None,
                    })
                    .collect();
                let spelled: String = typed_w2.iter().map(|(c, _)| c).collect();
                if spelled != w2 {
                    return Err(format!(
                        "typed \"{spelled}\" after the chord, expected \"{w2}\" — type word 2 letter by letter."
                    ));
                }
                let f2 = typed_w2[0].1;
                Ok(benchmark::SegmentReport::chording(f1, l1, f2, ret))
            }
            Category::TypeType => {
                // Letters and the single space, in press order.
                let mut seq: Vec<(Instant, Option<char>)> = events
                    .iter()
                    .filter_map(|e| match e {
                        SegEvent::Letter { c, at } => Some((*at, Some(*c))),
                        SegEvent::Space { at } => Some((*at, None)),
                        _ => None,
                    })
                    .collect();
                seq.sort_by_key(|(at, _)| *at);
                let expected: Vec<Option<char>> = w1
                    .chars()
                    .map(Some)
                    .chain(std::iter::once(None))
                    .chain(w2.chars().map(Some))
                    .collect();
                let observed: Vec<Option<char>> = seq.iter().map(|(_, c)| *c).collect();
                if observed != expected {
                    return Err(format!(
                        "keystrokes didn't match \"{w1} {w2}\" exactly (a typo or backspace makes the boundaries meaningless)."
                    ));
                }
                let i = w1.chars().count();
                Ok(benchmark::SegmentReport::typing(
                    seq[0].0,
                    seq[i - 1].0,
                    seq[i].0,
                    seq[i + 1].0,
                    ret,
                ))
            }
        }
    }
}

fn chord_parts(e: &SegEvent) -> (&str, Instant, Instant) {
    match e {
        SegEvent::Chord {
            word,
            first_press,
            last_release,
        } => (word.as_str(), *first_press, *last_release),
        _ => unreachable!("chord_parts called on a non-Chord event"),
    }
}

/// Prompts `n` valid attempts of one category, retrying (in place, with a
/// reason) on any invalid attempt, and returns the per-attempt reports.
fn run_category(
    seg_rx: &mpsc::Receiver<SegEvent>,
    category: Category,
    w1: &str,
    w2: &str,
    n: u32,
) -> anyhow::Result<Vec<benchmark::SegmentReport>> {
    println!("\n--- {} (x{n}) ---", category.label());
    let mut reports = Vec::with_capacity(n as usize);
    while (reports.len() as u32) < n {
        while seg_rx.try_recv().is_ok() {} // clear stale events before this attempt
        print!(
            "[{}/{n}] {}\n> ",
            reports.len() + 1,
            category.prompt(w1, w2)
        );
        io::stdout().flush()?;
        let mut typed = String::new();
        io::stdin().read_line(&mut typed)?;
        std::thread::sleep(Duration::from_millis(80)); // let the Return-triggered commit report

        let mut events = Vec::new();
        while let Ok(ev) = seg_rx.try_recv() {
            events.push(ev);
        }
        match category.extract(&events, w1, w2) {
            Ok(report) => reports.push(report),
            Err(reason) => println!("  {reason} Retrying."),
        }
    }
    Ok(reports)
}

/// A raw key press (for typed-word segmentation), a resolved chord (for
/// chorded segmentation), or the terminating Return, timestamped.
enum SegEvent {
    Letter {
        c: char,
        at: Instant,
    },
    Space {
        at: Instant,
    },
    Chord {
        word: String,
        first_press: Instant,
        last_release: Instant,
    },
    Return {
        at: Instant,
    },
}

/// Unified segments listener. Forwards every letter/space press and the
/// Return, and — running the same release-to-commit chord detection as
/// before — emits a Chord event whenever a released key-group resolves to
/// a word. One loop serves all three categories; each reads the events it
/// needs. See Category::extract.
fn segments_event_loop(
    dict: Dictionary,
    roll_gap_ms: u64,
    tx: mpsc::Sender<SegEvent>,
) -> anyhow::Result<()> {
    let mut buffer = ChordBuffer::new(Duration::from_millis(roll_gap_ms));
    let mut held: HashSet<char> = HashSet::new();
    let mut burst_first: Option<Instant> = None;
    let rx = spawn_listener();
    for event in rx {
        let now = Instant::now();
        match event.event_type {
            EventType::KeyPress(key) if key == RdevKey::Return => {
                let _ = tx.send(SegEvent::Return { at: now });
            }
            EventType::KeyPress(key) if key == RdevKey::Space => {
                let _ = tx.send(SegEvent::Space { at: now });
            }
            EventType::KeyPress(key) => {
                if let Some(c) = key_to_letter(key) {
                    let _ = tx.send(SegEvent::Letter { c, at: now });
                    burst_first.get_or_insert(now);
                    held.insert(c);
                    buffer.key_down(c, now);
                }
            }
            EventType::KeyRelease(key) => {
                if let Some(c) = key_to_letter(key) {
                    buffer.key_up(c, now);
                    held.remove(&c);
                    if held.is_empty() {
                        if let Some(burst) = buffer.flush_now() {
                            if let Some(word) = handle_burst(&burst, &dict, None, true) {
                                let _ = tx.send(SegEvent::Chord {
                                    word,
                                    first_press: burst_first.unwrap_or(now),
                                    last_release: now,
                                });
                            }
                        }
                        burst_first = None;
                    }
                }
            }
            _ => {}
        }
    }
    anyhow::bail!("listener thread ended unexpectedly")
}

/// Resolves and (optionally) injects a committed chord. Returns the
/// matched word, so a caller timing the chord can pair the result with a
/// span; returns None if the letter set matched nothing.
fn handle_burst(
    burst: &matcher::BurstResult,
    dict: &Dictionary,
    injector: Option<&mut Injector>,
    quiet: bool,
) -> Option<String> {
    let word = dict.lookup(&burst.letters)?.to_string();
    if !quiet {
        eprintln!(
            "korder: chord {:?} -> \"{word}\"",
            sorted_display(&burst.letters)
        );
    }
    if let Some(injector) = injector {
        if let Err(e) = injector.replace(burst.press_count, &word) {
            eprintln!("korder: injection failed: {e}");
        }
    }
    Some(word)
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
