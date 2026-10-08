//! One `.po` catalog: entries by key and the compiled `Plural-Forms` rule.

use std::collections::HashMap;

use polib::po_file::parse_from_reader;

use crate::plural::{PluralError, PluralRule};
use crate::Error;

/// One message of a catalog.
#[derive(Debug, Clone)]
pub struct Entry {
    /// `msgctxt` when set, otherwise `msgid`.
    pub key: String,
    /// English source text.
    pub msgid: String,
    /// English plural source, for plural messages.
    pub msgid_plural: Option<String>,
    /// `msgstr`, or `msgstr[0..]` for a plural message.
    pub forms: Vec<String>,
    /// Marked `#, fuzzy`. Gettext does not show a fuzzy translation.
    pub fuzzy: bool,
}

impl Entry {
    /// Translated forms, or `None` when the entry must fall back.
    #[must_use]
    pub fn translated(&self, nplurals: usize) -> Option<&[String]> {
        let want = if self.msgid_plural.is_some() {
            nplurals
        } else {
            1
        };
        let ok =
            !self.fuzzy && self.forms.len() == want && self.forms.iter().all(|f| !f.is_empty());
        ok.then_some(self.forms.as_slice())
    }
}

/// A parsed catalog.
#[derive(Debug, Clone)]
pub struct Catalog {
    rule: PluralRule,
    entries: Vec<Entry>,
    by_key: HashMap<String, usize>,
}

impl Catalog {
    /// Parses `source`, the text of a `.po` file. `code` names it in errors.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when the file is not a catalog.
    /// [`Error::Plural`] when `Plural-Forms` is not a gettext expression.
    /// [`Error::Duplicate`] when two entries share a key.
    pub fn parse(code: &str, source: &str) -> Result<Self, Error> {
        // polib panics on a bare `#` before any field has been read.
        if source
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim_end)
            == Some("#")
        {
            return Err(Error::Parse {
                code: code.to_owned(),
                detail: "the file starts with a bare `#` line".to_owned(),
            });
        }
        let po = parse_from_reader(source.as_bytes()).map_err(|err| Error::Parse {
            code: code.to_owned(),
            detail: err.to_string(),
        })?;
        let rules = &po.metadata.plural_rules;
        let rule =
            PluralRule::new(rules.nplurals, &rules.expr).map_err(|PluralError(detail)| {
                Error::Plural {
                    code: code.to_owned(),
                    detail,
                }
            })?;
        let mut entries = Vec::new();
        let mut by_key = HashMap::new();
        for message in po.messages() {
            let key = message
                .msgctxt()
                .unwrap_or_else(|| message.msgid())
                .to_owned();
            let forms = match (message.msgstr(), message.msgstr_plural()) {
                (Ok(text), _) => vec![text.to_owned()],
                (_, Ok(plural)) => plural.clone(),
                _ => Vec::new(),
            };
            let entry = Entry {
                key: key.clone(),
                msgid: message.msgid().to_owned(),
                msgid_plural: message.msgid_plural().ok().map(str::to_owned),
                forms,
                fuzzy: message.is_fuzzy(),
            };
            if by_key.insert(key.clone(), entries.len()).is_some() {
                return Err(Error::Duplicate {
                    code: code.to_owned(),
                    key,
                });
            }
            entries.push(entry);
        }
        Ok(Self {
            rule,
            entries,
            by_key,
        })
    }

    /// The entry for `key`, if the catalog has one.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.by_key.get(key).map(|&index| &self.entries[index])
    }

    /// The plural form of `msgid` for count `n`.
    #[must_use]
    pub fn plural<'a>(&'a self, msgid: &str, n: u64) -> Option<&'a str> {
        let entry = self.get(msgid)?;
        let forms = entry.translated(self.rule.nplurals())?;
        let index = self.rule.index(n);
        forms.get(index).map(String::as_str)
    }
}
