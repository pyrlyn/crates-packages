//! Fuzz plain text and header sanitizing. A crash is a panic in the harness.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let plain = text_sanitize::sanitize_plain(data);
    assert!(!plain.contains('\u{1b}'));
    let header = text_sanitize::sanitize_header(data);
    assert!(!header.contains('\n'));
    assert!(!header.contains('\r'));
});
