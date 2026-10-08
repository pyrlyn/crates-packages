//! JSON HTTP through an injected transport.
//!
//! The caller supplies every byte that would have crossed a socket. Retry
//! covers HTTP 429 and 5xx. There is no client and no sleep: [`Pause`] is
//! injected, and tests use [`NoPause`].

/// Why a call failed. Messages name the status, not a header or a body.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The transport could not complete a try.
    #[error("transport failed")]
    Transport,
    /// The status was not success, and retries are exhausted or the status is final.
    #[error("HTTP {status} after {attempts} attempts")]
    Status {
        /// Last HTTP status.
        status: u16,
        /// How many tries ran, including the first.
        attempts: u32,
    },
}

/// One outbound call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// HTTP method, such as `POST`.
    pub method: String,
    /// Absolute URL the transport should use. This crate does not resolve it.
    pub url: String,
    /// Header name and value pairs.
    pub headers: Vec<(String, String)>,
    /// Raw body.
    pub body: Vec<u8>,
}

/// One reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// HTTP status.
    pub status: u16,
    /// Raw body.
    pub body: Vec<u8>,
}

/// Something that can perform one HTTP call without this crate opening a socket.
pub trait Transport {
    /// Perform `request` once.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] when the call cannot be made.
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, Error>;
}

/// Wait between retries. Tests use [`NoPause`].
pub trait Pause {
    /// Called before try number `attempt` (the first retry is 1).
    fn pause(&self, attempt: u32);
}

/// A pause that returns immediately.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoPause;

impl Pause for NoPause {
    fn pause(&self, _attempt: u32) {}
}

/// Retries a POST against `T`.
#[derive(Debug)]
pub struct Client<T, P = NoPause> {
    transport: T,
    pause: P,
    attempts: u32,
}

impl<T> Client<T, NoPause> {
    /// Three tries, no delay.
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            pause: NoPause,
            attempts: 3,
        }
    }
}

impl<T, P> Client<T, P> {
    /// Replaces the try count. Zero becomes one try.
    #[must_use]
    pub fn with_attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts.max(1);
        self
    }

    /// Replaces the pause hook.
    #[must_use]
    pub fn with_pause<Q>(self, pause: Q) -> Client<T, Q> {
        Client {
            transport: self.transport,
            pause,
            attempts: self.attempts,
        }
    }
}

impl<T: Transport, P: Pause> Client<T, P> {
    /// POST `body` to `url` with `headers`.
    ///
    /// Retries HTTP 429 and 500..=599 until `attempts` is spent.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] when a try cannot be made.
    /// [`Error::Status`] when the final status is not 2xx.
    pub fn post(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<HttpResponse, Error> {
        let request = HttpRequest {
            method: "POST".to_owned(),
            url: url.to_owned(),
            headers: headers.to_vec(),
            body: body.to_vec(),
        };
        let mut last_status = 0;
        let mut tries = 0;
        for attempt in 0..self.attempts {
            if attempt > 0 {
                self.pause.pause(attempt);
            }
            tries += 1;
            let response = self.transport.execute(&request)?;
            if (200..300).contains(&response.status) {
                return Ok(response);
            }
            last_status = response.status;
            if !retryable(response.status) {
                break;
            }
        }
        Err(Error::Status {
            status: last_status,
            attempts: tries,
        })
    }
}

fn retryable(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Script {
        statuses: Vec<u16>,
        hits: AtomicUsize,
    }

    impl Transport for Script {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, Error> {
            assert_eq!(request.method, "POST");
            let index = self.hits.fetch_add(1, Ordering::Relaxed);
            let status = self.statuses.get(index).copied().unwrap_or(500);
            Ok(HttpResponse {
                status,
                body: b"ok".to_vec(),
            })
        }
    }

    #[test]
    fn a_429_is_retried_and_a_400_is_not() {
        let client = Client::new(Script {
            statuses: vec![429, 200],
            hits: AtomicUsize::new(0),
        });
        let response = client
            .post("http://example.test/v1/chat", &[], b"{}")
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(client.transport.hits.load(Ordering::Relaxed), 2);

        let client = Client::new(Script {
            statuses: vec![400],
            hits: AtomicUsize::new(0),
        });
        let err = client
            .post("http://example.test/v1/chat", &[], b"{}")
            .unwrap_err();
        assert_eq!(
            err,
            Error::Status {
                status: 400,
                attempts: 1
            }
        );
        assert_eq!(client.transport.hits.load(Ordering::Relaxed), 1);
    }
}
