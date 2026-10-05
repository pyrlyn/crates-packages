# cargo-changed-packages — completed tasks

### T1. Affected workspace packages via determinator

`affected(workspace_root, changed)` returns the workspace packages whose files changed
plus their reverse dependencies, from a `guppy` package graph and `determinator`. Paths
that alter the build or the graph (`Cargo.toml`, `Cargo.lock`, `.cargo/config*`,
`rust-toolchain*`, `build.rs`) or cannot be mapped select everything (`all: true`).
`Affected::nextest_filter` renders a nextest filterset.
