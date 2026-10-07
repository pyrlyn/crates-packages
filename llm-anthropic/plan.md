# llm-anthropic

The Anthropic Messages wire for LLM agent loops: request translation, SSE stream parser and a streaming client.

| # | Status | Priority | Complexity | Readiness | Agent |
| --- | --- | --- | --- | --- | --- |
| T2 | todo | P2 | 3 | 0% | |

### T2. Adopt llm-anthropic in cox (T1.6, needs publication)

cox's `cox-provider-anthropic` drops its own `request`, `stream`, `wire`, `build.rs` and `schema/` and re-exports them from this crate, so `cox_provider::anthropic::request::build_body` and every other old path keep working. cox keeps what is cox-only: it builds `llm_anthropic::Transport` from `cox_protocol::config::Transport` and passes `"cox"` and `COX_KEYRING` to `AnthropicProvider::new`; the `cox-models` `Api::Anthropic` arm of `effort_for` and `supports_adaptive_thinking` become one copy (this crate's `request::supports_adaptive_thinking`, or cox-models keeps the list and the crate takes the answer as an argument). cox's `Replay` keeps feeding cassettes through `AnthropicStream`. Blocked on publication of `llm-anthropic`, `llm-http` and `llm-wire`: cox can only depend on released crates, because a path dependency breaks cox CI. Done when cox builds against the published crates, its tests and snapshots pass unchanged, and the moved code is gone from `cox-provider-anthropic`.
