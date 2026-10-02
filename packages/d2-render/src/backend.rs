//! The pluggable rendering backend.

use std::path::Path;

use crate::diagnostic::Diagnostic;
use crate::{Format, RenderOptions, Result};

/// What a backend reports about a successful render.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackendOutput {
    /// Warnings printed while rendering.
    pub warnings: Vec<String>,
    /// Located warnings, when the backend could place them.
    pub diagnostics: Vec<Diagnostic>,
    /// Version string of the tool that produced the output.
    pub version: Option<String>,
    /// Raw standard error, empty for in-process backends.
    pub stderr: String,
}

/// Something that turns a `.d2` file into an image.
///
/// [`crate::CliBackend`] (the `d2` executable) is the default and supports
/// the whole language; [`crate::NativeBackend`] (feature `native`) renders a
/// subset in-process.
pub trait Backend: Send + Sync + std::fmt::Debug {
    /// Short name for reports, e.g. `"cli"`.
    fn name(&self) -> &'static str;

    /// Whether this backend can write `format`.
    fn supports(&self, format: Format) -> bool;

    /// Render `input` to `output`. `output` already has the extension that
    /// matches `format`, and its parent directory exists.
    fn render(
        &self,
        input: &Path,
        output: &Path,
        format: Format,
        options: &RenderOptions,
    ) -> Result<BackendOutput>;

    /// Check that `input` compiles without writing anything.
    fn validate(&self, input: &Path) -> Result<()>;

    /// Version of the underlying renderer.
    fn version(&self) -> Result<String>;
}
