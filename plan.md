# crates (crates-packages)

<https://github.com/pyrlyn/crates-packages>

Cargo workspace of five small published crates shared by ketch and rtok — git-changed-paths, path-gates, cargo-changed-packages, scoped-check, file-backup — released via release-plz.

| # | Status | Priority | Complexity | Readiness | Agent |
| --- | --- | --- | --- | --- | --- |
| T1 | todo | P1 | 3 | 0% | |
| T2 | todo | P1 | 2 | 0% | |
| T3 | todo | P2 | 2 | 0% | |
| T4 | todo | P2 | 2 | 0% | |
| T5 | todo | P3 | 1 | 0% | |
| T6 | todo | P2 | 2 | 0% | |
| T7 | todo | P3 | 2 | 0% | |
| T8 | todo | P3 | 1 | 0% | |
| T9 | todo | P3 | 2 | 0% | |
| T10 | todo | P2 | 1 | 0% | |
| T11 | in progress | P1 | 2 | 90% | Claude / opus-5.5 |
| T13 | in progress | P1 | 2 | 90% | Claude / opus-5.5 |
| T20.2 | todo | P1 | 3 | 0% | |
| T20.3 | todo | P2 | 4 | 0% | |
| T22.1 | in progress | P1 | 2 | 0% | Cursor / claude-opus-5.5 |
| T22.2 | in progress | P1 | 2 | 0% | Cursor / claude-opus-5.5 |

Audit note (2026-10-07): verified defenses — git argument injection refused, POSIX shell quoting correct, path traversal blocked, fail-safe direction is always "everything changed". The tasks below are what remains.

### T1. Windows: POSIX quoting handed to cmd.exe in scoped-check

`scoped-check/src/plan.rs:139,155-157` builds `{changed}`/`{packages}` with `shlex::try_join`/`try_quote` (POSIX single quotes), but `scoped-check/src/main.rs:146-149` runs commands via `cmd /C` on Windows, which does not honor them. A file named `docs/x & calc & .md` executes `calc`; plain spaces break arguments. Tests are `#![cfg(unix)]` (`scoped-check/tests/cli.rs:4`), so the path is untested. Done means: Windows-appropriate quoting (or raw argument passing) with tests covering it.

### T2. `{changed}` silently expands to empty in full mode

`scoped-check/src/plan.rs:87-91` sets `changed: String::new()` for `--all`, unmatched-path fallback, and error fallbacks, while `paths` is usually already known (`plan.rs:58-63`). A gate `run = "gofmt -w {changed}"` becomes `gofmt -w` (reads stdin, hangs CI); test `cli.rs:176-180` pins the trailing-empty behavior. Done means: the full quoted path list is substituted whenever paths are known, and a forced plan that still contains `{changed}` with unknown paths warns or refuses.

### T3. `--config` outside the repository top mixes path bases

`git-changed-paths` returns repo-top-relative paths, but `Config::parse` sets `dir` to the config file's parent (`scoped-check/src/config.rs:96-99`) and `plan::build` uses that dir both for git and as the cargo root (`plan.rs:58,174`), so `--config sub/scoped-check.toml` feeds determinator wrong relative paths and gates get wrong package sets. Done means: `dir` is resolved via `git rev-parse --show-toplevel` from the config parent, with a test.

### T4. file-backup: name-probe race and symlink-following overwrite

`file-backup/src/lib.rs:39-45` does `while bak.exists()` then `fs::copy` — two concurrent backups can pick the same name and the second silently truncates the first (exactly what the comment claims to prevent), and a planted `<name>.bak-<ts>` dangling symlink is followed by `fs::copy`. Done means: the destination is created with `OpenOptions::create_new(true)` + `io::copy`, bumping the suffix on `AlreadyExists`.

### T5. Error-quality fixes: canonicalize and NotARepository

`cargo-changed-packages/src/lib.rs:103` compares two `canonicalize().ok()` results — double failure passes as equality; bail when either errors. `git-changed-paths/src/lib.rs:78-86` reports "not a git repository" for a nonexistent directory; distinguish the two cases.

### T6. Deduplicate git-toplevel discovery

Repo-top discovery exists privately in `git-changed-paths/src/lib.rs:78-86` and again in `scoped-check/src/main.rs:94-103` (losing `GIT_OPTIONAL_LOCKS=0` and the error mapping); the same pattern is copy-pasted in cox (`crates/cox-tools/src/git.rs:518`) and swarfr (`src/session.rs:520`). Done means: `git_changed_paths::toplevel(dir)` is public and all four call sites use it (coordinate the cox/swarfr swap with those projects).

### T7. Extract the shared test fixtures

The "a depends on b, c unrelated" workspace fixture is built twice (`cargo-changed-packages/tests/affected.rs:13-34`, `scoped-check/tests/cli.rs:55-92`) and the `git()` helper twice (`git-changed-paths/tests/changed_paths.rs:11-26`, `cli.rs:38-47`). Done means: one dev-only shared test-support crate in this workspace.

### T8. Parse the scoped-check config TOML once

`Config::parse` parses the same TOML text twice with two schemas (`scoped-check/src/config.rs:73-74`). Done means: one struct embedding `GateSpec` parses once, keeping the acknowledged trade-offs in mind.

### T9. Robustness batch: dead gates, dropped paths, `./` stripping

A `[[gate]]` with `run` but no `paths`/`always` is silently unselectable (`path-gates/src/lib.rs:121-131`) — a typo'd glob skips checks; warn or add `plan --lint`. `Selection::unmatched` is never surfaced when `unmatched = "ignore"` (`plan.rs:71-84`). `path-gates/src/lib.rs:199-202` strips only one leading `./` (`././x` stays `./x`, breaking exact patterns like `Cargo.lock`). Done means: all three fixed with tests.

### T10. Fix crate metadata: license mix and repository URL drift

Workspace `Cargo.toml` sets `license = "GPL-3.0-or-later"` (inherited only by file-backup) while the other four crates hardcode `MIT`; every `repository` field, README badges, and the release-plz postprocessor say `pyrlyn/crates-packages` while origin is `listepo/crates-packages`. Done means: one license everywhere (intentionally chosen) and URLs match the canonical remote.

### T11. atomic-replace: one atomic file write for every project

Six projects (rtok, ketch, cox, swarfr, aulo, runa) carry about 18 hand-written temp-file-and-rename writes that disagree on fsync, permissions, symlinks and temp naming. New crate `atomic-replace` (std only, edition 2021, Rust 1.86 so ketch can use it): `write(path, bytes)` and `Options { durable, preserve_permissions, follow_symlinks, create_parent, mode }` with `write`/`write_with`, plus `sync_dir`. Ported from rtok `rtok-agent-sdk` `write_atomic`/`link_target` (permissions, Windows read-only bit, dangling links) and ketch `state.rs` `save_path` (fsync file and directory). Done means: the crate is in the workspace with tests, wired into CI dry-run publish, bump options, README and sonar; consumers migrate in their own tasks.

Plan:
1. `atomic-replace/` with Cargo.toml, src/lib.rs, AGENTS.md, README.md, plan/todo/done/roadmap/ideas/toolchain.md, justfile (copy of change-preview's layout).
2. Temp file beside the target via `create_new`, named `.<name>.tmp-<pid>-<nanos>-<counter>`; removed on every error path.
3. Register: workspace members, `ci.yml` dry-run, `bump.yml` package options, root README crate table and token scope, `sonar-project.properties`.
4. Check: `cargo test -p atomic-replace`, clippy `-D warnings`, fmt, `cargo +1.86 check -p atomic-replace`.


### T20.2. wasm-plugin-host: package layout

Plugin packages on disk, generic over the application's manifest type and home directory: discovery of user plugins under `<home>/plugins/<id>/versions/<digest12>/` (following `current`) and project plugins under `<root>/.<app>/plugins/<id>/`, the package digest, staging a package and the `current`/`previous` swap, `link` for development, and `remove` confined to the plugins root. Ported from cox `crates/cox-plugin/src/discover.rs` and `install.rs`; coordinate with roadmap T16 `dir-ops`, which also draws on `install.rs`. Done when both are in the crate with their tests.

### T20.3. wasm-plugin-host: host-function kit

What every application's host functions repeat: the `(u64) -> u64` JSON wire with an `{"Ok": …}` / `{"Err": …}` reply, the per-export refusal rule, the plugin key-value store with its quotas, and the outbound HTTP allow-list with its body cap. Ported from cox `crates/cox-plugin/src/hostfn.rs` and `net.rs`, leaving the cox-only functions (context, tools, model calls) in cox. Done when cox's kernel functions could be rebuilt on the kit with their tests passing here.

### T13. app-home: one home, app-home and XDG resolver

About 30 hand-written resolvers across rtok, cox, aulo, ketch, runa and swarfr disagree on empty variables, the Windows `USERPROFILE` fallback, relative XDG values and the no-home fallback (runa alone has 9; cox and ketch each carry two that disagree). New crate `app-home` (std only, edition 2021, Rust 1.86). Done means: the crate is in the workspace with tests and registered like `atomic-replace`; consumers migrate in their own tasks.

Plan:
1. `Dirs` over an injectable variable lookup (`Dirs::from_env()`, `Dirs::with(lookup)`), plus free functions over the process env.
2. `user_home`: non-empty `HOME`, else non-empty `USERPROFILE`, else `std::env::home_dir` (process env only). From rtok-hook `user_home_from` and cox `home_dir`.
3. `app_home(var, dot_name)`: non-empty `$var` with `~` expanded, else `<user home>/<dot_name>`. From rtok-hook `home_dir_from`, cox `cox_home`, aulo `aulo_home`.
4. `config_home`/`data_home`/`cache_home`/`state_home`: the XDG variable when absolute (the spec says relative values are invalid), else `~/.config`, `~/.local/share`, `~/.cache`, `~/.local/state` on every OS. From ketch `platform`/`shell.rs` and runa.
5. `expand_tilde(path, home)`: `~`, `~/` and `~\` joined by components. From ketch `config.rs` and rtok-hook `join_tilde_rest`.
6. Check: tests with injected env, clippy for macOS/Linux/Windows, Rust 1.86, publish dry-run.

### T22.1. gettext-catalog: catalog parsing, plural rules and placeholders

Approved by the creator as Mailune's X4 (2026-10-10: extract the shared code and publish it through release-plz). cox's `cox-i18n` parses `.po` catalogs with `polib`, evaluates `Plural-Forms` with its own parser, and fills `{name}` placeholders; Mailune's F10 needs the same for core-originated strings. New crate `gettext-catalog`, split into two tasks to stay under 500 lines each. This one: `catalog` (entries by key, fuzzy and incomplete plurals untranslated, the `# cldr-other:` override), `plural` (the GNU gettext C subset, tested against CLDR for ru and uk) and `format` (`pieces`, `placeholders`, `render`, `validate`), with one `Error` enum for the crate. Done means: the crate is in the workspace with those modules and their tests, registered for CI dry-run publish, bump, README, sonar and the root `toolchain.md`.

Plan:
1. `gettext-catalog/` with the member files of `wasm-plugin-host`. Edition 2024, `rust-version` 1.98 (cox). Dependency versions are the ones cox locks (`polib 0.3.0`, `unic-langid 0.9.6`, `sys-locale 0.3.2`).
2. Port `catalog.rs`, `plural.rs` and `format.rs` from `cox-i18n` with their tests; the plural and placeholder errors become variants of the crate's `Error`, and the evaluator loses its `unreachable!`.
3. Register as T21 did; the package ships only `src`, `tests` and `README.md`.
4. Check: fmt, clippy `-D warnings`, `cargo test --workspace --locked`, `cargo publish --dry-run -p gettext-catalog --locked`.

### T22.2. gettext-catalog: localizer with negotiation and fallback

The second half of Mailune's X4. `Localizer` over the locales an application passes in (code, `.po` text, CLDR `other` form) and a default locale: negotiation by language subtag from POSIX and OS tags (`uk_UA.UTF-8`, `ru-RU`), the user's languages from `LC_ALL`, `LC_MESSAGES`, `LANG` and `sys-locale`, per-message fallback (translation, default `msgstr`, source text, then the id itself), `count` selecting the plural form, and constants such as a product name (cox's `{brand}`). cox keeps its embedded catalogs, `global()`, `tr!` and the native-catalog export. Done means: the localizer is in the crate with `.po` fixtures for en, ru and uk and tests for plural tables, fractional and text counts, fallback, negotiation and placeholders.

Plan:
1. `src/lib.rs`: `Locale`, `Value`, `Args`, `Localizer` (`new`, `for_tags`, `from_env`, `with_constant`, `chain`, `try_format`, `format`), `negotiate`, `parse_tag`, `requested_languages`, ported from `cox-i18n` with the cox constants (`LOCALES`, `BRAND_NAME`, `DEFAULT_LOCALE`) turned into parameters.
2. `tests/fixtures/{en,ru,uk}.po` and `tests/localizer.rs`.
3. Check as T22.1, plus the doc example.

Left (both): review and merge; GitHub Actions must be enabled on this repository and `CARGO_REGISTRY_TOKEN` must cover `gettext-catalog` before its first `bump.yml` run. cox and Mailune migrate once the crate is on crates.io.
