#!/usr/bin/env python3
"""
Builds a korder chord dictionary CSV (letterset,word,frequency) from a
hermitdave/FrequencyWords list (https://github.com/hermitdave/FrequencyWords)
— plain "word<space>count" lines, already frequency-sorted, one file per
language, uniform format across ~60 languages.

Words are grouped by their *chord-detectable* letter set: lowercased,
diacritics folded to their base Latin letter (a plain physical keyboard
key can't sense an accent — there's no separate physical key for "é" on
any layout; it's a compose/dead-key/AltGr result), German ß folded to
"ss". On a letterset collision (very common: café/cafe fold to the same
chord, as do many real anagrams), the highest-frequency word wins and the
rest are dropped. The winning word's *original* spelling (accents intact)
is what gets typed on a chord match — only the detection key is folded.

Usage: scripts/build_dictionary.py <lang_50k.txt> <out_dictionary.csv> [--limit N]
"""
import csv
import sys
import unicodedata

FOLD_SPECIAL = {
    "ß": "ss",  # German sharp s: not a diacritic, a historical ligature for "ss"
}


def fold_letters(word: str) -> str:
    """Lowercases and strips accents down to plain a-z, for chord detection."""
    folded = "".join(FOLD_SPECIAL.get(c, c) for c in word.lower())
    decomposed = unicodedata.normalize("NFD", folded)
    return "".join(c for c in decomposed if unicodedata.category(c) != "Mn")


def letterset(word: str) -> str:
    ascii_letters = "".join(c for c in fold_letters(word) if c.isascii() and c.isalpha())
    return "".join(sorted(set(ascii_letters)))


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--limit")]
    limit = 5000
    for a in sys.argv[1:]:
        if a.startswith("--limit="):
            limit = int(a.split("=", 1)[1])

    if len(args) != 2:
        print(f"usage: {sys.argv[0]} <lang_50k.txt> <out_dictionary.csv> [--limit=N]", file=sys.stderr)
        sys.exit(1)

    src, out = args
    best: dict[str, tuple[str, int]] = {}
    seen = 0

    with open(src, encoding="utf-8") as f:
        for line in f:
            if seen >= limit:
                break
            parts = line.rstrip("\n").split(" ")
            if len(parts) != 2:
                continue
            word, count_str = parts
            if not word.isalpha() or len(word) < 2:
                continue  # skip contractions/punctuation-bearing tokens etc.
            try:
                count = int(count_str)
            except ValueError:
                continue
            seen += 1
            key = letterset(word)
            if len(key) < 2:
                continue  # e.g. an all-repeated-letter word
            current = best.get(key)
            if current is None or count > current[1]:
                best[key] = (word.lower(), count)

    rows = sorted(best.items(), key=lambda kv: -kv[1][1])
    with open(out, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["letterset", "word", "frequency"])
        for key, (word, count) in rows:
            writer.writerow([key, word, count])

    print(f"read {seen} words, wrote {len(rows)} chords to {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
