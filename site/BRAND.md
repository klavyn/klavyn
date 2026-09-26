# klavyn brand

A small, opinionated system. Point of view borrowed in spirit from
[coder.com/brand](https://coder.com/brand) — near-black canvas, monospace-forward,
high contrast, confident and technical — but with klavyn's own **cool** palette:
electric **cyan** and **violet** instead of Coder's blue-and-cream.

## Wordmark

`klavyn`, set in **JetBrains Mono 800**, lowercase, letter-spacing `-0.02em`.
The two consonants from the Latin root *clavis* / German *klavier* ("key",
"keyboard") are colorized:

- **k → cyan**
- **v → violet**

The rest of the word stays in the primary text color, so the eye lands on the
two "key" letters. In light mode the wordmark keeps its vivid cyan (logos are
exempt from text-contrast rules).

> **Easter egg / origin:** the mark is really *clavis* wearing a k. Spelled with
> a **c** — `clavyn` — the colored initials would literally match the colors
> (**C**yan, **V**iolet). The c-spelling's GitHub org was taken, so the name
> ships with a k; the color pairing keeps the wink.

## Palette

Cool, dark-first. Every color below is a CSS custom property in `klavyn.css`.

| Token             | Dark        | Light       | Role                                   |
| ----------------- | ----------- | ----------- | -------------------------------------- |
| `--bg`            | `#080b14`   | `#f3f7fd`   | page background                        |
| `--surface`       | `#111a2b`   | `#ffffff`   | cards, code, nav                       |
| `--text`          | `#e7eef9`   | `#0d1526`   | primary text                           |
| `--muted`         | `#a2b0cb`   | `#454f66`   | secondary text                         |
| `--faint`         | `#8090ab`   | `#5f6e88`   | incidental labels                      |
| `--cyan` (`--k`)  | `#34e0e6`   | `#0aa9c2`   | brand accent · wordmark **k**          |
| `--violet` (`--v`)| `#a98bfb`   | `#7431e0`   | brand accent · wordmark **v**          |
| `--teal`          | `#2cd4c0`   | `#0c7a6d`   | kickers, prompts                       |
| `--blue`          | `#4c7df7`   | `#3355fd`   | gradient mid · light button fill       |
| `--purple`        | `#c77dff`   | `#9333ea`   | gradient end                           |
| `--link`          | `#34e0e6`   | `#0b7385`   | text links & inline code (AA-safe)     |

**Signature gradient** (`--grad`): `cyan → blue → violet → purple`, 120°.
Used sparingly — primary button, a hairline on cards, feature-icon chips.

`--cyan` is the *decorative* brand cyan (wordmark, icons, lit keys). `--link` is
its readable sibling for anything that is actual text, so links and inline code
stay legible on light backgrounds where bright cyan would not.

## Contrast (WCAG 2.1)

Target: **AA minimum, AAA where it matters.** Verified with the standard relative
-luminance formula against the backgrounds each token is used on.

- **AAA** (≥7:1): primary text, secondary (`--muted`) text, links, teal labels
  — both themes.
- **AA** (≥4.5:1): incidental `--faint` labels, inline code, button text.
- The primary button uses dark ink on the bright cyan→violet gradient in dark
  mode (AAA both ends); on light backgrounds it switches to a blue→violet fill
  with white ink, because no single ink color clears AA across the full
  cyan→violet span on light.

Re-run the audit if you touch a color: the ratios above are load-bearing, not
decorative.

## Type

- **JetBrains Mono** — wordmark, nav, code, kickers, UI labels, buttons.
- **Inter** — body prose and paragraphs.

Mono carries the identity; Inter keeps long-form reading comfortable.
