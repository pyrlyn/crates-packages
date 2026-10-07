# llm-http — completed tasks

### T1. Extract llm-http: credentials, retries, SSE framing

Copied from cox's `cox-provider-http` so aulo can share the plumbing; cox is untouched. Moved: `http` (`client_with_timeout`, `resolve_key`, `resolve_key_with`, `bearer`, `api_key`, `error_message`, `parse_context_too_long`, `map_http_error`), `retry` (`Policy`, `retryable`, `stream_with_retry`) and `sse` (`sse_stream`, `parse_sse_str`), with their unit tests. `ProviderError`, `ProviderEvent` and `Usage` now come from `llm-wire`. Differences from cox: `resolve_key` takes the keyring service and the switch variable name (cox hard-coded `cox` and `COX_KEYRING`), and `keyring_enabled` moved here from `cox-protocol`. The tests inject the environment instead of calling `set_var` (this crate forbids `unsafe`) and gain a blank-env-var case and the sensitive-header and no-key-in-errors checks; the test that read cox's `.cargo/config.toml` is gone. `reqwest` no longer enables `json` and `stream`; the wires that send requests enable them. Check: `cargo test -p llm-http` (19 tests), `cargo clippy -p llm-http --all-targets -- -D warnings`, `cargo fmt --all --check` and `cargo build --workspace` green.

### T3. One Transport for the wires

Aulo follow-up to T1.3 and T1.4. `Transport` (base URL, key env var, idle timeout, max retries) is defined here and used by `llm-openai` and `llm-anthropic`, which each kept a private copy; `llm_openai::config::Transport` and `llm_anthropic::Transport` are gone. `llm-wire` stays free of transport settings. Check: `cargo test --workspace --all-features`, clippy and `cargo fmt --all --check` green.
