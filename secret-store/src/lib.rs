//! Environment, then a credential store.
//!
//! A non-empty environment value wins and the store is not asked. The store
//! is a trait so tests use [`MemoryBackend`] and never open the platform
//! store. The platform adapter is the `os` feature; constructing it is what
//! binds the process to the real store, so the default test build leaves the
//! feature off.

mod memory;

#[cfg(feature = "os")]
mod os;

pub use memory::MemoryBackend;

#[cfg(feature = "os")]
pub use os::OsBackend;

use std::fmt;

/// Why a secret could not be resolved.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The injected store failed. The message never includes the secret.
    #[error("credential store failed: {0}")]
    Backend(String),
}

/// A secret that debug and display output refuse to print.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    /// Wraps `value`. Callers pass a value they already hold; this type does not invent one.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The secret text, for a caller that is about to use it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        let mut bytes = std::mem::take(&mut self.0).into_bytes();
        bytes.fill(0);
    }
}

/// Where a secret lives after the environment has been checked.
pub trait SecretBackend {
    /// The stored secret, or `None` when this service and account have no entry.
    ///
    /// # Errors
    ///
    /// [`Error::Backend`] when the store cannot answer.
    fn get(&self, service: &str, account: &str) -> Result<Option<Secret>, Error>;

    /// Replaces the stored secret.
    ///
    /// # Errors
    ///
    /// [`Error::Backend`] when the store cannot write.
    fn set(&self, service: &str, account: &str, secret: &Secret) -> Result<(), Error>;

    /// Removes the stored secret. Missing entries succeed.
    ///
    /// # Errors
    ///
    /// [`Error::Backend`] when the store cannot delete.
    fn delete(&self, service: &str, account: &str) -> Result<(), Error>;
}

/// Environment first, then `B`.
#[derive(Debug)]
pub struct Store<B> {
    backend: B,
}

impl<B> Store<B> {
    /// A resolver that asks `backend` only when the environment value is empty.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// The injected store, for tests that count calls.
    #[must_use]
    pub fn backend(&self) -> &B {
        &self.backend
    }
}

impl<B: SecretBackend> Store<B> {
    /// `env_value` when it is non-empty after trimming, otherwise the store.
    ///
    /// # Errors
    ///
    /// [`Error::Backend`] when the store is consulted and fails.
    pub fn resolve(
        &self,
        env_value: Option<&str>,
        service: &str,
        account: &str,
    ) -> Result<Option<Secret>, Error> {
        if let Some(value) = env_value.map(str::trim).filter(|value| !value.is_empty()) {
            return Ok(Some(Secret::new(value)));
        }
        self.backend.get(service, account)
    }
}

/// In-memory store for tests and for callers that have no platform store.
#[must_use]
pub fn memory() -> Store<MemoryBackend> {
    Store::new(MemoryBackend::new())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    struct Probe {
        hits: AtomicUsize,
        value: Option<String>,
    }

    impl SecretBackend for Probe {
        fn get(&self, _service: &str, _account: &str) -> Result<Option<Secret>, Error> {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Ok(self.value.clone().map(Secret::new))
        }

        fn set(&self, _: &str, _: &str, _: &Secret) -> Result<(), Error> {
            Ok(())
        }

        fn delete(&self, _: &str, _: &str) -> Result<(), Error> {
            Ok(())
        }
    }

    #[test]
    fn env_wins_and_the_store_is_not_opened() {
        let store = Store::new(Probe {
            hits: AtomicUsize::new(0),
            value: Some("from-store".into()),
        });
        let secret = store
            .resolve(Some("  from-env  "), "mail", "account")
            .unwrap()
            .unwrap();
        assert_eq!(secret.expose(), "from-env");
        assert_eq!(store.backend().hits.load(Ordering::Relaxed), 0);
        assert_eq!(format!("{secret:?}"), "Secret([REDACTED])");
        assert!(!format!("{secret:?}").contains("from-env"));
        assert_eq!(secret.to_string(), "[REDACTED]");
    }

    #[test]
    fn empty_env_falls_through_to_the_memory_store() {
        let store = memory();
        store
            .backend()
            .set("mail", "account", &Secret::new("fixture-value"))
            .unwrap();
        let secret = store
            .resolve(Some("   "), "mail", "account")
            .unwrap()
            .unwrap();
        assert_eq!(secret.expose(), "fixture-value");
        store.backend().delete("mail", "account").unwrap();
        assert!(store.resolve(None, "mail", "account").unwrap().is_none());
    }

    #[test]
    fn memory_helper_is_not_the_platform_store() {
        let name = std::any::type_name::<MemoryBackend>();
        assert!(name.contains("MemoryBackend"));
        assert!(!name.contains("OsBackend"));
    }
}
