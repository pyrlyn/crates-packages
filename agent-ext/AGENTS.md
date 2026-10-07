# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

The extension points cox and aulo load from files: `SKILL.md` skills (discovery,
the index line for the prompt, the deferred `skill` tool). It
carries no host types: the tool and hook traits belong to the host, which wraps
what this crate returns.

Skill files are untrusted input. A skill that does not parse, has no
`name`/`description`, is not lowercase-hyphen or does not match its directory is
skipped with a notice, never fatal and never half-loaded. A change that loads
more than this needs a regression test in `tests/skills.rs`.

## Commands

```bash
cargo test
cargo clippy --all-targets
cargo fmt
```

`just test` runs the same tests and finishes with a lossless `swarfr` cleanup of the target dir.
