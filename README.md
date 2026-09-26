# korder

A QWERTY chorded-typing trainer.

Press the letters of a word at once, in any order, and korder replaces what
you typed with the whole word — the same idea as [CharaChorder](https://www.charachorder.com/),
approximated in software on a normal keyboard, with no special hardware
required.

## Why

Existing free ways to practice chorded input all teach something slightly
different from CharaChorder's actual mechanic (spell the whole word at once):

- [Plover](https://www.openstenoproject.org/) teaches real stenography —
  phonetic, different keys, a different theory entirely.
- [chordgen](https://github.com/dlip/chordgen) teaches a single-seed-letter
  plus thumb-modifier scheme.

korder teaches the actual thing: press a word's letters together, get the
word. If you later buy a CharaChorder, the skill should transfer directly.

## How it works

korder listens globally for key presses (via `rdev`) and can recognize a
chord two ways:

- **Held together** — press several letters at once, release them.
- **Arpeggiated / rolled** — press and release letters one at a time (or a
  couple at once), as long as each new press starts within `--roll-gap-ms`
  (default 200ms) of the keyboard last going idle. This is what makes long
  chords work on ordinary hardware: your keyboard's simultaneous-key
  ceiling (commonly 6 on non-NKRO boards — run `korder detect-rollover` to
  measure yours) only limits how many keys you can hold at *one instant*.
  Rolling through a 7+ letter word one or two keys at a time never asks
  for more than that, so it works regardless of the ceiling.

Either way, once the chord's letters are known, korder checks them against
its dictionary and, on a match, backspaces what you literally typed and
types the matched word instead (via `enigo`).

### A fundamental limitation, by design

A plain keyboard can't sense press *order* or *direction* the way
CharaChorder's switches can — only *which keys*, as a set. That means
anagrams are indistinguishable: `stop`, `pots`, `tops`, and `spot` are the
same chord here. The bundled dictionary (`data/dictionary.csv`) resolves
every such collision by keeping only the highest-frequency word and
dropping the rest (see `scripts/build_dictionary.py`) — of an initial
2,000-word list, 1,776 chords survived.

### Checking your keyboard's rollover ceiling

```sh
cargo run -- detect-rollover
```

Hold down as many different letter keys as you can for a few seconds;
korder reports the max it saw. This is purely informational — the
roll/arpeggiate matching above works regardless of the result — but it
tells you which words you'll need to roll rather than hold. This measures
what your OS/keyboard combo actually delivers end-to-end (via the same
`rdev` listener korder itself uses), rather than a theoretical spec-sheet
number from a USB descriptor that might not survive OS-level coalescing
anyway.

### v1 doesn't suppress the original keystrokes

korder lets your OS type the individual letters normally, then corrects
them after the fact (backspace + retype), the same way autocorrect works.
This means genuinely fast sequential typing that happens to land within the
timing window can occasionally get "corrected" into a chord match you didn't
intend — that's the `--window-ms` knob to tune. True keystroke suppression
(via a macOS `CGEventTap`) is a possible v2, not implemented yet.

## Setup

```sh
cargo build --release
```

On macOS, korder needs two permissions, granted the first time you run it
(System Settings → Privacy & Security):

- **Input Monitoring** — to listen for key presses globally.
- **Accessibility** — to inject the backspace/retype correction. Not
  needed in `--dry-run` mode.

## Usage

```sh
# Try it safely first: logs detected chords, doesn't touch your typing.
cargo run -- --dry-run

# The real thing.
cargo run --release
```

```
korder: loaded 1776 chords from "data/dictionary.csv"
korder: listening globally. Ctrl+C to quit.
korder: chord "klo" -> "look"
```

## Cross-platform status

Built on `rdev` and `enigo`, both of which target macOS, Windows, and Linux
(X11). Honest caveat: **only build- and run-verified on macOS so far.**
Known gaps to expect, not yet confirmed either way:

- **Linux/Wayland**: `rdev`'s Linux backend targets X11; Wayland sessions
  are a known weak spot for this whole category of global-input-hook
  crate, not just here.
- **Permission models differ per OS**: macOS needs Input Monitoring +
  Accessibility (TCC prompts, as documented above); Linux typically needs
  either running as a user in the `input` group or appropriate udev rules
  for raw input access; Windows generally works out of the box via global
  hooks, no special grant needed.
- The `Key` enum and chord-matching logic (`key_to_letter`, `matcher.rs`)
  are already OS-agnostic — cross-platform support is really a question of
  `rdev`/`enigo`'s own backend maturity per OS, not anything korder-specific.

## Status

MVP: chord detection (held *and* arpeggiated/rolled) + dictionary lookup +
naive (non-suppressing) text replacement + a rollover self-test. No
spaced-repetition training yet — see [chordgen](https://github.com/dlip/chordgen)
for that if you want it today.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
