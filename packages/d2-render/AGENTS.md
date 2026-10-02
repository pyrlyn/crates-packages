# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

Renders D2 diagrams. The default backend shells out to the `d2` CLI and parses
its stderr; `src/native/` (feature `native`) is a pure-Rust rewrite of a D2
subset and keeps the MPL-2.0 header on every file. README.md documents the
public API and the CLI-vs-library decision; keep it in sync.

## Rules

- Only pass d2 flags that `d2 --help` lists; check new ones against the
  installed d2 before adding them.
- Tests that need a real d2 must soft-skip (`common::real_d2`) when it is missing.

## Commands

```bash
cargo test -p d2-render --all-features
cargo clippy -p d2-render --all-targets --all-features -- -D warnings
cargo fmt
```
