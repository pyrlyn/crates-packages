//! Layered TOML configuration.
//!
//! Files and an environment layer merge in order. A later layer replaces a
//! leaf and deep-merges a table. A missing file is skipped. Each leaf records
//! the layer that set it.
//!
//! Figment 0.10.19 leaves its TOML provider compiled out (`feature = "toml"`
//! is commented out of that release), so this crate merges `toml` values
//! itself instead of depending on a provider that is not there.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use toml::map::Map;
use toml::Value;

/// Why a layered load failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The layer's text is not a TOML table.
    #[error("config layer {layer} is not valid TOML: {detail}")]
    Parse {
        /// Layer name the caller passed.
        layer: String,
        /// Parser message, with no file body attached.
        detail: String,
    },
    /// The layer file could not be read.
    #[error("config layer {layer} could not be read: {detail}")]
    Read {
        /// Layer name the caller passed.
        layer: String,
        /// Filesystem message.
        detail: String,
    },
    /// The merged document does not match `T`.
    #[error("config does not match the expected type: {0}")]
    Shape(String),
}

/// Ordered TOML layers and the name of the layer that last set each leaf.
#[derive(Debug, Default)]
pub struct Loader {
    root: Map<String, Value>,
    sources: HashMap<String, String>,
}

impl Loader {
    /// An empty document. Extracting it fails unless `T` can be built from no fields.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses `text` as a TOML table and merges it under `layer`.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `text` is not a TOML table.
    pub fn merge_toml(&mut self, layer: &str, text: &str) -> Result<(), Error> {
        let value: Value = toml::from_str(text).map_err(|err| Error::Parse {
            layer: layer.to_owned(),
            detail: err.to_string(),
        })?;
        let Value::Table(table) = value else {
            return Err(Error::Parse {
                layer: layer.to_owned(),
                detail: "the document is not a table".to_owned(),
            });
        };
        merge_table(&mut self.root, table, "", layer, &mut self.sources);
        Ok(())
    }

    /// Merges the TOML file at `path` when it exists.
    ///
    /// A missing file leaves the loader unchanged.
    ///
    /// # Errors
    ///
    /// [`Error::Read`] when the file exists but cannot be read.
    /// [`Error::Parse`] when the file is not a TOML table.
    pub fn merge_file(&mut self, layer: &str, path: &Path) -> Result<(), Error> {
        if !path.exists() {
            return Ok(());
        }
        let text = fs::read_to_string(path).map_err(|err| Error::Read {
            layer: layer.to_owned(),
            detail: err.to_string(),
        })?;
        self.merge_toml(layer, &text)
    }

    /// Merges `PREFIX_KEY` entries from `vars`.
    ///
    /// `__` in the remainder becomes a nested key (`APP_SERVER__PORT` sets
    /// `server.port`). An empty value does not override a previous layer.
    /// Integers and booleans are typed; anything else stays a string.
    ///
    /// # Errors
    ///
    /// This layer does not parse a document, so it currently always returns `Ok`.
    pub fn merge_env(
        &mut self,
        layer: &str,
        prefix: &str,
        vars: impl IntoIterator<Item = (String, String)>,
    ) -> Result<(), Error> {
        for (key, value) in vars {
            if value.is_empty() {
                continue;
            }
            let Some(path) = env_path(&key, prefix) else {
                continue;
            };
            let parts: Vec<&str> = path.split('.').collect();
            insert_leaf(&mut self.root, &parts, leaf_from_env(&value));
            forget_under(&mut self.sources, &path);
            self.sources.insert(path, layer.to_owned());
        }
        Ok(())
    }

    /// [`merge_env`] reading the process environment.
    ///
    /// # Errors
    ///
    /// Same as [`merge_env`].
    pub fn merge_process_env(&mut self, layer: &str, prefix: &str) -> Result<(), Error> {
        self.merge_env(layer, prefix, std::env::vars())
    }

    /// Deserializes the merged document into `T`.
    ///
    /// # Errors
    ///
    /// [`Error::Shape`] when `T` does not match the document.
    pub fn extract<T: DeserializeOwned>(&self) -> Result<T, Error> {
        T::deserialize(Value::Table(self.root.clone())).map_err(|err| Error::Shape(err.to_string()))
    }

    /// The layer that last set `key`, using dotted names (`server.port`).
    #[must_use]
    pub fn source_of(&self, key: &str) -> Option<&str> {
        self.sources.get(key).map(String::as_str)
    }
}

fn env_path(key: &str, prefix: &str) -> Option<String> {
    let rest = key.strip_prefix(prefix)?;
    if rest.is_empty() {
        return None;
    }
    let path = rest
        .split("__")
        .map(str::to_ascii_lowercase)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(".");
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn leaf_from_env(raw: &str) -> Value {
    if let Ok(number) = raw.parse::<i64>() {
        return Value::Integer(number);
    }
    match raw {
        "true" => Value::Boolean(true),
        "false" => Value::Boolean(false),
        _ => Value::String(raw.to_owned()),
    }
}

fn insert_leaf(table: &mut Map<String, Value>, path: &[&str], leaf: Value) {
    let Some((head, tail)) = path.split_first() else {
        return;
    };
    if tail.is_empty() {
        table.insert((*head).to_owned(), leaf);
        return;
    }
    let entry = table
        .entry((*head).to_owned())
        .or_insert_with(|| Value::Table(Map::new()));
    if !matches!(entry, Value::Table(_)) {
        *entry = Value::Table(Map::new());
    }
    if let Value::Table(next) = entry {
        insert_leaf(next, tail, leaf);
    }
}

fn merge_table(
    dst: &mut Map<String, Value>,
    src: Map<String, Value>,
    prefix: &str,
    layer: &str,
    sources: &mut HashMap<String, String>,
) {
    for (key, value) in src {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Table(child) => {
                let slot = dst.entry(key).or_insert_with(|| Value::Table(Map::new()));
                if !matches!(slot, Value::Table(_)) {
                    *slot = Value::Table(Map::new());
                    sources.remove(&path);
                }
                if let Value::Table(existing) = slot {
                    merge_table(existing, child, &path, layer, sources);
                }
            }
            other => {
                dst.insert(key, other);
                forget_under(sources, &path);
                sources.insert(path, layer.to_owned());
            }
        }
    }
}

fn forget_under(sources: &mut HashMap<String, String>, path: &str) {
    let nested = format!("{path}.");
    sources.retain(|key, _| key != path && !key.starts_with(&nested));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct App {
        name: String,
        port: u16,
        server: Server,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Server {
        host: String,
    }

    fn sample() -> Loader {
        let mut loader = Loader::new();
        loader
            .merge_toml(
                "default",
                "name = \"default\"\nport = 1\n\n[server]\nhost = \"localhost\"\n",
            )
            .unwrap();
        loader
    }

    #[test]
    fn later_file_overrides_a_leaf_and_keeps_siblings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("user.toml");
        fs::write(&path, "name = \"user\"\n\n[server]\nhost = \"db\"\n").unwrap();
        let mut loader = sample();
        loader.merge_file("user", &path).unwrap();
        loader
            .merge_file("missing", &dir.path().join("nope.toml"))
            .unwrap();
        let app = loader.extract::<App>().unwrap();
        assert_eq!(
            app,
            App {
                name: "user".into(),
                port: 1,
                server: Server { host: "db".into() },
            }
        );
        assert_eq!(loader.source_of("name"), Some("user"));
        assert_eq!(loader.source_of("port"), Some("default"));
        assert_eq!(loader.source_of("server.host"), Some("user"));
    }

    #[test]
    fn env_overrides_the_file_and_types_integers() {
        let mut loader = sample();
        loader
            .merge_env(
                "env",
                "APP_",
                [
                    ("APP_NAME".into(), "from-env".into()),
                    ("APP_PORT".into(), "9".into()),
                    ("APP_SERVER__HOST".into(), "env-host".into()),
                    ("OTHER".into(), "ignored".into()),
                    ("APP_NAME".into(), String::new()),
                ],
            )
            .unwrap();
        let app = loader.extract::<App>().unwrap();
        assert_eq!(app.name, "from-env");
        assert_eq!(app.port, 9);
        assert_eq!(app.server.host, "env-host");
        assert_eq!(loader.source_of("port"), Some("env"));
    }

    #[test]
    fn a_scalar_drops_nested_provenance() {
        let mut loader = sample();
        loader.merge_toml("flag", "server = \"down\"\n").unwrap();
        assert_eq!(loader.source_of("server"), Some("flag"));
        assert_eq!(loader.source_of("server.host"), None);
    }

    #[test]
    fn invalid_toml_names_the_layer() {
        let mut loader = Loader::new();
        let err = loader.merge_toml("project", "name = [").unwrap_err();
        assert!(matches!(err, Error::Parse { layer, .. } if layer == "project"));
    }
}
