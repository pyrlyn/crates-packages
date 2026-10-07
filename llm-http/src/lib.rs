//! HTTP plumbing every network provider wire needs on top of a plain
//! `reqwest` call: connection-pooled client construction, credential
//! resolution (env var, then platform keyring), auth headers and non-2xx →
//! [`ProviderError`](llm_wire::ProviderError) mapping ([`http`]); generic
//! Server-Sent-Events framing ([`sse`]); and the retry/backoff policy wrapped
//! around one provider stream ([`retry`]), plus the [`Transport`] section
//! every wire's constructor takes.
//!
//! Extracted from cox's `cox-provider-http` so cox and aulo share one copy.
//! The error and event types come from [`llm_wire`].

pub mod http;
pub mod retry;
pub mod sse;
mod transport;

pub use transport::Transport;
