<!--
  Destination: klavyn/.github → profile/README.md
  This renders on the org landing page at https://github.com/klavyn
-->

# klavyn

**Keyboard shortcuts, for every language.**

Chorded typing on the keyboard you already have: press a word's letters
at once — in any order — and get the whole word. No special hardware, no
separate steno theory. The idea works across scripts and layouts, from
QWERTY to AZERTY to Korean Dubeolsik.

→ **[klavyn.app](https://klavyn.app)** · [Docs](https://klavyn.app/docs) · [Learn chorded typing](https://klavyn.app/learn)

## Repositories

| Repo | What it is |
| ---- | ---------- |
| [**klavyn**](https://github.com/klavyn/klavyn) | The core app — a Rust chorded-typing trainer (CLI + WASM core) and the website. |
| **dictionaries** | Per-language dictionaries and the tooling that builds them. Where new languages and abbreviation packs land. |

*(more to come as the ecosystem grows — a config/format spec, install taps, editor integrations.)*

## Get started

```sh
curl -fsSL https://klavyn.app/install.sh | sh
```

macOS and Linux. See [Download](https://klavyn.app/download) for Cargo and
build-from-source, and [SECURITY.md](https://github.com/klavyn/klavyn/blob/main/SECURITY.md)
for exactly what a global keyboard listener does and doesn't do with that access.

## Contributing

New languages, better dictionaries, and abbreviation packs are the most
valuable contributions — see **CONTRIBUTING.md**. Everything is open source
under MIT / Apache-2.0.
