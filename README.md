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

korder listens globally for key presses (via `rdev`), buffers letters that
are pressed within a short window of each other (default 150ms — tune with
`--window-ms`), and on release, checks whether the set of letters matches a
known word in its dictionary. If it does, korder backspaces what you
literally typed and types the matched word instead (via `enigo`).

### A fundamental limitation, by design

A plain keyboard can't sense press *order* or *direction* the way
CharaChorder's switches can — only *which keys*, as a set. That means
anagrams are indistinguishable: `stop`, `pots`, `tops`, and `spot` are the
same chord here. The bundled dictionary (`data/dictionary.csv`) resolves
every such collision by keeping only the highest-frequency word and
dropping the rest (see `scripts/build_dictionary.py`) — of an initial
2,000-word list, 1,776 chords survived.

Your keyboard's own key-rollover limit is a second hard ceiling — run a
[ghosting test](https://keyboardtest.me/keyboard-ghosting-test) to find
yours. korder doesn't enforce a maximum chord size itself.

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

## Status

MVP: chord detection + dictionary lookup + naive (non-suppressing) text
replacement. No spaced-repetition training yet — see
[chordgen](https://github.com/dlip/chordgen) for that if you want it today.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
