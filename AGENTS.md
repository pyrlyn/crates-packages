# AGENTS.md

If an AGENTS.md or CLAUDE.md exists higher in the tree, follow it too; on conflict, ask the creator.

## What this workspace is

Cargo workspace of five small Rust crates shared by ketch and rtok:

| Crate | Role |
| --- | --- |
| `cargo-changed-packages` | Workspace packages a set of changed paths affects, including reverse dependencies |
| `file-backup` | Copy a file to `<name>.bak-<unix-seconds>` beside it before replacing it |
| `gettext-catalog` | gettext `.po` catalogs for an application's own strings: plural rules, named placeholders, language negotiation and per-message fallback |
| `git-changed-paths` | Paths a git working tree changed relative to a base ref |
| `path-gates` | Map changed paths to named gates by glob rules from a TOML config |
| `scoped-check` | Binary that runs only the check commands a change touches |
| `telemetry-setup` | `tracing` setup with rotating JSON logs, optional OTLP traces and secret redaction, shared by aulo and Mailune |
| `sqlite-change-feed` | Change feed for a SQLite database shared between processes: `PRAGMA data_version` through Diesel |
| `wasm-plugin-host` | WebAssembly plugin host (extism) shared by cox and scull: worker per plugin, memory cap, deadlines |

Each member directory has its own `AGENTS.md`. This file is the workspace root.

`Cargo.toml` `repository` fields and the root README name
`https://github.com/pyrlyn/crates-packages`. This checkout's `origin` is
`git@github.com:listepo/crates-packages.git`.

## Commands

From the workspace root:

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Rust is pinned by `rust-toolchain.toml` (channel 1.99.0, with rustfmt and clippy).
Releases are described in the README: `release-plz` updates versions, and only
`file-backup` is published to crates.io.

## Tests

`file-backup` keeps its tests in `src/lib.rs` under `#[cfg(test)]` (missing
file, identical-byte skip, and the suffix used when the timestamp collides).
The other four crates use a `tests/` directory. Do not add a second harness
for `file-backup` while those inline tests are still the suite.
