# crates-packages

[![ci](https://github.com/pyrlyn/crates-packages/actions/workflows/ci.yml/badge.svg)](https://github.com/pyrlyn/crates-packages/actions/workflows/ci.yml) [![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=listepo_crates-packages&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=listepo_crates-packages) [![Coverage](https://sonarcloud.io/api/project_badges/measure?project=listepo_crates-packages&metric=coverage)](https://sonarcloud.io/component_measures?id=listepo_crates-packages&metric=coverage) [![Tests](https://img.shields.io/sonar/tests/listepo_crates-packages?server=https%3A%2F%2Fsonarcloud.io&compact_message)](https://sonarcloud.io/component_measures?id=listepo_crates-packages&metric=tests)

Small, focused Rust crates shared by `ketch` and `rtok`.

| Crate | What it does |
| --- | --- |
| [`file-backup`](file-backup) | Copy a file to `<name>.bak-<unix-seconds>` beside it before replacing it |

## Commands

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

## Release

- Pushing to `main` runs CI (commits lint on PRs, fmt, clippy, unit tests,
  publish dry-run).
- A release is `gh workflow run bump.yml` (Actions → Bump and release), the
  only way to a release and the only thing that creates a
  `file-backup-v<version>` tag. It runs `release-plz update` (version and
  changelog from the conventional commits), opens a pull request with that one
  commit, rebase-merges it once the required checks are green, tags the commit
  that landed on main, creates a draft release and dispatches `release.yml`
  on the tag: the gate again, crates.io, then the draft is published.
  Anything failing before the merge closes the pull request: no tag, no
  release. `-f dry-run=true` opens the pull request, waits for its checks and
  closes it.

Secrets: `CARGO_REGISTRY_TOKEN` (crates.io API token) and
`RELEASE_PLZ_TOKEN` (a PAT that can trigger workflows; bump opens the
version pull request with it so CI runs on it).

## Secrets

Two secrets must exist in the repo settings
(`Settings → Secrets and variables → Actions → New repository secret`):

| Secret | Where to get it | Used by |
| --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | crates.io → Account Settings → API Tokens → New Token (needs `publish` scope; restrict it to the `file-backup` crate). Add with `gh secret set CARGO_REGISTRY_TOKEN --repo pyrlyn/crates-packages` | `release.yml`, job `crates-io`: `cargo publish -p file-backup` |
| `RELEASE_PLZ_TOKEN` | A fine-grained PAT (or GitHub App token) with **Contents** and **Pull requests** read/write on this repo — see https://release-plz.dev/docs/github/token. The default `GITHUB_TOKEN` cannot trigger `release.yml` from the release PR it opens, so without this the release PR would land without CI. Add with `gh secret set RELEASE_PLZ_TOKEN --repo pyrlyn/crates-packages` | `bump.yml`: pushes the version branch and opens the version PR |

Without `CARGO_REGISTRY_TOKEN` the release stops at the `crates-io` job with
a loud error and publishes nothing — the tag is never created.
Without `RELEASE_PLZ_TOKEN` bump falls back to GITHUB_TOKEN, which cannot
open the pull request (workflow permissions), so it fails before any tag.

## License

Licensed under the [GNU General Public License v3.0 or later](LICENSE)
(`GPL-3.0-or-later`).
