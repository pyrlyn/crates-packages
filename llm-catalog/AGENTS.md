# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

The model catalog an agent loop reads instead of hard-coding model facts: for
a model id, its context window, max output, efforts, capability flags and
price, merged from built-in rows, plugin rows, the host's configured entries
and a user price file, plus the per-wire effort map (`effort_for`). Extracted
from cox's `cox-models` so cox and aulo share one catalog. Pure: no I/O beyond
parsing an embedded or caller-supplied string.

## Rules

- **Behaviour-neutral with cox.** The merge order, the fill-only plugin rule
  and the effort table are cox's. Change one only together with cox.
- **The host's config types never enter this crate.** The host maps its own
  config and plugin manifest onto `ModelEntry` and `PluginModels`.
- **Config files have a schema.** `models.toml` and `prices.toml` are
  described by the types behind `models_schema` and `prices_schema`; the
  committed `schema/*.json` must match them (a test fails when stale;
  `UPDATE_SCHEMAS=1 cargo test -p llm-catalog` rewrites them). Only
  `config.rs` touches `figment`.
- **Data files are copied, not edited.** `models.toml` and `prices.toml` come
  from cox's vendor pipeline (models.dev). Refresh them from cox, never by hand.
- **No `unwrap`, `expect` or `panic!` outside tests** (clippy denies them).
- **Doc comments on every public item** (`missing_docs` is on).
- **Not published yet** (`publish = false`): cox depends on a published crate
  only, because a path dependency breaks cox CI. The path dependency on
  `llm-wire` becomes a version requirement when both are published.

## Commands

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

`just test` runs the tests and finishes with a lossless `swarfr` cleanup of the target dir.
