#!/usr/bin/env python3
"""
For every 2-distinct-letter subset of the alphabet, lists the highest-
frequency English words containing both letters (any position, any
order) - candidate material for hand-picking abbreviation codes
(data/abbrev.en.csv), the same way "kq" was picked for "quick".

Pairs that land on the same touch-typing finger (standard QWERTY chart)
are excluded: those two keys are awkward to press as part of the same
chord attempt, so they're poor abbreviation-code material regardless of
how many good candidate words share them.

Usage: scripts/letter_pairs.py <lang_50k.txt> <out.csv> [--top N]
"""
import csv
import itertools
import sys
from collections import defaultdict

FINGER = {
    "q": "L-pinky", "a": "L-pinky", "z": "L-pinky",
    "w": "L-ring", "s": "L-ring", "x": "L-ring",
    "e": "L-middle", "d": "L-middle", "c": "L-middle",
    "r": "L-index", "f": "L-index", "v": "L-index",
    "t": "L-index", "g": "L-index", "b": "L-index",
    "y": "R-index", "h": "R-index", "n": "R-index",
    "u": "R-index", "j": "R-index", "m": "R-index",
    "i": "R-middle", "k": "R-middle",
    "o": "R-ring", "l": "R-ring",
    "p": "R-pinky",
}


def same_finger(a: str, b: str) -> bool:
    return FINGER[a] == FINGER[b]


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--top")]
    top_n = 8
    for a in sys.argv[1:]:
        if a.startswith("--top="):
            top_n = int(a.split("=", 1)[1])

    if len(args) != 2:
        print(f"usage: {sys.argv[0]} <lang_50k.txt> <out.csv> [--top=N]", file=sys.stderr)
        sys.exit(1)

    src, out = args
    words: list[tuple[str, int]] = []
    with open(src, encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split(" ")
            if len(parts) != 2:
                continue
            word, count_str = parts
            if not word.isalpha():
                continue
            try:
                count = int(count_str)
            except ValueError:
                continue
            words.append((word.lower(), count))

    letters = "abcdefghijklmnopqrstuvwxyz"
    all_pairs = list(itertools.combinations(letters, 2))
    easy_pairs = [(a, b) for a, b in all_pairs if not same_finger(a, b)]
    excluded = len(all_pairs) - len(easy_pairs)

    matches: dict[tuple[str, str], list[tuple[str, int]]] = defaultdict(list)
    for word, count in words:
        letter_set = set(word)
        if len(letter_set) < 2:
            continue
        for a, b in easy_pairs:
            if a in letter_set and b in letter_set:
                matches[(a, b)].append((word, count))

    rows = []
    for pair, candidates in matches.items():
        candidates.sort(key=lambda wc: -wc[1])
        top = candidates[:top_n]
        total_freq = sum(c for _, c in top)
        rows.append((pair, top, total_freq))
    rows.sort(key=lambda r: -r[2])  # highest-value pairs first

    with open(out, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["pair", "top_words", "top_words_total_frequency"])
        for pair, top, total_freq in rows:
            writer.writerow(["".join(pair), " ".join(w for w, _ in top), total_freq])

    print(
        f"{len(all_pairs)} total pairs, {excluded} excluded (same finger), "
        f"{len(rows)} pairs had >=1 candidate word. Wrote {out}.",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
