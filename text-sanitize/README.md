# text-sanitize

Strip escape sequences, bidi controls, and suspicious zero-width runs from untrusted plain text. `sanitize_header` also removes CR, LF, and tab so a value cannot start another header line.

The fuzz target lives in `fuzz/` and is its own Cargo workspace, so `cargo test` of this crate does not need a nightly toolchain. Run it with `cargo +nightly fuzz run plain` from `fuzz/` after installing `cargo-fuzz`.
