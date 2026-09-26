# Security & privacy

korder registers a global keyboard listener.
Mechanically, that's what a keylogger does — chording can't work any other way, since korder has to see every keystroke to recognize one as part of a chord.
This document says exactly what korder does and does not do with that access, and how to verify each claim yourself rather than take it on faith.

## What korder stores and sends

- **No network access, anywhere, ever.** Not a design promise — a
  structural one: no crate in korder's entire dependency tree does
  networking. Verify: `cargo tree | grep -iE "reqwest|hyper|tokio|curl|http|socket|tls"`
  returns nothing.
- **No disk writes of anything typed.** korder's own code contains zero
  calls that write to the filesystem — it only *reads* the dictionary CSV
  you point it at. Verify:
  `grep -rn "File::create\|OpenOptions\|fs::write" src/` returns nothing.
- **No persistent in-memory log.** `ChordBuffer` (`src/matcher.rs`) holds
  at most the *current, unflushed* chord attempt — a handful of
  characters — and is cleared on every commit (`std::mem::take` in
  `commit()`). There is no growing buffer, history, or transcript of past
  keystrokes anywhere in the process, at any point.
- **One optional, local, ephemeral echo.** By default, when a chord
  resolves, korder prints the resulting *word* (not your raw keystrokes)
  to your own terminal's stderr — purely as feedback while you're
  learning your chords, the same way the CharaChorder hardware or its web
  sandbox shows you what fired. This is never written to a file or sent
  anywhere by korder; it exists only as long as your terminal's own
  scrollback keeps it. Pass `--quiet` to suppress it entirely if you want
  zero terminal echo (e.g. on a shared or recorded terminal session).

## What the permission grants mean, and don't mean

On macOS, korder needs **Input Monitoring** (to listen) and **Accessibility** (to inject corrections).
Both are Apple's own TCC-mediated, user-space APIs (`CGEventTap` / synthetic `CGEvent` posting) — the same mechanism any ordinary app uses for this class of feature, not a kernel extension or driver.
Concretely:

- korder never runs with elevated privileges and installs nothing at the
  kernel level.
- Revoking either permission in System Settings blinds/mutes korder
  instantly — there's no persistence mechanism working around that.
- The OS's own permission system, not korder's code, is the actual
  gatekeeper here.

## Supply chain

- The dependency list is intentionally small: `rdev`, `enigo`, `clap`,
  `csv`, `serde`, `anyhow` and their transitive deps — 95 crates total.
- `cargo audit` (checks `Cargo.lock` against the RustSec advisory
  database) currently reports **zero known vulnerabilities** across all
  95. Re-run it yourself before trusting a given commit — this is a
  point-in-time result, not a standing guarantee, and it should be rerun
  whenever `Cargo.lock` changes.
- The entire capture → match → inject pipeline is under 800 lines across
  five files (`src/main.rs`, `matcher.rs`, `dictionary.rs`, `inject.rs`,
  `benchmark.rs`) — small enough to read in full in a few minutes rather
  than take on trust.

## What korder can't promise

Being direct about the limits, rather than overselling:

- korder hasn't formally audited `rdev`/`enigo`'s own internals beyond
  what `cargo audit` checks against known advisories. "No known
  vulnerabilities" is not the same claim as "provably correct."
- korder cannot protect you from a compromised OS, a maliciously modified
  binary from an untrusted source, or another process on your machine
  that has its own Input Monitoring grant. **Build from source and read
  the diff yourself** rather than run a prebuilt binary from anyone,
  including the maintainer of this repo.
- No user-space software can honestly claim to be immune to
  "kernel-level attacks" — that's a property of the OS/kernel itself, not
  of any app running on top of it. What *is* true, and verifiable above:
  korder requires none of the kernel-level access that would expand that
  attack surface in the first place.
