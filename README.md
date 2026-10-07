# crates-packages

[![ci](https://github.com/pyrlyn/crates-packages/actions/workflows/ci.yml/badge.svg)](https://github.com/pyrlyn/crates-packages/actions/workflows/ci.yml) [![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=listepo_crates-packages&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=listepo_crates-packages) [![Coverage](https://sonarcloud.io/api/project_badges/measure?project=listepo_crates-packages&metric=coverage)](https://sonarcloud.io/component_measures?id=listepo_crates-packages&metric=coverage) [![Tests](https://img.shields.io/sonar/tests/listepo_crates-packages?server=https%3A%2F%2Fsonarcloud.io&compact_message)](https://sonarcloud.io/component_measures?id=listepo_crates-packages&metric=tests)

Small, focused Rust crates shared by `ketch` and `rtok`.

| Crate | What it does |
| --- | --- |
| [`cargo-changed-packages`](cargo-changed-packages) | Workspace packages a set of changed paths affects, including reverse dependencies |
| [`change-preview`](change-preview) | Preview what a command would change: diff or `--stat` of edits, size and file count of removals, totals |
| [`file-backup`](file-backup) | Copy a file to `<name>.bak-<unix-seconds>` beside it before replacing it |
| [`git-changed-paths`](git-changed-paths) | Paths a git working tree changed relative to a base ref |
| [`path-gates`](path-gates) | Map changed paths to named gates by glob rules from a TOML config |
| [`scoped-check`](scoped-check) | Run only the check commands a change touches (binary) |

## Commands

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

## Release

Any workspace crate without `publish = false` can be released, one crate per
run, each with its own version, `<crate>/CHANGELOG.md` and
`<crate>-v<version>` tag.

- Pushing to `main` runs CI (commits lint on PRs, fmt, clippy, unit tests, and
  `cargo publish --workspace --dry-run`, which verifies every publishable crate).
- A release is `gh workflow run bump.yml -f package=<crate>` (Actions → Bump and
  release), the only way to a release and the only thing that creates a
  `<crate>-v<version>` tag. First it checks that the crate is a workspace
  member, is publishable and that the workspace crates it depends on are already
  on crates.io. It runs `release-plz update` (version and changelog from the
  conventional commits), opens a pull request with that one commit, rebase-merges
  it once the required checks are green, tags the commit that landed on main,
  creates a draft release and dispatches `release.yml` on the tag: the tag must
  name a publishable crate and the version in its `Cargo.toml`, then the gate
  runs again, `cargo publish -p <crate> --locked` goes to crates.io, and the
  draft is published. Anything failing before the merge closes the pull
  request: no tag, no release. `-f dry-run=true` opens the pull request, waits
  for its checks and closes it.
- A crate that has never been tagged is released at the version in its
  `Cargo.toml` (`0.1.0`), with a new changelog. A crate that must not be
  published says `publish = false`.
- `bump.yml` must call `release-plz update -p <crate>`. pyrlyn/ci's `bump.yml`
  (pinned in `.github/workflows/bump.yml`) runs a bare `release-plz update`,
  which rewrites the version and changelog of every crate with unreleased
  commits into the one release commit. Do not run a release here until the pin
  points at a pyrlyn/ci revision that limits the update to `package`.

### Release order

A crate is released only after every workspace crate it depends on (normal,
build, and dev-dependencies that carry a version) is on crates.io at the version
in this repository: crates.io resolves the dependencies when a crate is
published. Both workflows check it and stop before a pull request or a publish.
The rest of the order is free, so release the crates others wait for first.

crates.io limits how fast new crates appear: a burst of 5 new crates, then one
every 10 minutes; new versions of existing crates have a burst of 30 and one per
minute (`default_burst` and `default_rate_seconds` in
[`src/rate_limiter.rs`](https://github.com/rust-lang/crates.io/blob/29692b85b6e7506f5cf504ad5e2eb05ba36a8e05/src/rate_limiter.rs),
read 2026-10-07; the registry operator can change the defaults or raise them for
one account, which this file cannot show). A first release past the burst fails
at `crates-io` with an HTTP 429: re-run `release.yml` for the same tag once the
window has passed.

Secrets: `CARGO_REGISTRY_TOKEN` (crates.io API token) and
`RELEASE_PLZ_TOKEN` (a PAT that can trigger workflows; bump opens the
version pull request with it so CI runs on it).

## Secrets

Two secrets must exist in the repo settings
(`Settings → Secrets and variables → Actions → New repository secret`):

| Secret | Where to get it | Used by |
| --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | crates.io → Account Settings → API Tokens → New Token (needs `publish-new` and `publish-update` scopes; if you restrict it by crate, use name patterns such as `llm-*` or list every crate released from here, since a new crate has no name to pick yet). Add with `gh secret set CARGO_REGISTRY_TOKEN --repo pyrlyn/crates-packages` | `release.yml`, job `crates-io`: `cargo publish -p <crate>` |
| `RELEASE_PLZ_TOKEN` | A fine-grained PAT (or GitHub App token) with **Contents** and **Pull requests** read/write on this repo — see https://release-plz.dev/docs/github/token. The default `GITHUB_TOKEN` cannot trigger `release.yml` from the release PR it opens, so without this the release PR would land without CI. Add with `gh secret set RELEASE_PLZ_TOKEN --repo pyrlyn/crates-packages` | `bump.yml`: pushes the version branch and opens the version PR |

Without `CARGO_REGISTRY_TOKEN` the release stops at the `crates-io` job with
a loud error and publishes nothing; the tag and the draft release already exist,
so re-run `release.yml` for that tag once the secret is set.
Without `RELEASE_PLZ_TOKEN` bump falls back to GITHUB_TOKEN, which cannot
open the pull request (workflow permissions), so it fails before any tag.

## License

Licensed under the [GNU General Public License v3.0 or later](LICENSE)
(`GPL-3.0-or-later`).
