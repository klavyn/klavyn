#!/usr/bin/env python3
"""
Builds data/dictionary.csv: a letterset -> word chord dictionary.

Source: a chordgen-style chords.csv (word,chord,category,frequency,...),
already frequency-ranked. Words are grouped by their sorted, de-duplicated
letter set (a "chord" on a plain QWERTY keyboard can't distinguish key
order or repeated letters, so "stop"/"pots"/"tops"/"spot" all collide).
On collision, the highest-frequency word wins; the rest are dropped.

Usage: scripts/build_dictionary.py <source_chords.csv> <out_dictionary.csv>
"""
import csv
import sys


def letterset(word: str) -> str:
    return "".join(sorted(set(c for c in word.lower() if c.isalpha())))


def main() -> None:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <source_chords.csv> <out_dictionary.csv>", file=sys.stderr)
        sys.exit(1)

    src, out = sys.argv[1], sys.argv[2]
    best: dict[str, tuple[str, float]] = {}

    with open(src, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            word = row["word"].strip()
            if not word.isalpha() or len(word) < 2:
                continue  # skip punctuation-only/contraction/single-letter entries
            try:
                freq = float(row["frequency"])
            except (KeyError, ValueError):
                continue
            key = letterset(word)
            if len(key) < 2:
                continue  # e.g. all-repeated-letter words like "aa"
            current = best.get(key)
            if current is None or freq > current[1]:
                best[key] = (word.lower(), freq)

    rows = sorted(best.items(), key=lambda kv: -kv[1][1])
    with open(out, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["letterset", "word", "frequency"])
        for key, (word, freq) in rows:
            writer.writerow([key, word, freq])

    print(f"wrote {len(rows)} chords to {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
