# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

Library: given repo-relative changed paths, the workspace packages they affect,
reverse dependencies included, via `guppy` and `determinator`. Anything that
changes the package graph or the build selects everything (`all: true`); it
never answers "nothing" when it cannot tell.

## Commands

```bash
cargo test
cargo clippy --all-targets
cargo fmt
```

`just test` runs the same tests and finishes with a lossless `swarfr` cleanup of the target dir.
