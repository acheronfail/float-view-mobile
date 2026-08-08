#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{CatalogStore, SeriesPoint, StoreError, reader::stable_column, select_prune_rows};

pub const MAX_PRUNE_SOURCE_ROWS: usize = 200_000;
pub const INCREMENTAL_VACUUM_PAGES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PruneResult {
    pub source_samples: u64,
    pub retained_samples: u64,
    pub removed_samples: u64,
    pub history_revision: u64,
}

impl CatalogStore {
    pub fn prune_session(
        &self,
        session_id: &str,
        target_resolution_seconds: u64,
        fields: &[String],
        pruned_at_millis: i64,
    ) -> Result<PruneResult, StoreError> {
        if !(1..=300).contains(&target_resolution_seconds)
            || fields.is_empty()
            || fields.len() > crate::MAX_SERIES_FIELDS
            || pruned_at_millis < 0
        {
            return Err(StoreError::Invalid);
        }
        let columns = fields
            .iter()
            .map(|field| stable_column(field).ok_or(StoreError::Invalid))
            .collect::<Result<Vec<_>, _>>()?;
        let catalog = self.writer.lock().map_err(|_| StoreError::Poisoned)?;
        if self
            .read_leases
            .lock()
            .map_err(|_| StoreError::Poisoned)?
            .get(session_id)
            .copied()
            .unwrap_or(0)
            != 0
        {
            return Err(StoreError::Conflict {
                current_revision: 0,
            });
        }
        let file_name: String = catalog
            .query_row(
                "SELECT file_name FROM session_catalog WHERE session_id = ?1 AND pending_delete = 0 AND state IN ('completed', 'interrupted', 'partial')",
                [session_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        if file_name.contains('/') || file_name.contains('\\') || !file_name.ends_with(".sqlite") {
            return Err(StoreError::Invalid);
        }
        let mut session = Connection::open(self.root.join(file_name))?;
        session.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 1000;")?;
        let selected = columns
            .iter()
            .enumerate()
            .map(|(index, column)| format!("{column} AS value_{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT sequence, monotonic_elapsed_ms, ride_state, {selected} FROM sample ORDER BY monotonic_elapsed_ms, sequence LIMIT ?1"
        );
        let rows = {
            let mut statement = session.prepare(&sql)?;
            statement
                .query_map([MAX_PRUNE_SOURCE_ROWS + 1], |row| {
                    let mut values = HashMap::with_capacity(fields.len());
                    for (index, field) in fields.iter().enumerate() {
                        values.insert(field.clone(), row.get::<_, Option<f64>>(index + 3)?);
                    }
                    Ok(SeriesPoint {
                        sequence: row.get(0)?,
                        elapsed_millis: row.get(1)?,
                        ride_state: row.get(2)?,
                        values,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        if rows.len() > MAX_PRUNE_SOURCE_ROWS {
            return Err(StoreError::Invalid);
        }
        let event_sources = {
            let mut statement = session.prepare(
                "SELECT DISTINCT source_sample_sequence FROM session_event WHERE source_sample_sequence IS NOT NULL",
            )?;
            statement
                .query_map([], |row| row.get(0))?
                .collect::<Result<HashSet<u64>, _>>()?
        };
        let mut protected = event_sources.clone();
        {
            let mut statement =
                session.prepare("SELECT first_sequence FROM session_numeric_field")?;
            protected.extend(
                statement
                    .query_map([], |row| row.get(0))?
                    .collect::<Result<HashSet<u64>, _>>()?,
            );
        }
        protect_riding_boundaries(&session, &mut protected)?;
        let retained = select_prune_rows(
            &rows,
            fields,
            target_resolution_seconds,
            &protected,
            &event_sources,
        )?;
        let source_samples = u64::try_from(rows.len()).map_err(|_| StoreError::Invalid)?;
        let retained_samples = u64::try_from(retained.len()).map_err(|_| StoreError::Invalid)?;
        let transaction = session.transaction()?;
        transaction.execute_batch(
            "CREATE TEMP TABLE prune_keep(sequence INTEGER PRIMARY KEY NOT NULL);",
        )?;
        {
            let mut insert = transaction.prepare("INSERT INTO prune_keep(sequence) VALUES (?1)")?;
            for sequence in &retained {
                insert.execute([sequence])?;
            }
        }
        transaction.execute(
            "DELETE FROM sample WHERE sequence NOT IN (SELECT sequence FROM prune_keep)",
            [],
        )?;
        transaction.execute(
            "DELETE FROM session_numeric_field WHERE first_sequence NOT IN (SELECT sequence FROM sample)",
            [],
        )?;
        transaction.execute(
            "UPDATE session_metadata SET pruned_at_ms = ?1, retained_resolution_seconds = ?2, prune_policy = 'deterministic_bucket_evidence_v1' WHERE singleton = 1",
            params![pruned_at_millis, target_resolution_seconds],
        )?;
        transaction.execute(
            "UPDATE commit_marker SET stored_sample_count = ?1, history_revision = history_revision + 1, committed_at_ms = ?2 WHERE singleton = 1",
            params![retained_samples, pruned_at_millis],
        )?;
        transaction.execute_batch("DROP TABLE prune_keep;")?;
        let history_revision: u64 = transaction.query_row(
            "SELECT history_revision FROM commit_marker WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        transaction.commit()?;
        session.execute_batch(&format!(
            "PRAGMA wal_checkpoint(PASSIVE); PRAGMA incremental_vacuum({INCREMENTAL_VACUUM_PAGES});"
        ))?;
        catalog.execute(
            "UPDATE session_catalog SET stored_sample_count = ?1, history_revision = ?2, integrity = 'complete' WHERE session_id = ?3",
            params![retained_samples, history_revision, session_id],
        )?;
        Ok(PruneResult {
            source_samples,
            retained_samples,
            removed_samples: source_samples.saturating_sub(retained_samples),
            history_revision,
        })
    }
}

fn protect_riding_boundaries(
    session: &Connection,
    protected: &mut HashSet<u64>,
) -> Result<(), StoreError> {
    let mut statement = session.prepare(
        "SELECT started_elapsed_ms, ended_elapsed_ms FROM riding_interval ORDER BY interval_id",
    )?;
    let intervals = statement
        .query_map([], |row| {
            Ok((row.get::<_, u64>(0)?, row.get::<_, Option<u64>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (start, end) in intervals {
        for elapsed in [Some(start), end].into_iter().flatten() {
            if let Some(sequence) = session
                .query_row(
                    "SELECT sequence FROM sample ORDER BY abs(monotonic_elapsed_ms - ?1), sequence LIMIT 1",
                    [elapsed],
                    |row| row.get(0),
                )
                .optional()?
            {
                protected.insert(sequence);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{NewSession, SessionSample, SessionStore};

    use super::*;

    fn root() -> crate::TestRoot {
        crate::TestRoot::new("float-view-prune-")
    }

    #[test]
    fn prune_is_lease_exclusive_atomic_and_updates_published_revision() {
        let root = root();
        let sessions = SessionStore::new(CatalogStore::open(&root).expect("catalog"));
        sessions
            .create(&NewSession {
                session_id: "session",
                job_id: "job",
                device_history_id: "history",
                device_snapshot_json: "{}",
                profile_snapshot_json: "{}",
                profile_revision: 0,
                started_at_millis: 0,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .expect("create");
        let samples: Vec<_> = (1_u64..=20)
            .map(|sequence| SessionSample {
                sequence,
                observed_at_millis: i64::try_from(sequence * 100).expect("time"),
                slow_observed_at_millis: None,
                monotonic_elapsed_millis: sequence * 100,
                ride_state: "idle".into(),
                package_state: Some("ready".into()),
                package_mode: Some("normal".into()),
                raw_pack_voltage_v: Some(if sequence == 10 { 60.0 } else { 80.0 }),
                duty_cycle: Some(if sequence == 11 { 0.9 } else { 0.1 }),
                speed_kph: Some(5.0),
                extension_json: "{}".into(),
            })
            .collect();
        sessions
            .append_samples("session", &samples, 2_000)
            .expect("append");
        sessions.close("session", 2_100, "done").expect("close");
        let lease = sessions
            .catalog()
            .acquire_read_lease("session")
            .expect("lease");
        assert!(matches!(
            sessions.catalog().prune_session(
                "session",
                1,
                &["packVoltageV".into(), "dutyCycle".into()],
                2_200
            ),
            Err(StoreError::Conflict { .. })
        ));
        drop(lease);
        let result = sessions
            .catalog()
            .prune_session(
                "session",
                1,
                &["packVoltageV".into(), "dutyCycle".into()],
                2_200,
            )
            .expect("prune");
        assert_eq!(result.source_samples, 20);
        assert!(result.retained_samples < result.source_samples);
        assert_eq!(result.history_revision, 2);
        drop(sessions);
    }
}
