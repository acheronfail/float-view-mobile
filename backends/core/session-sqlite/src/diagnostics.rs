//! Bounded diagnostic storage, written only by the service's background worker.
use super::*;
pub const MAX_DIAGNOSTIC_ROWS: u64 = 5_000;
pub const MAX_DIAGNOSTIC_BYTES: usize = 1_024;

pub struct DiagnosticWriter(Connection);
impl DiagnosticWriter {
    pub fn open(path: &Path) -> Result<Self, WriterError> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(map_sqlite)?;
        // Diagnostics yield to telemetry rather than waiting for its writer.
        connection
            .busy_timeout(Duration::ZERO)
            .map_err(map_sqlite)?;
        Ok(Self(connection))
    }
    pub fn append(&mut self, records: &[String]) -> Result<(), WriterError> {
        if records.len() > 32
            || records.iter().any(|r| {
                r.len() > MAX_DIAGNOSTIC_BYTES
                    || serde_json::from_str::<serde_json::Value>(r).is_err()
            })
        {
            return Err(WriterError::InvalidInput);
        }
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite)?;
        for record in records {
            tx.execute(
                "INSERT INTO session_diagnostic(record_json) VALUES (?)",
                [record],
            )
            .map_err(map_sqlite)?;
        }
        tx.execute("DELETE FROM session_diagnostic WHERE sequence > 1 AND sequence <= (SELECT COALESCE(MAX(sequence),0) FROM session_diagnostic) - ?", [MAX_DIAGNOSTIC_ROWS - 1]).map_err(map_sqlite)?;
        tx.commit().map_err(map_sqlite)
    }
}

pub(crate) fn validate_diagnostics(connection: &Connection) -> Result<(), WriterError> {
    let invalid: bool = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM (SELECT 1 FROM session_diagnostic LIMIT 5001)) > 5000 OR EXISTS(SELECT 1 FROM session_diagnostic WHERE length(CAST(record_json AS BLOB)) > 1024 OR NOT json_valid(record_json))", [], |r| r.get(0),
    ).map_err(map_validation_sqlite)?;
    if invalid {
        Err(WriterError::Incompatible)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caps_rows_preserves_metadata_and_rejects_bad_records_atomically() {
        let dir = std::env::temp_dir().join(format!(
            "float-view-diagnostic-store-{}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("diagnostics.sqlite3");
        SessionWriter::create(
            &path,
            SessionMetadata {
                expected_riding_persistence_ms: 100,
                expected_idle_persistence_ms: 1000,
                committed_at_epoch_ms: 1,
            },
        )
        .unwrap()
        .close()
        .unwrap();
        let mut writer = DiagnosticWriter::open(&path).unwrap();
        writer.append(&[r#"{"event":"metadata"}"#.into()]).unwrap();
        for n in 0..160 {
            writer
                .append(
                    &(0..32)
                        .map(|i| format!("{{\"number\":{}}}", n * 32 + i))
                        .collect::<Vec<_>>(),
                )
                .unwrap();
        }
        let db = Connection::open(&path).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM session_diagnostic", [], |r| r
                .get::<_, u64>(0))
                .unwrap(),
            MAX_DIAGNOSTIC_ROWS
        );
        assert_eq!(
            db.query_row(
                "SELECT record_json FROM session_diagnostic WHERE sequence=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            r#"{"event":"metadata"}"#
        );
        assert_eq!(
            writer.append(&["not JSON".into()]),
            Err(WriterError::InvalidInput)
        );
        assert_eq!(
            writer.append(&[format!("\"{}\"", "x".repeat(1024))]),
            Err(WriterError::InvalidInput)
        );
        validate_diagnostics(&db).unwrap();
        // Diagnostic contention must not wait behind a recording transaction.
        db.execute_batch("BEGIN IMMEDIATE").unwrap();
        assert_eq!(writer.append(&["{}".into()]), Err(WriterError::Busy));
        db.execute_batch("ROLLBACK").unwrap();
        writer.append(&["{}".into()]).unwrap();
        drop(writer);
        drop(db);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
