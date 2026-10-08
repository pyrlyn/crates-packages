//! Tracing setup with secret redaction.
//!
//! Lines are scrubbed before they are stored. There is no OTLP exporter:
//! a process that only needs local logs should not open a collector socket.

use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};

use regex::Regex;
use tracing_subscriber::fmt::MakeWriter;

const REDACTED: &str = "[REDACTED]";

const VALUE_PATTERN: &str = r"(?i)\bbearer\s+\S+|\bsk-[A-Za-z0-9_-]{8,}";

const PAIR_PATTERN: &str = concat!(
    r"(?i)\b([\w.-]*(?:token|password|secret|authorization|credential)[\w.-]*)",
    r#"(\s*[=:]\s*)("[^"]*"|'[^']*'|\S+)"#,
);

/// Why a captured log could not be read back.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The capture buffer lock was poisoned. The secret is not copied into this error.
    #[error("log buffer lock failed")]
    Lock,
}

struct Patterns {
    value: Regex,
    pair: Regex,
}

fn patterns() -> Option<&'static Patterns> {
    static PATTERNS: OnceLock<Option<Patterns>> = OnceLock::new();
    PATTERNS
        .get_or_init(|| {
            Some(Patterns {
                value: Regex::new(VALUE_PATTERN).ok()?,
                pair: Regex::new(PAIR_PATTERN).ok()?,
            })
        })
        .as_ref()
}

/// `text` with credential-shaped values and secret-named pairs masked.
///
/// If the patterns cannot be compiled, the whole text is masked.
#[must_use]
pub fn scrub_line(text: &str) -> String {
    let Some(patterns) = patterns() else {
        return REDACTED.to_owned();
    };
    let after_values = patterns.value.replace_all(text, REDACTED);
    patterns
        .pair
        .replace_all(&after_values, "${1}${2}[REDACTED]")
        .into_owned()
}

/// In-memory log that stores only scrubbed bytes.
#[derive(Clone, Default)]
pub struct Capture {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Capture {
    /// An empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Scrubbed text collected so far.
    ///
    /// # Errors
    ///
    /// [`Error::Lock`] when the buffer lock is poisoned.
    pub fn text(&self) -> Result<String, Error> {
        let buf = self.buf.lock().map_err(|_| Error::Lock)?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}

impl<'a> MakeWriter<'a> for Capture {
    type Writer = RedactingWriter;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            buf: Arc::clone(&self.buf),
        }
    }
}

/// Writer that scrubs each chunk before appending it to the capture.
pub struct RedactingWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Write for RedactingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let clean = scrub_line(&String::from_utf8_lossy(buf));
        let mut guard = self
            .buf
            .lock()
            .map_err(|_| io::Error::other("log buffer lock failed"))?;
        guard.extend_from_slice(clean.as_bytes());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt;

    #[test]
    fn redacts_a_bearer_token_and_a_secret_field() {
        let line =
            scrub_line("authorization=Bearer sk-fixturevalue1234 password=fixture-value hello");
        assert!(line.contains("[REDACTED]"));
        assert!(!line.contains("sk-fixturevalue1234"));
        assert!(!line.contains("fixture-value"));
        assert!(line.contains("hello"));
    }

    #[test]
    fn a_tracing_line_is_stored_without_the_secret() {
        let capture = Capture::new();
        let subscriber = fmt().with_ansi(false).with_writer(capture.clone()).finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(password = "fixture-value", "ready");
        });
        let text = capture.text().unwrap();
        assert!(text.contains("ready"));
        assert!(!text.contains("fixture-value"));
        assert!(text.contains("[REDACTED]"));
    }
}
