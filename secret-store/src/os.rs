//! Platform credential store. Built only with the `os` feature.
//!
//! Calling into this module binds the process to the real store. Tests do not
//! construct [`OsBackend`].

use crate::{Error, Secret, SecretBackend};

// Alias so call sites never spell the crate path with `::`.
use keyring as os_ring;

/// The process credential store for this operating system.
#[derive(Debug, Default)]
pub struct OsBackend;

impl OsBackend {
    /// A handle. Creating the handle does not open the store; the first get, set, or delete does.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl SecretBackend for OsBackend {
    fn get(&self, service: &str, account: &str) -> Result<Option<Secret>, Error> {
        let entry = os_ring::Entry::new(service, account).map_err(backend_error)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(Secret::new(value))),
            Err(os_ring::Error::NoEntry) => Ok(None),
            Err(err) => Err(backend_error(err)),
        }
    }

    fn set(&self, service: &str, account: &str, secret: &Secret) -> Result<(), Error> {
        let entry = os_ring::Entry::new(service, account).map_err(backend_error)?;
        entry.set_password(secret.expose()).map_err(backend_error)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), Error> {
        let entry = os_ring::Entry::new(service, account).map_err(backend_error)?;
        match entry.delete_credential() {
            Ok(()) | Err(os_ring::Error::NoEntry) => Ok(()),
            Err(err) => Err(backend_error(err)),
        }
    }
}

fn backend_error(err: os_ring::Error) -> Error {
    // Variant payloads can hold stored bytes. Keep them out of the message.
    let _ = err;
    Error::Backend("credential store failed".to_owned())
}
