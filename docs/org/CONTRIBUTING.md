<!-- Destination: klavyn/.github → CONTRIBUTING.md (inherited by every repo in the org) -->

# Contributing to klavyn

Thanks for helping build klavyn. This applies across every repo in the org;
individual repos may add their own notes.

## Ways to help

- **Languages and dictionaries** — the highest-leverage contribution. Add a
  language, improve an existing dictionary, or submit an abbreviation pack.
  See the [`dictionaries`](https://github.com/klavyn/dictionaries) repo.
- **The app** — the Rust core, the CLI, the WASM demo, platform support.
  See [`klavyn`](https://github.com/klavyn/klavyn).
- **Docs and the site** — corrections and clarity fixes are always welcome.

## Ground rules

- Open an issue before a large change so we can agree on the approach.
- Keep pull requests focused; one concern per PR.
- Commit subjects in the imperative mood, ≤ 70 characters. Use the body for
  *why*, not *what*.
- Be excellent to each other (see CODE_OF_CONDUCT.md).

## Developing the app

```sh
git clone https://github.com/klavyn/klavyn
cd klavyn
cargo build            # native CLI (needs X11 dev libs on Linux)
cargo test -p klavyn-core
```

On Linux you'll need the X11 development libraries for the input backend
(Debian/Ubuntu: `libxi-dev libxtst-dev libx11-dev`).

## A note on the keyboard listener

klavyn is, mechanically, a global keyboard listener — chording can't work
any other way. Contributions must preserve that posture: no network calls,
no writing keystrokes to disk. See `SECURITY.md` in the app repo.

## Licensing

By contributing you agree your work is dual-licensed under MIT and
Apache-2.0, matching the project.
