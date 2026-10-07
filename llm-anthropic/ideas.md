# Ideas

- cox re-vendors its copy of the OpenAPI snapshot with its `cox-vendor anthropic-spec` script. Move that script (or a small Rust equivalent) next to `schema/` so the spec is re-vendored from this crate, and cox stops carrying the file.
- `supports_adaptive_thinking` is a model-id prefix list that goes stale with each model family. Let the caller pass the answer from its own model catalog instead of keeping a list here.
- `count_tokens` returns `Unsupported` because cox has not implemented it either; `POST /v1/messages/count_tokens` takes the same body as `build_body`.
