//! Bundled `SQLite` identity and canonical lifecycle smoke.

use std::path::Path;

use rusqlite::{Connection, Result};

mod archive;
mod catalog;
mod history;
mod prune;
mod reader;
mod session_data;
mod session_store;

pub use archive::*;
pub use catalog::*;
pub use history::*;
pub use prune::*;
pub use reader::*;
pub use session_data::*;
pub use session_store::*;

pub const CATALOG_SQL: &str = include_str!("../../../../sql/catalog.sql");
pub const SESSION_SQL: &str = include_str!("../../../../sql/session.sql");
pub const STORAGE_VERSION: &str = "12";

#[derive(Debug, Eq, PartialEq)]
pub struct EngineIdentity {
    pub version: String,
    pub source_id: String,
    pub compile_options: Vec<String>,
}

/// Reads the exact `SQLite` engine identity and sorted compile-option set.
///
/// # Errors
/// Returns the originating `SQLite` query error.
pub fn engine_identity(connection: &Connection) -> Result<EngineIdentity> {
    let version = connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))?;
    let source_id = connection.query_row("SELECT sqlite_source_id()", [], |row| row.get(0))?;
    let mut statement = connection.prepare("SELECT compile_options FROM pragma_compile_options")?;
    let mut compile_options = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>>>()?;
    compile_options.sort();
    Ok(EngineIdentity {
        version,
        source_id,
        compile_options,
    })
}

/// Opens the canonical catalog with the Cargo-bundled `SQLite` engine.
///
/// # Errors
/// Returns schema or `SQLite` open errors.
pub fn open_catalog(path: &Path) -> Result<Connection> {
    let connection = Connection::open(path)?;
    connection.execute_batch(CATALOG_SQL)?;
    Ok(connection)
}

/// Opens a new canonical session database.
///
/// # Errors
/// Returns schema or `SQLite` open errors.
pub fn open_session(path: &Path) -> Result<Connection> {
    let connection = Connection::open(path)?;
    connection.execute_batch(SESSION_SQL)?;
    Ok(connection)
}

#[cfg(test)]
pub(crate) struct TestRoot(tempfile::TempDir);

#[cfg(test)]
impl TestRoot {
    pub(crate) fn new(prefix: &str) -> Self {
        Self(
            tempfile::Builder::new()
                .prefix(prefix)
                .tempdir()
                .expect("temporary directory"),
        )
    }
}

#[cfg(test)]
impl std::ops::Deref for TestRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.path()
    }
}

#[cfg(test)]
impl AsRef<Path> for TestRoot {
    fn as_ref(&self) -> &Path {
        self.0.path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn bundled_engine_runs_canonical_wal_lifecycle() {
        let root = TestRoot::new("float-view-catalog-");
        let path = root.join("catalog.sqlite");
        let connection = open_catalog(&path).unwrap();
        let identity = engine_identity(&connection).unwrap();
        assert!(!identity.version.is_empty());
        assert!(identity.source_id.len() > 20);
        assert!(
            identity
                .compile_options
                .iter()
                .any(|option| option == "THREADSAFE=1")
        );
        assert_eq!(
            connection
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "wal"
        );
        connection
            .execute(
                "INSERT INTO saved_device(saved_device_id, device_history_id, created_at_ms) VALUES ('saved', 'history', 1)",
                [],
            )
            .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM saved_device", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        connection.close().unwrap();

        let reopened = open_catalog(&path).unwrap();
        assert_eq!(
            reopened
                .query_row("SELECT count(*) FROM saved_device", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        reopened.close().unwrap();
    }

    #[test]
    fn interrupt_cancels_bounded_reader() {
        let connection = Connection::open_in_memory().unwrap();
        let interrupt = connection.get_interrupt_handle();
        let (started, ready) = mpsc::channel();
        let worker = thread::spawn(move || {
            started.send(()).unwrap();
            connection.query_row(
                "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x + 1 FROM n WHERE x < 100000000) SELECT sum(x) FROM n",
                [],
                |row| row.get::<_, i64>(0),
            )
        });
        ready.recv().unwrap();
        thread::sleep(Duration::from_millis(5));
        interrupt.interrupt();
        assert!(worker.join().unwrap().is_err());
    }
}
