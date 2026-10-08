//! Process-local secret map. Tests use this so the platform store stays closed.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::{Error, Secret, SecretBackend};

type Guard<'a> = std::sync::MutexGuard<'a, HashMap<(String, String), String>>;

/// Secrets kept in a mutex. A poisoned lock becomes [`Error::Backend`] and drops the secret text.
#[derive(Debug, Default)]
pub struct MemoryBackend {
    entries: Mutex<HashMap<(String, String), String>>,
}

impl MemoryBackend {
    /// An empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<Guard<'_>, Error> {
        self.entries
            .lock()
            .map_err(|_| Error::Backend("memory store lock failed".to_owned()))
    }
}

impl SecretBackend for MemoryBackend {
    fn get(&self, service: &str, account: &str) -> Result<Option<Secret>, Error> {
        let entries = self.lock()?;
        Ok(entries
            .get(&(service.to_owned(), account.to_owned()))
            .cloned()
            .map(Secret::new))
    }

    fn set(&self, service: &str, account: &str, secret: &Secret) -> Result<(), Error> {
        self.lock()?.insert(
            (service.to_owned(), account.to_owned()),
            secret.expose().to_owned(),
        );
        Ok(())
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), Error> {
        self.lock()?
            .remove(&(service.to_owned(), account.to_owned()));
        Ok(())
    }
}
