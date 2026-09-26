<!-- Destination: klavyn/dictionaries → README.md -->

# klavyn dictionaries

Per-language chord dictionaries for [klavyn](https://github.com/klavyn/klavyn),
plus the tooling that builds them. This is where new languages and
abbreviation packs live.

## What a dictionary is

A chord is an unordered **set** of the letters in a word. A dictionary maps
each letter-set to the single word klavyn should emit for it.

```
# dictionary.en.csv  —  letters,word
klo,look
ehllo,hello
aeht,the
```

Because chords are sets, some words share one — `{a,e,r}` is `are`, `ear`,
and `era`. The dictionary resolves every collision by keeping only the
**highest-frequency** word for that letter-set, so a chord is never
ambiguous.

## Layout

| File | Purpose |
| ---- | ------- |
| `dictionary.<lang>.csv`  | The core letter-set → word map for a language. |
| `abbrev.<lang>.csv`      | An abbreviation overlay layered on top of the dictionary (e.g. `tbh` → `to be honest`). Community-extensible. |
| `build/`                 | Scripts that generate a dictionary from a frequency word list. |

`<lang>` is a BCP-47-ish tag: `en`, `fr`, `ko`, `ru`, `ar`, …

## Adding or improving a language

1. Start from a public-domain or permissively licensed frequency word list
   for the language (cite the source in your PR).
2. Run the build script to produce `dictionary.<lang>.csv`, applying the
   highest-frequency collision rule.
3. Open a PR with the generated CSV **and** the source/command used, so the
   result is reproducible.

Non-Latin scripts are welcome and expected — Korean (Dubeolsik jamo),
Cyrillic, Arabic, and more. The chord model is script-agnostic: it works on
whatever units a layout produces.

## Abbreviation packs

`abbrev.<lang>.csv` is meant to grow by community submission. Keep entries
unambiguous and broadly useful; niche or personal shortcuts belong in a
user's own config, not the shipped pack.

## Licensing

Dictionaries are dual-licensed MIT / Apache-2.0. Only contribute word lists
whose license permits redistribution, and note the provenance in your PR.
