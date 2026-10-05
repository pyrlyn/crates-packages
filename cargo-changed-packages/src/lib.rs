//! Workspace packages affected by a set of changed paths.
//!
//! Paths are matched to workspace members by `determinator` (the guppy project's crate for
//! this), which also adds every reverse dependency of a changed package.
#![deny(missing_docs)]

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use determinator::Determinator;
use guppy::{MetadataCommand, graph::DependencyDirection};

/// Why the affected packages could not be computed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `cargo metadata` failed or its output could not be understood.
    #[error("cannot read the cargo workspace: {0}")]
    Metadata(#[from] Box<guppy::Error>),
    /// Cargo resolved a different workspace root than the one requested, so the changed
    /// paths would be matched against the wrong directory.
    #[error("{requested} is not the workspace root (cargo reports {actual})")]
    NotWorkspaceRoot {
        /// The directory passed to [`affected`].
        requested: PathBuf,
        /// The workspace root cargo found.
        actual: PathBuf,
    },
}

/// The workspace packages affected by a change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Affected {
    /// Workspace package names that changed or depend, directly or not, on a changed
    /// package. Empty when [`all`](Self::all) is set: check `all` first.
    pub packages: BTreeSet<String>,
    /// The answer could not be computed exactly, so every package must be treated as
    /// affected.
    pub all: bool,
}

impl Affected {
    fn everything() -> Self {
        Self {
            packages: BTreeSet::new(),
            all: true,
        }
    }

    /// A nextest filterset selecting the affected packages, for `cargo nextest run -E`.
    ///
    /// `None` means no filter is needed: run everything. An empty set gives `none()`.
    /// Package names use the `=` (equality) matcher because the default is a glob and
    /// the reference says to always prefix programmatically built expressions; operators
    /// and `none()` are defined in the filterset reference at
    /// <https://nexte.st/docs/filtersets/reference/> (checked 2026-10-05).
    pub fn nextest_filter(&self) -> Option<String> {
        if self.all {
            return None;
        }
        if self.packages.is_empty() {
            return Some("none()".to_owned());
        }
        let terms: Vec<String> = self
            .packages
            .iter()
            .map(|p| format!("package(={p})"))
            .collect();
        Some(terms.join(" | "))
    }
}

/// Computes the workspace packages that `changed` affects, reverse dependencies included.
///
/// `workspace_root` is the directory holding the `[workspace]` manifest; `changed` paths
/// are relative to it. They are repo-relative as `git-changed-paths` produces them, which
/// is the same thing when the workspace sits at the repository top level. A path outside
/// every package (a root `README.md`) affects nothing.
///
/// The old package graph would need a checkout of the base ref, so a change to anything
/// that alters the graph or the build (`Cargo.toml`, `Cargo.lock`, `.cargo/config*`,
/// `rust-toolchain*`, `build.rs`) returns `all: true` instead of guessing. So does a path
/// that cannot be mapped to the workspace (non-UTF-8, absolute elsewhere, or escaping it).
/// In those cases cargo is not run and [`Affected::packages`] is empty.
///
/// # Errors
///
/// Fails when `cargo metadata` cannot describe the workspace or `workspace_root` is not
/// the workspace root.
pub fn affected(workspace_root: &Path, changed: &[PathBuf]) -> Result<Affected, Error> {
    let Some(relative) = relative_paths(workspace_root, changed) else {
        return Ok(Affected::everything());
    };
    if relative.iter().any(|p| alters_build(p)) {
        return Ok(Affected::everything());
    }

    let graph = MetadataCommand::new()
        .current_dir(workspace_root)
        .build_graph()
        .map_err(Box::new)?;
    // std::fs::canonicalize resolves symlinks such as macOS's /tmp, which cargo also does.
    let actual = graph.workspace().root();
    if workspace_root.canonicalize().ok().as_deref() != actual.canonicalize().ok().as_deref() {
        return Err(Error::NotWorkspaceRoot {
            requested: workspace_root.to_path_buf(),
            actual: actual.as_std_path().to_path_buf(),
        });
    }

    // Both graphs are the same: the base graph is unavailable, and manifest changes already
    // returned above, so only file-to-package matching and reverse dependencies remain.
    let mut determinator = Determinator::new(&graph, &graph);
    determinator.add_changed_paths(&relative);
    let set = determinator.compute();
    let packages = set
        .affected_set
        .packages(DependencyDirection::Forward)
        .filter(|p| p.in_workspace())
        .map(|p| p.name().to_owned())
        .collect();
    Ok(Affected {
        packages,
        all: false,
    })
}

/// Normalises to `/`-separated workspace-relative strings; `None` when any path cannot be.
fn relative_paths(root: &Path, changed: &[PathBuf]) -> Option<Vec<String>> {
    changed
        .iter()
        .map(|path| {
            let path = if path.is_absolute() {
                path.strip_prefix(root).ok()?
            } else {
                path.as_path()
            };
            let mut parts = Vec::new();
            for component in path.components() {
                match component {
                    Component::Normal(part) => parts.push(part.to_str()?),
                    Component::CurDir => {}
                    _ => return None,
                }
            }
            Some(parts.join("/"))
        })
        .collect()
}

fn alters_build(path: &str) -> bool {
    let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
    matches!(name, "Cargo.toml" | "Cargo.lock" | "build.rs")
        || name.starts_with("rust-toolchain")
        || (name.starts_with("config") && (dir == ".cargo" || dir.ends_with("/.cargo")))
}
