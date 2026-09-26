# typhony roadmap

Planning doc for the two in-flight tracks: the demo website and the config/macro system.
Snapshot date: 2026-09-26.

## Where things stand

The native CLI works and covers a full suite of measurement tools:

- `run` — the live chord trainer (rdev listener + enigo injector), with held and arpeggiated/rolled chord entry.
- `detect-rollover` — empirical N-key-rollover self-test.
- `benchmark` — sentence WPM/accuracy, `--repeat-phrase`, `--trials`, discrete `--trials/--word`.
- `segments` — per-segment timing of a two-word phrase, `--compare` across three categories, `--trials`.
- `crossover` — sweep word length, typed vs chorded, isolated.
- `cascade` — continuous phrase, per-word cost including the transition into each word, custom `--chords`.
- `race` — max-speed repeated word with auto-space-on-release.
- `resolution` — autonomous timing-resolution self-test (measured floor: ~0.055ms jitter).

Architecture: a Cargo workspace.

- `typhony-core` — pure, OS-independent chord logic (dictionary letter-set lookup + abbreviation overlay, and the `ChordBuffer` timing/roll model). Compiles to WASM.
- root binary — the CLI: rdev listener, enigo injector, benchmark scoring.
- `typhony-wasm` — wasm-bindgen bindings over core for the browser demo.

Key empirical findings (candidates for the site's data section):

- Chording a long word (~5 letters) is ~2.4x faster than typing it; a 2-letter word is a wash-to-loss in mixed context but ~1.8x faster in sustained repetition.
- The value is concentrated in longer words; short words are often better typed. Chord long, type short.
- Transition/setup cost (forming a chord after the previous word) is what makes short chords lose in flowing text — visible in `cascade`, hidden in `crossover`.
- The measurement instrument is trustworthy to sub-millisecond (0.055ms jitter floor), so every number in the project sits far above noise.

## Track A: demo website (2 of 4 done)

Goal: a triple-threat portfolio piece — the keylogger-demonstration software, a Rust/WASM interactive site, and documentation backing the data.
Stack: wasm-pack + a lightweight static site.
Host: GitHub Pages on the repo.

1. **Done** — extract `typhony-core` (pure, WASM-ready).
2. **Done** — `typhony-wasm` crate; builds to a 184KB module exporting a `Trainer` class (`lookup`, `size`), dictionaries embedded via `include_str!`. Toolchain moved from brew Rust to rustup for the wasm32 target.
3. **Todo — the site.** Three sections:
   - Interactive in-page chord trainer running on the WASM core. The page's own textarea provides keydown/keyup in a sandbox; JS tracks held keys and calls `Trainer.lookup` on release. This is also the keylogger-ethics hook: the page contrasts the sandboxed in-page demo against what the native tool does (global capture, OS permissions) — the distinction SECURITY.md already draws.
   - Data/methodology writeup: the findings above, with the resolution floor as the rigor credential.
   - Keylogger-ethics piece adapted from SECURITY.md.
4. **Todo — CI/deploy.** GitHub Actions: `rustup target add wasm32-unknown-unknown`, `wasm-pack build`, bundle the site, deploy to Pages on push. `wasm/pkg` stays gitignored (CI regenerates).

Open: whether the site uses any JS framework (currently planned: hand-rolled, no framework) or Trunk instead of wasm-pack (decided against for now — wasm-pack is current/maintained under the new `wasm-bindgen` org).

## Track B: config + macro system (planned, not started)

The idea: move chords/macros out of hardcoded CSVs into a versionable, CI-testable TOML config, with a recorder and a deterministic timing-aware replay/test harness.

### Config file

- A TOML file — name TBD: `.typhony.toml` (project-local, git-tracked) is the leading option; a global `~/.config/typhony/config.toml` may also be supported, with project config overriding.
- Stores everything: chord rules, macros, and settings (timing thresholds, dictionary references, abbreviation tiers).
- Because it is a plain file, the whole configuration is versionable in git.

### Rule syntax (TOML)

- Chords are defined as letter-sets mapping to an output (a word, a macro, or a longer expansion). Key presses are sets — order and duplicates do not matter — so a rule keys on the sorted, de-duplicated letters (the existing `letterset` in core).
- Settings include the word-boundary timing threshold (see below) and any per-rule modifiers.
- Sketch (exact schema TBD):

  ```toml
  [settings]
  roll_gap_ms = 100          # the word-boundary threshold

  [[chord]]
  keys = "be"                # letter-set; "b+e" and "eb" are equivalent
  output = "because"

  [[macro]]
  keys = "sig"
  output = "Best,\nNick"     # multi-key expansion / macro
  ```

### Recorder

- The CLI records macros as you type: capture a keystroke/timing sequence and the chord it resolves to, then write it into the config.
- Turns "I just did this useful thing" into a persisted, named rule without hand-editing TOML.

### Deterministic replay + timing semantics

- A timed key-event sequence (press/release with timestamps) replays through the chord engine to produce a deterministic output — no hardware, no human, so it is CI-safe.
- Timing rule: after the configured threshold of the keyboard being idle (nothing new pressed), the current chord/word commits; a later press starts a new chord. This is exactly the existing `ChordBuffer` roll-gap model, now parameterized by config.
- Worked example (threshold = 100ms):
  - press `b` at t=0
  - press `e` at t=50 — gap 50ms < 100ms, so `e` joins `b`'s chord
  - wait, then press `t` at t=200 — the 150ms idle before `t` exceeds 100ms, so `b+e` has already committed; `t` begins a new chord
  - result: the chord `b+e` (→ its mapped output), then `t` separately — **not** `b+e+t`

### CI testing

- The config carries test cases: a timed input sequence and the expected output.
- `typhony test` (new subcommand) replays each case deterministically and asserts output == expected, failing with actual-vs-expected on mismatch.
- This lets common words/phrases be locked down: change a rule, run CI, and know immediately whether every expected expansion still holds.
- Sketch:

  ```toml
  [[test]]
  name = "be commits before t"
  # input as (key, press_ms) events; releases inferred or given explicitly
  input = [["b", 0], ["e", 50], ["t", 200]]
  expect = ["because", "t"]
  ```

### Rule tracing

- When a test (or a live replay) runs, `--explain` prints every rule that fired to produce the output, in order — the chord letter-set matched, the rule it hit, and what it emitted.
- Because presses are sets, the trace is a clean sequence of set → rule → output steps, which doubles as documentation of why a given input produced a given result.

### Rough build order

1. Define and parse the TOML schema in `typhony-core` (serde), behind a `Config` type; keep the CSV path as a fallback/import source.
2. Deterministic replay harness in core: feed `(key, timestamp)` events through `ChordBuffer` with the configured threshold, collect outputs. Pure and unit-testable.
3. `typhony test` subcommand: load config, run `[[test]]` cases, report pass/fail, `--explain` traces.
4. Recorder: `typhony record` to capture live input into config rules.
5. Migrate the bundled dictionaries/abbreviations to (or make them importable into) the config format.

## Open reminders

- Dotfiles PRs awaiting merge: #99 (chordgen), #100 (cargo-audit), #101 (rustup + wasm-pack).
- Name: settled on `typhony` (chord + accordion; free on crates.io/npm/GitHub).
