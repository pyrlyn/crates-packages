// Copyright (c) 2026 Ivan Tugay
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A host for WebAssembly plugins, shared by applications that each define
//! their own plugin contract. It owns what every such host needs and nothing
//! about what plugins are for: loading a module with extism under a memory
//! cap, running each plugin on a worker thread of its own, cancelling a call
//! at its deadline, and serving calls the application is waiting on before
//! background events.
//!
//! The application supplies its host functions through [`HostEnv`] and names
//! its required export and limits in [`Options`].
//!
//! - [`host`] — [`PluginHost`]: load, the two lanes, deadlines, the memory cap.
//! - [`error`] — [`PluginError`], mapped from `extism::Error`.

pub mod error;
pub mod host;

pub use error::PluginError;
/// Re-exported so host functions are built against the extism this crate runs.
pub use extism;
pub use host::{HostEnv, Lane, Limits, Options, PluginHost};
