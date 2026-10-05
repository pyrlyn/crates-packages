# cargo-changed-packages

Workspace packages a set of changed paths affects, including reverse dependencies.

Built on `guppy` and `determinator`. The package graph comes from `cargo metadata`
run in the workspace root. Changed paths are relative to that root, as produced by
`git-changed-paths`. A change to `Cargo.toml`, `Cargo.lock`, `.cargo/config*`,
`rust-toolchain*` or `build.rs` returns `all: true`, because the old package graph
would need a checkout of the base ref.

```rust
use cargo_changed_packages::affected;
use std::path::{Path, PathBuf};

let result = affected(Path::new("."), &[PathBuf::from("b/src/lib.rs")])?;
if let Some(filter) = result.nextest_filter() {
    println!("cargo nextest run -E '{filter}'");
}
```
