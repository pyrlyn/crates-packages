# secret-store

Resolve a secret from a non-empty environment value, then from an injected store. [`MemoryBackend`](src/memory.rs) is the stand-in tests use. The platform store is the `os` feature and is not built by the default test suite, so tests never open the real credential store.

Nothing in this crate prints a secret. `Debug` and `Display` for `Secret` are `[REDACTED]`.
