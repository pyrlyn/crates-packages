# Toolchain

Workspace pin: `rust-toolchain.toml` channel `1.99.0` (profile `minimal`, components `rustfmt` and `clippy`). Member crates also document mise; the root pin is the toolchain file.

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| rustc | rustup, via `rust-toolchain.toml` | Build | https://github.com/rust-lang/rust |
| cargo | rustup, via `rust-toolchain.toml` | Build, test, and `cargo metadata` | https://github.com/rust-lang/cargo |
| git | system package | `git-changed-paths` shells out to it | https://git-scm.com |
| just | mise | Test recipes in the member crates | https://github.com/casey/just |
| ketch | see its README | Installs swarfr for member justfiles | https://github.com/pyrlyn/ketch |
| swarfr | ketch | Lossless cleanup of `target/` after tests | https://github.com/listepo/swarfr |

## cargo

Direct dependencies of the workspace members. Path crates are this repo.

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| cargo-changed-packages | local | https://github.com/pyrlyn/crates-packages | Affected workspace packages |
| file-backup | local | https://github.com/pyrlyn/crates-packages | Sibling `.bak-<unix-seconds>` copies |
| gettext-catalog | local | https://github.com/pyrlyn/crates-packages | `.po` catalog loader and plural selection |
| git-changed-paths | local | https://github.com/pyrlyn/crates-packages | Change set against a base ref |
| polib | local | https://github.com/brettdong/polib | Reads `.po` files for `gettext-catalog` |
| layered-config | local | https://github.com/pyrlyn/crates-packages | Ordered TOML layers with per-leaf provenance |
| llm-http | local | https://github.com/pyrlyn/crates-packages | Injected HTTP transport and retry; no sockets |
| llm-openai | local | https://github.com/pyrlyn/crates-packages | OpenAI chat completions on `llm-http` |
| llm-wire | local | https://github.com/pyrlyn/crates-packages | Provider-neutral chat messages and SSE frames |
| secret-store | local | https://github.com/pyrlyn/crates-packages | Environment, then an injected credential store |
| telemetry-setup | local | https://github.com/pyrlyn/crates-packages | Tracing setup that redacts secrets before storage |
| regex | local | https://github.com/rust-lang/regex | Secret redaction patterns in `telemetry-setup` |
| tracing | local | https://github.com/tokio-rs/tracing | Log events for `telemetry-setup` |
| tracing-subscriber | local | https://github.com/tokio-rs/tracing | Formatter `telemetry-setup` wraps; no OTLP exporter |
| keyring | local | https://crates.io/crates/keyring | Optional `os` feature of `secret-store`; tests leave it off |
| path-gates | local | https://github.com/pyrlyn/crates-packages | Changed paths to gates |
| scoped-check | local | https://github.com/pyrlyn/crates-packages | Check commands for a change |
| anyhow | local | https://github.com/dtolnay/anyhow | Top-level errors of `scoped-check` |
| assert_cmd | local | https://github.com/assert-rs/assert_cmd | Tests that run the `scoped-check` binary |
| clap | local | https://github.com/clap-rs/clap | `scoped-check` CLI |
| determinator | local | https://github.com/guppy-rs/guppy/tree/main/determinator | Maps changed paths to affected packages |
| globset | local | https://github.com/BurntSushi/ripgrep/tree/master/crates/globset | Compile and match gate globs |
| guppy | local | https://github.com/guppy-rs/guppy | Cargo package graph |
| rstest | local | https://github.com/la10736/rstest | Parameterized tests in `file-backup` |
| serde | local | https://github.com/serde-rs/serde | Config types |
| serde_json | local | https://github.com/serde-rs/json | `scoped-check` JSON output |
| shlex | local | https://github.com/comex/rust-shlex | Shell-quoting substituted values |
| tempfile | local | https://github.com/Stebalien/tempfile | Throwaway repos and files in tests |
| thiserror | local | https://github.com/dtolnay/thiserror | Crate-local error enums |
| toml | local | https://github.com/toml-rs/toml | TOML documents, including layered config |

## ketch

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| swarfr | global | https://github.com/listepo/swarfr | Lossless cleanup of `target/` after tests |
