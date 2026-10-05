# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

A binary that runs only the check commands a change touches. It glues three sibling
crates: `git-changed-paths` (change set), `path-gates` (glob rules to gates) and
`cargo-changed-packages` (affected workspace packages). Do not re-implement any of them.

## Rules

- Fail safe: any error computing the change set or the affected packages warns on stderr
  and behaves as `--all`, never as "nothing to do". Only a config error is fatal (exit 2).
- Substituted values are shell-quoted with `shlex`; the `run` text itself is the
  repository's own, trusted like its justfile.
- Tests build throwaway git repos with `tempfile`; keep host git config out of them.

## Commands

```bash
cargo test -p scoped-check
cargo clippy -p scoped-check --all-targets
cargo fmt
```
