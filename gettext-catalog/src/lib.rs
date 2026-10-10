//! gettext catalogs (`.po`) for an application's own strings: parse them,
//! evaluate their `Plural-Forms` rules, and fill named `{placeholders}`.
//!
//! Pure Rust: catalogs are parsed with `polib` and `Plural-Forms` rules
//! evaluated by [`plural`], so there is no libintl to install. Extracted from
//! cox's `cox-i18n`.

pub mod catalog;
pub mod format;
pub mod plural;

/// Why a catalog, a plural rule or a placeholder was rejected.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// polib could not read the catalog.
    #[error("locale `{code}`: {detail}")]
    Parse {
        /// The locale being parsed.
        code: String,
        /// polib's message.
        detail: String,
    },
    /// The catalog's `Plural-Forms` header is outside the gettext grammar.
    #[error("locale `{code}`: bad Plural-Forms: {detail}")]
    PluralForms {
        /// The locale being parsed.
        code: String,
        /// What is wrong with the rule.
        detail: String,
    },
    /// A `Plural-Forms` expression is outside the gettext grammar.
    #[error("{0}")]
    PluralExpr(String),
    /// Two entries share a key.
    #[error("locale `{code}`: message `{key}` is defined twice")]
    Duplicate {
        /// The locale being parsed.
        code: String,
        /// The repeated `msgctxt` or `msgid`.
        key: String,
    },
    /// A brace that is neither doubled nor part of a `{name}` placeholder.
    #[error("{0}")]
    Placeholder(String),
}
