//! Regenerated C and C# bindings, compared with the committed files.
//!
//! `BLESS` (or a name the caller passes) rewrites the committed file. A
//! mismatch is an error that includes the differing lines. Windows checkouts
//! may store CRLF; the comparison uses LF.

use std::fs;
use std::path::{Path, PathBuf};

/// Why a binding check failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A committed file could not be read.
    #[error("could not read {path}: {detail}")]
    Read {
        /// File that was compared.
        path: String,
        /// Filesystem message.
        detail: String,
    },
    /// A blessed file could not be written.
    #[error("could not write {path}: {detail}")]
    Write {
        /// File that was blessed.
        path: String,
        /// Filesystem message.
        detail: String,
    },
    /// cbindgen could not render a header.
    #[error("cbindgen failed: {0}")]
    Cbindgen(String),
    /// csbindgen could not render C# bindings.
    #[error("csbindgen failed: {0}")]
    Csbindgen(String),
    /// The committed file does not match the rendering.
    #[error("{path} is stale:\n{diff}")]
    Stale {
        /// File that drifted.
        path: String,
        /// Lines that differ.
        diff: String,
    },
}

/// `true` when `value` is set. [`check_bless`] passes `BLESS`.
#[must_use]
pub fn bless_requested(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some()
}

/// Compares `rendered` with `path`, or rewrites `path` when `bless` is set.
///
/// # Errors
///
/// [`Error::Read`] when the committed file exists but cannot be read.
/// [`Error::Write`] when blessing fails.
/// [`Error::Stale`] when the texts differ and `bless` is false. A missing
/// file is an empty committed text, so the diff shows every generated line.
pub fn check(path: &Path, rendered: &str, bless: bool) -> Result<(), Error> {
    let rendered = rendered.replace("\r\n", "\n");
    if bless {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| Error::Write {
                path: path.display().to_string(),
                detail: err.to_string(),
            })?;
        }
        return fs::write(path, &rendered).map_err(|err| Error::Write {
            path: path.display().to_string(),
            detail: err.to_string(),
        });
    }
    let committed = match fs::read_to_string(path) {
        Ok(text) => text.replace("\r\n", "\n"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(Error::Read {
                path: path.display().to_string(),
                detail: err.to_string(),
            });
        }
    };
    if committed == rendered {
        Ok(())
    } else {
        Err(Error::Stale {
            path: path.display().to_string(),
            diff: unified(&committed, &rendered),
        })
    }
}

/// [`check`] with the `BLESS` environment variable.
///
/// # Errors
///
/// Same as [`check`].
pub fn check_bless(path: &Path, rendered: &str) -> Result<(), Error> {
    check(
        path,
        rendered,
        bless_requested(std::env::var_os("BLESS").as_deref()),
    )
}

/// [`check`] with a caller-chosen variable name, such as `SCULL_BLESS`.
///
/// # Errors
///
/// Same as [`check`].
pub fn check_env(path: &Path, rendered: &str, bless_var: &str) -> Result<(), Error> {
    check(
        path,
        rendered,
        bless_requested(std::env::var_os(bless_var).as_deref()),
    )
}

/// Header cbindgen renders for the crate at `crate_dir`.
///
/// Uses `cbindgen.toml` in that directory when it exists.
///
/// # Errors
///
/// [`Error::Cbindgen`] when the header cannot be rendered.
pub fn cbindgen_header(crate_dir: &Path) -> Result<String, Error> {
    let mut builder = cbindgen::Builder::new().with_crate(crate_dir);
    let config_path = crate_dir.join("cbindgen.toml");
    if config_path.exists() {
        let config = cbindgen::Config::from_file(&config_path)
            .map_err(|err| Error::Cbindgen(err.to_string()))?;
        builder = builder.with_config(config);
    }
    let bindings = builder
        .generate()
        .map_err(|err| Error::Cbindgen(err.to_string()))?;
    let mut rendered = Vec::new();
    bindings.write(&mut rendered);
    String::from_utf8(rendered).map_err(|_| Error::Cbindgen("header is not UTF-8".to_owned()))
}

/// C# bindings csbindgen renders from `sources`.
///
/// The file is written to `output` because csbindgen writes the path itself.
///
/// # Errors
///
/// [`Error::Csbindgen`] when generation fails.
/// [`Error::Read`] when the generated file cannot be read back.
pub fn csbindgen_csharp(
    sources: &[PathBuf],
    dll_name: &str,
    namespace: &str,
    class_name: &str,
    output: &Path,
) -> Result<String, Error> {
    let mut builder = csbindgen::Builder::new()
        .csharp_dll_name(dll_name)
        .csharp_namespace(namespace)
        .csharp_class_name(class_name);
    for source in sources {
        builder = builder.input_extern_file(source);
    }
    builder
        .generate_csharp_file(output)
        .map_err(|err| Error::Csbindgen(err.to_string()))?;
    fs::read_to_string(output).map_err(|err| Error::Read {
        path: output.display().to_string(),
        detail: err.to_string(),
    })
}

fn unified(committed: &str, rendered: &str) -> String {
    let mut out = String::new();
    let committed_lines: Vec<&str> = committed.lines().collect();
    let rendered_lines: Vec<&str> = rendered.lines().collect();
    let len = committed_lines.len().max(rendered_lines.len());
    for index in 0..len {
        let left = committed_lines.get(index).copied().unwrap_or("");
        let right = rendered_lines.get(index).copied().unwrap_or("");
        if left != right {
            out.push_str(&format!(
                "line {index}: committed {left:?} generated {right:?}\n"
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_match_passes_and_a_mismatch_shows_the_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("include.h");
        fs::write(&path, "int add(int a, int b);\n").unwrap();
        check(&path, "int add(int a, int b);\r\n", false).unwrap();
        let err = check(&path, "int sub(int a, int b);\n", false).unwrap_err();
        let Error::Stale { diff, .. } = err else {
            panic!("expected a diff");
        };
        assert!(diff.contains("sub"));
    }

    #[test]
    fn bless_rewrites_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("include.h");
        check(&path, "int add(int a);\n", true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "int add(int a);\n");
        assert!(!bless_requested(None));
        assert!(bless_requested(Some(std::ffi::OsStr::new("1"))));
    }

    #[test]
    fn cbindgen_and_csbindgen_regenerate() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"abi_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(
            src.join("lib.rs"),
            "#[no_mangle]\npub extern \"C\" fn abi_fixture_add(left: i32, right: i32) -> i32 {\n    left + right\n}\n",
        )
        .unwrap();
        let header = cbindgen_header(dir.path()).unwrap();
        assert!(header.contains("abi_fixture_add"), "{header}");
        let csharp_path = dir.path().join("Native.cs");
        let csharp = csbindgen_csharp(
            &[src.join("lib.rs")],
            "abi_fixture",
            "Abi.Native",
            "NativeMethods",
            &csharp_path,
        )
        .unwrap();
        assert!(csharp.contains("abi_fixture_add"), "{csharp}");
        let committed = dir.path().join("include").join("abi.h");
        check(&committed, &header, true).unwrap();
        check(&committed, &header, false).unwrap();
    }
}
