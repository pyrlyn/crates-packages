# llm-http

HTTP plumbing for LLM provider wires, on top of `reqwest`.

- `http`: `client_with_timeout` (bounded connect timeout, idle read timeout),
  `resolve_key` (env var first, then the platform keyring; a switch variable
  turns the keyring off for tests and dev runs), `bearer` and `api_key`
  (sensitive auth headers), and `map_http_error` (a non-2xx response to a
  `llm_wire::ProviderError`).
- `retry`: `stream_with_retry` runs one provider attempt with exponential
  backoff and jitter, honours `retry-after`, and retries only before the
  first event reached the caller.
- `sse`: `sse_stream` turns a byte stream into `(event, data)` frames and
  `parse_sse_str` parses a whole body for fixtures.

```rust
use llm_http::http::{bearer, resolve_key};

fn auth_header() -> Result<reqwest::header::HeaderValue, llm_wire::ProviderError> {
    let key = resolve_key("OPENAI_API_KEY", "my-app", "openai", "MY_APP_KEYRING")?;
    bearer(&key)
}
```

Licensed `GPL-3.0-or-later OR LicenseRef-Royalty-Free`; see the repository's
`LICENSE` and `LICENSE-ROYALTY-FREE.md`.
