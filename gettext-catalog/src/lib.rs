//! Gettext catalog loader.
//!
//! A `.po` file becomes a [`Catalog`]. Plural selection uses the `Plural-Forms`
//! header. `polib` reads the file; it does not evaluate the plural expression,
//! so that evaluator lives here.

mod catalog;
mod plural;

pub use catalog::{Catalog, Entry};
pub use plural::PluralRule;

/// Why a catalog could not be loaded.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The `.po` text could not be parsed.
    #[error("catalog {code} could not be parsed: {detail}")]
    Parse {
        /// Caller-supplied catalog name.
        code: String,
        /// Parser message.
        detail: String,
    },
    /// `Plural-Forms` is missing or not an expression this crate accepts.
    #[error("catalog {code} has an invalid plural rule: {detail}")]
    Plural {
        /// Caller-supplied catalog name.
        code: String,
        /// What the expression parser rejected.
        detail: String,
    },
    /// Two entries share a lookup key.
    #[error("catalog {code} repeats key {key}")]
    Duplicate {
        /// Caller-supplied catalog name.
        code: String,
        /// The repeated key.
        key: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    const FR: &str = include_str!("../fixtures/fr.po");

    #[test]
    fn french_plural_selects_the_fixture_forms() {
        let catalog = Catalog::parse("fr", FR).unwrap();
        assert_eq!(catalog.plural("file", 1), Some("fichier"));
        assert_eq!(catalog.plural("file", 0), Some("fichiers"));
        assert_eq!(catalog.plural("file", 2), Some("fichiers"));
    }

    #[test]
    fn a_bare_hash_is_an_error() {
        let err = Catalog::parse("xx", "#\n").unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }
}
