//! Render [D2](https://d2lang.com) diagrams from Rust.
//!
//! The default backend runs the `d2` executable, so every D2 feature, layout
//! engine and output format works exactly as on the command line. Its stderr
//! is parsed into structured [`Diagnostic`]s, a successful render returns a
//! typed [`RenderReport`], and rendered SVG is scanned for its size and the
//! keys of every shape and connection ([`SvgInfo`]).
//!
//! ```no_run
//! use d2_render::{Format, Renderer, RenderOptions};
//! use std::path::Path;
//!
//! let renderer = Renderer::new().options(RenderOptions::new().theme(200).pad(20));
//! let out = renderer.render_str("a -> b", Path::new("out/ab.svg"), Format::Svg)?;
//! println!("wrote {}", out.display());
//! # Ok::<(), d2_render::Error>(())
//! ```
//!
//! Optional features: `native` (pure-Rust backend for a subset of D2),
//! `tui` (ratatui widget), `web` (HTML embedding helpers) and `watch`
//! (debounced re-render on change). See the README for each.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod backend;
mod cli;
pub mod diagnostic;
mod error;
pub mod fresh;
mod options;
mod renderer;
mod report;
pub mod svg;

#[cfg(feature = "native")]
pub mod native;
#[cfg(feature = "tui")]
pub mod tui;
#[cfg(feature = "watch")]
pub mod watch;
#[cfg(feature = "web")]
pub mod web;

pub use backend::{Backend, BackendOutput};
pub use cli::{CliBackend, D2_BIN_ENV};
pub use diagnostic::{parse_stderr, Diagnostic, ParsedOutput, Severity};
pub use error::{Error, Result};
pub use fresh::{Freshness, StaleOutput};
#[cfg(feature = "native")]
pub use native::NativeBackend;
pub use options::{Format, Layout, RenderOptions};
pub use renderer::Renderer;
pub use report::RenderReport;
pub use svg::{ElementKind, SvgElement, SvgInfo};
