//! Replay cassettes for scripted LLM calls.
//!
//! A cassette stores the request and the response after secret redaction, so
//! a fixture file never keeps a bearer token. Replay compares the redacted
//! request. [`Cassette`] is an [`llm_http::Transport`]: it does not open a socket.

use llm_http::{Error as HttpError, HttpRequest, HttpResponse, Transport};
use serde::{Deserialize, Serialize};

/// Why a cassette could not answer.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The cassette text is not a list of exchanges.
    #[error("cassette is not valid JSON")]
    Json,
    /// No stored exchange matches the redacted request.
    #[error("cassette has no matching exchange")]
    Miss,
}

/// One recorded call, already redacted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exchange {
    /// Redacted request body.
    pub request: String,
    /// Redacted response body.
    pub response: String,
}

/// A list of exchanges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Cassette {
    exchanges: Vec<Exchange>,
}

impl Cassette {
    /// An empty cassette.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses JSON written by [`Cassette::save`].
    ///
    /// # Errors
    ///
    /// [`Error::Json`] when `text` is not a cassette.
    pub fn load(text: &str) -> Result<Self, Error> {
        serde_json::from_str(text).map_err(|_| Error::Json)
    }

    /// JSON for a fixture file. Exchanges are already redacted.
    #[must_use]
    pub fn save(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "[]".to_owned())
    }

    /// Stores `request` and `response` after redaction.
    pub fn record(&mut self, request: &str, response: &str) {
        self.exchanges.push(Exchange {
            request: redact(request),
            response: redact(response),
        });
    }

    /// The stored response for `request`, compared after redaction.
    ///
    /// # Errors
    ///
    /// [`Error::Miss`] when no exchange matches.
    pub fn replay<'a>(&'a self, request: &str) -> Result<&'a str, Error> {
        let key = redact(request);
        self.exchanges
            .iter()
            .find(|exchange| exchange.request == key)
            .map(|exchange| exchange.response.as_str())
            .ok_or(Error::Miss)
    }
}

/// Masks credential-shaped text. Same scrubber as `telemetry-setup`.
#[must_use]
pub fn redact(text: &str) -> String {
    telemetry_setup::scrub_line(text)
}

impl Transport for Cassette {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpError> {
        let body = String::from_utf8_lossy(&request.body);
        let response = self.replay(&body).map_err(|_| HttpError::Transport)?;
        Ok(HttpResponse {
            status: 200,
            body: response.as_bytes().to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cassette_replays_without_keeping_the_token() {
        let mut cassette = Cassette::new();
        cassette.record(
            r#"{"authorization":"Bearer sk-fixturevalue1234"}"#,
            r#"{"text":"hello"}"#,
        );
        let saved = cassette.save();
        assert!(!saved.contains("sk-fixturevalue1234"));
        assert!(saved.contains("[REDACTED]"));
        let loaded = Cassette::load(&saved).unwrap();
        let response = loaded
            .replay(r#"{"authorization":"Bearer sk-fixturevalue1234"}"#)
            .unwrap();
        assert_eq!(response, r#"{"text":"hello"}"#);

        let http = loaded
            .execute(&HttpRequest {
                method: "POST".into(),
                url: "http://example.test/v1/chat".into(),
                headers: Vec::new(),
                body: br#"{"authorization":"Bearer sk-fixturevalue1234"}"#.to_vec(),
            })
            .unwrap();
        assert_eq!(http.status, 200);
        assert_eq!(http.body, br#"{"text":"hello"}"#);
    }
}
