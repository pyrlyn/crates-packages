# scoped-check

Run only the check commands a change touches. It computes the paths changed against a
base ref, maps them to gates by glob, and narrows each gate's command to the affected cargo
packages.

```bash
cargo install --path scoped-check
scoped-check plan   # what would run, and why
scoped-check run    # run it
```

## Config: `scoped-check.toml` at the repository top level

```toml
base = "origin/main"   # default; --base overrides
unmatched = "all"      # path-gates key: a path no gate claims selects every gate
workspace = "."        # cargo workspace root, relative to this file

[[gate]]
name = "fmt"
paths = ["**/*.rs"]
run = "cargo fmt --all -- --check"

[[gate]]
name = "test"
paths = ["**/*.rs", "tests/**"]
run = "cargo nextest run {packages} {nextest_filter}"

[[gate]]
name = "docs"
paths = ["**/*.md"]
run = "just docs-check"
```

`paths`, `always` and `unmatched` are read by [`path-gates`](../path-gates). `run` is
executed with `sh -c` (`cmd /C` on Windows) from the config file's directory. A gate
without `run`, or a `run` with an unknown `{placeholder}`, is a config error (exit 2).
Only `{identifier}` is a placeholder; `${VAR}`, `awk '{print}'` and `{a,b}` stay literal.

| Placeholder | Scoped value | `--all`, `all: true` or any error |
| --- | --- | --- |
| `{packages}` | `-p a -p b` (sorted, affected packages incl. reverse dependencies) | `--workspace` |
| `{nextest_filter}` | `-E 'package(=a) \| package(=b)'` | empty |
| `{changed}` | the paths the gate claims, shell-quoted | empty |

Values are POSIX shell-quoted, so avoid `{changed}` on Windows. A gate using `{packages}` or
`{nextest_filter}` whose affected set is empty is skipped: nothing it covers changed.

## Commands

- `scoped-check plan [--base REF] [--config PATH] [--all] [--json]` prints the base, merge
  base, changed-path count and every selected gate with its expanded command and why:
  `scoped`, `all: unmatched <path>`, `all: <error>` or `--all`.
- `scoped-check run [--base REF] [--config PATH] [--all] [--keep-going]` runs the selected
  gates in config order, printing `== <gate>: <command>` before each. Exit code 0 when all
  pass, else the first failing gate's code (1 if it has none). It stops at the first failure
  unless `--keep-going`.

`--config` defaults to `scoped-check.toml` at the git top level of the current directory.

## Fail safe

Any error computing the change set or the affected packages (no git, unknown base, no merge
base, `cargo metadata` failure) prints a warning on stderr and behaves as `--all`. No changed
paths prints `nothing changed against <base>` and exits 0. A config error or bad usage exits 2.

## `plan --json`

```json
{
  "base": "origin/main",
  "merge_base": "<sha>",
  "changed": 3,
  "nothing_changed": false,
  "gates": [{ "name": "test", "command": "cargo nextest run -p a -p b ...", "why": "scoped" }],
  "skipped": [{ "name": "docs", "reason": "nothing it covers changed" }]
}
```

`merge_base` and `changed` are `null` when the change set could not be computed.
