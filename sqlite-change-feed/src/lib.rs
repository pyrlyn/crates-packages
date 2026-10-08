//! A pull-based SQLite change feed.
//!
//! `PRAGMA data_version` changes when another connection commits and stays
//! put for this connection's own commits. The caller polls; there is no
//! thread and no timer.

use std::path::Path;

use diesel::prelude::*;
use diesel::sql_types::BigInt;

/// Why a feed operation failed. The message does not include SQL text.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The path could not be passed to SQLite.
    #[error("sqlite path is not UTF-8")]
    Path,
    /// The file could not be opened.
    #[error("sqlite connection failed")]
    Connect,
    /// A statement failed.
    #[error("sqlite query failed")]
    Query,
}

/// A position in one connection's change feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeToken(i64);

/// One SQLite connection and its `data_version` cursor.
pub struct ChangeFeed {
    conn: SqliteConnection,
}

#[derive(QueryableByName)]
struct DataVersion {
    #[diesel(sql_type = BigInt)]
    data_version: i64,
}

impl ChangeFeed {
    /// Opens `path`, creating the file when it is missing.
    ///
    /// # Errors
    ///
    /// [`Error::Path`] when the path is not UTF-8.
    /// [`Error::Connect`] when SQLite cannot open it.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let text = path.to_str().ok_or(Error::Path)?;
        let conn = SqliteConnection::establish(text).map_err(|_| Error::Connect)?;
        Ok(Self { conn })
    }

    /// The feed's current position. The next [`ChangeFeed::changed`] reports
    /// only commits after this call.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the pragma cannot be read.
    pub fn token(&mut self) -> Result<ChangeToken, Error> {
        self.data_version().map(ChangeToken)
    }

    /// Whether another connection committed since `since`, and advances it.
    ///
    /// This connection's own commits do not report `true`.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when the pragma cannot be read.
    pub fn changed(&mut self, since: &mut ChangeToken) -> Result<bool, Error> {
        let now = self.data_version()?;
        let changed = now != since.0;
        since.0 = now;
        Ok(changed)
    }

    /// Runs `sql` on this connection. Tests use it to commit from a second connection.
    ///
    /// # Errors
    ///
    /// [`Error::Query`] when SQLite rejects the statement.
    pub fn execute(&mut self, sql: &str) -> Result<(), Error> {
        // Diesel's query builder cannot express arbitrary DDL the tests use to commit.
        diesel::sql_query(sql)
            .execute(&mut self.conn)
            .map(|_| ())
            .map_err(|_| Error::Query)
    }

    fn data_version(&mut self) -> Result<i64, Error> {
        // Diesel's query builder cannot express PRAGMA.
        diesel::sql_query("PRAGMA data_version")
            .get_result::<DataVersion>(&mut self.conn)
            .map(|row| row.data_version)
            .map_err(|_| Error::Query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn another_connection_is_visible_and_our_own_write_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("feed.db");
        let mut writer = ChangeFeed::open(&path).unwrap();
        let mut reader = ChangeFeed::open(&path).unwrap();
        writer
            .execute("create table probe (id integer primary key)")
            .unwrap();
        let mut token = reader.token().unwrap();
        assert!(!reader.changed(&mut token).unwrap());

        writer.execute("insert into probe (id) values (1)").unwrap();
        assert!(reader.changed(&mut token).unwrap());
        assert!(!reader.changed(&mut token).unwrap());

        reader.execute("insert into probe (id) values (2)").unwrap();
        assert!(!reader.changed(&mut token).unwrap());
    }
}
