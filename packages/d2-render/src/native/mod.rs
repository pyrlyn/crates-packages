// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// The native backend is a Rust rewrite of a subset of D2
// (parser, compiler, dagre-style layout, SVG renderer),
// Copyright 2022 Terrastruct, Inc. See the README for what is covered.

//! Pure-Rust backend for a subset of D2 (feature `native`).
//!
//! Supported: shapes, labels (including `|md ...|` block strings shown as
//! plain text), nested containers, `->` `<-` `<->` `--` connections and
//! chains, connection labels, `direction`, `width`/`height`, `_` parent
//! references, and the common `style` keywords. Everything else D2 accepts
//! is either reported as a warning and ignored (icons, tooltips, `near`,
//! classes, vars, arrowhead shapes, grids) or rejected with a diagnostic
//! (imports, globs, `(a -> b)[0]` references, arrays). Output is SVG with
//! D2's "Neutral Default" colours; layout is a dagre-like layered layout,
//! so positions differ from the CLI's.

pub mod graph;
pub mod layout;
pub mod parser;
pub mod render;
mod text;
mod theme;

use std::fs;
use std::path::Path;

use crate::backend::{Backend, BackendOutput};
use crate::diagnostic::Diagnostic;
use crate::{Error, Format, RenderOptions, Result};

/// SVG and warnings from an in-memory render.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeOutput {
    /// The SVG document.
    pub svg: String,
    /// Unsupported features that were ignored.
    pub warnings: Vec<String>,
}

/// Parse, lay out and draw `source`. Errors carry D2-style diagnostics.
pub fn render_svg(source: &str, options: &RenderOptions) -> Result<NativeOutput> {
    let ast = parser::parse(source).map_err(syntax)?;
    let g = graph::compile(&ast).map_err(syntax)?;
    let mut warnings = g.warnings.clone();
    warnings.extend(option_warnings(options));
    let l = layout::layout(&g);
    let svg = render::render(
        &g,
        &l,
        &render::SvgOptions {
            pad: options.pad.map_or(100.0, f64::from),
            xml_tag: !options.no_xml_tag,
            version: !options.omit_version,
            salt: options.salt.clone().unwrap_or_default(),
        },
    );
    Ok(NativeOutput { svg, warnings })
}

/// Parse and compile without drawing.
pub fn check(source: &str) -> Result<()> {
    let ast = parser::parse(source).map_err(syntax)?;
    graph::compile(&ast).map_err(syntax)?;
    Ok(())
}

fn syntax(diagnostics: Vec<Diagnostic>) -> Error {
    let stderr = diagnostics.iter().map(|d| format!("err: {d}\n")).collect();
    Error::Syntax {
        diagnostics,
        stderr,
    }
}

fn with_path(e: Error, path: &Path) -> Error {
    match e {
        Error::Syntax {
            mut diagnostics, ..
        } => {
            for d in &mut diagnostics {
                d.path = Some(path.to_path_buf());
            }
            let stderr = diagnostics.iter().map(|d| format!("err: {d}\n")).collect();
            Error::Syntax {
                diagnostics,
                stderr,
            }
        }
        other => other,
    }
}

fn option_warnings(o: &RenderOptions) -> Vec<String> {
    let mut w = Vec::new();
    let mut ignored = |name: &str| {
        w.push(format!("option {name} is ignored by the native backend"));
    };
    if o.theme.is_some_and(|t| t != 0) {
        ignored("--theme (only theme 0 is drawn)");
    }
    if o.dark_theme.is_some() {
        ignored("--dark-theme");
    }
    if o.layout
        .as_ref()
        .is_some_and(|l| *l != crate::Layout::Dagre)
    {
        ignored("--layout (a dagre-like layout is always used)");
    }
    if o.sketch {
        ignored("--sketch");
    }
    if o.center {
        ignored("--center");
    }
    if o.scale.is_some() {
        ignored("--scale");
    }
    if o.target.is_some() {
        ignored("--target");
    }
    if o.animate_interval_ms.is_some() {
        ignored("--animate-interval");
    }
    w
}

/// In-process renderer for a subset of D2; SVG only.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeBackend;

impl NativeBackend {
    /// A new backend (stateless).
    pub fn new() -> Self {
        NativeBackend
    }
}

impl Backend for NativeBackend {
    fn name(&self) -> &'static str {
        "native"
    }

    fn supports(&self, format: Format) -> bool {
        format == Format::Svg
    }

    fn render(
        &self,
        input: &Path,
        output: &Path,
        format: Format,
        options: &RenderOptions,
    ) -> Result<BackendOutput> {
        if format != Format::Svg {
            return Err(Error::UnsupportedFormat {
                format,
                backend: self.name(),
            });
        }
        let source = fs::read_to_string(input)?;
        let out = render_svg(&source, options).map_err(|e| with_path(e, input))?;
        fs::write(output, out.svg)?;
        Ok(BackendOutput {
            stderr: out
                .warnings
                .iter()
                .map(|w| format!("warn: {w}\n"))
                .collect(),
            warnings: out.warnings,
            diagnostics: Vec::new(),
            version: Some(self.version()?),
        })
    }

    fn validate(&self, input: &Path) -> Result<()> {
        let source = fs::read_to_string(input)?;
        check(&source).map_err(|e| with_path(e, input))
    }

    fn version(&self) -> Result<String> {
        Ok(format!("d2-render-native v{}", env!("CARGO_PKG_VERSION")))
    }
}
