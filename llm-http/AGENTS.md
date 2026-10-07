# AGENTS.md

If an `AGENTS.md` or `CLAUDE.md` exists higher in the tree, follow it too. If it
conflicts with this file, ask the creator.

## What this crate is

The HTTP plumbing every LLM provider wire needs on top of `reqwest`: client
construction, credential lookup (env var, then platform keyring), auth
headers, non-2xx to `ProviderError` mapping, retry with backoff around one
provider stream, and generic SSE framing. Extracted from cox's
`cox-provider-http` so cox and aulo share one copy. Errors and events are
`llm-wire` types.

## Rules

- **Behaviour-neutral with cox.** Which statuses map to which `ProviderError`,
  which errors retry, the backoff curve and the env-first key lookup match
  cox's. Change one only together with cox, never alone.
- **Secrets stay out of logs and errors.** Auth headers are marked sensitive,
  a bad credential is `ProviderError::Auth` with no text, and nothing here
  formats a key. Keep the tests that prove it.
- **Tests never touch the real OS keyring.** Inject the lookup
  (`resolve_key_with`) or switch the keyring off; only a binary calls
  `resolve_key`.
- **No `unwrap`, `expect` or `panic!` outside tests** (clippy denies them), and
  no `unsafe` (forbidden), so tests inject the environment instead of setting it.
- **Doc comments on every public item** (`missing_docs` is on).
- **Not published yet** (`publish = false`): cox depends on a published crate
  only, because a path dependency breaks cox CI. It depends on `llm-wire` by
  path for the same reason, so both are published together.

## Commands

```bash
cargo test -p llm-http
cargo clippy -p llm-http --all-targets -- -D warnings
cargo fmt
```

`just test` runs the tests and finishes with a lossless `swarfr` cleanup of the target dir.
