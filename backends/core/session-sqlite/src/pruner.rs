use std::collections::BTreeSet;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use float_view_session_history_kernel::{PruneConfig, PruneObservation, PruneSelector, RidePhase};
use rusqlite::ffi::ErrorCode;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};

use super::{CommitMarker, WriterError, read_marker, validate_database};

pub const MAX_PRUNE_SOURCE_ROWS: usize = 200_000;
const MAX_TARGET_RESOLUTION_SECONDS: u64 = 300;
const PRUNE_LOCATIONS: &str = "DELETE FROM location WHERE NOT EXISTS (SELECT 1 FROM sample WHERE sample.monotonic_elapsed_ms BETWEEN location.monotonic_elapsed_ms - ?1 AND location.monotonic_elapsed_ms + ?1)";
const PRUNE_POLICY: &str = "m4-source-v1";
static PRUNER_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PruneError {
    InvalidInput,
    NotFound,
    Incompatible,
    Corrupt,
    Busy,
    Cancelled,
    Stale,
    Io,
    Database,
}

impl Display for PruneError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, formatter)
    }
}

impl std::error::Error for PruneError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrunePreview {
    pub recorded_sample_count: u64,
    pub stored_sample_count: u64,
    pub retained_sample_count: u64,
    pub estimated_reclaimable_bytes: u64,
    pub target_resolution_seconds: u64,
    pub history_revision: u64,
    pub skipped: bool,
}

pub struct PrunePlan {
    preview: PrunePreview,
    owner: u64,
    marker: CommitMarker,
    retained_resolution_seconds: u64,
    keep: Vec<u64>,
}

impl PrunePlan {
    pub fn preview(&self) -> PrunePreview {
        self.preview
    }
}

impl Debug for PrunePlan {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrunePlan")
            .field("preview", &self.preview)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PruneResult {
    pub preview: PrunePreview,
    pub history_revision: u64,
    pub stored_location_count: u64,
    pub skipped: bool,
}

#[derive(Clone)]
pub struct PruneInterrupter(Arc<rusqlite::InterruptHandle>);

impl PruneInterrupter {
    pub fn interrupt(&self) {
        self.0.interrupt();
    }
}

impl Debug for PruneInterrupter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PruneInterrupter([redacted])")
    }
}

pub struct SessionPruner {
    connection: Connection,
    interrupter: PruneInterrupter,
    owner: u64,
}

impl Debug for SessionPruner {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionPruner([redacted])")
    }
}

impl SessionPruner {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PruneError> {
        let path = path.as_ref();
        if !path.is_file() {
            return Err(PruneError::NotFound);
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(map_open)?;
        connection
            .busy_timeout(Duration::from_millis(250))
            .map_err(map_sqlite)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(map_sqlite)?;
        validate_database(&connection).map_err(map_writer)?;
        validate_prune_state(&connection)?;
        let interrupter = PruneInterrupter(Arc::new(connection.get_interrupt_handle()));
        let owner = PRUNER_SEQUENCE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| PruneError::InvalidInput)?;
        Ok(Self {
            connection,
            interrupter,
            owner,
        })
    }

    pub fn interrupter(&self) -> PruneInterrupter {
        self.interrupter.clone()
    }

    pub fn close(self) -> Result<(), PruneError> {
        self.connection
            .close()
            .map_err(|(_, error)| map_sqlite(error))
    }

    pub fn preview(&mut self, target_resolution_seconds: u64) -> Result<PrunePlan, PruneError> {
        if !(1..=MAX_TARGET_RESOLUTION_SECONDS).contains(&target_resolution_seconds) {
            return Err(PruneError::InvalidInput);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(map_sqlite)?;
        let marker = read_marker(&transaction).map_err(map_writer)?;
        let retained_resolution_seconds = read_retained_resolution(&transaction)?;
        validate_resolution(retained_resolution_seconds, target_resolution_seconds)?;
        validate_counts(&transaction, &marker)?;
        let protected = read_protected_sequences(&transaction, &marker)?;
        let keep = select_rows(&transaction, target_resolution_seconds, &protected)?;
        let retained_sample_count =
            u64::try_from(keep.len()).map_err(|_| PruneError::InvalidInput)?;
        let removed = marker
            .stored_sample_count
            .checked_sub(retained_sample_count)
            .ok_or(PruneError::Incompatible)?;
        let preview = PrunePreview {
            recorded_sample_count: marker.recorded_sample_count,
            stored_sample_count: marker.stored_sample_count,
            retained_sample_count,
            estimated_reclaimable_bytes: estimate_reclaimable_bytes(
                &transaction,
                removed,
                marker.stored_sample_count,
            )?,
            target_resolution_seconds,
            history_revision: marker.history_revision,
            skipped: removed == 0,
        };
        transaction.commit().map_err(map_sqlite)?;
        Ok(PrunePlan {
            preview,
            owner: self.owner,
            marker,
            retained_resolution_seconds,
            keep,
        })
    }

    pub fn commit(
        &mut self,
        plan: PrunePlan,
        committed_at_epoch_ms: i64,
    ) -> Result<PruneResult, PruneError> {
        if committed_at_epoch_ms < 0 || plan.owner != self.owner {
            return Err(PruneError::InvalidInput);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite)?;
        let current_marker = read_marker(&transaction).map_err(map_writer)?;
        let current_resolution = read_retained_resolution(&transaction)?;
        if current_marker != plan.marker || current_resolution != plan.retained_resolution_seconds {
            return Err(PruneError::Stale);
        }
        if plan.preview.skipped {
            transaction.commit().map_err(map_sqlite)?;
            return Ok(PruneResult {
                preview: plan.preview,
                history_revision: current_marker.history_revision,
                stored_location_count: current_marker.stored_location_count,
                skipped: true,
            });
        }
        apply_plan(&transaction, &plan, committed_at_epoch_ms)?;
        let marker = read_marker(&transaction).map_err(map_writer)?;
        transaction.commit().map_err(map_sqlite)?;
        Ok(PruneResult {
            preview: plan.preview,
            history_revision: marker.history_revision,
            stored_location_count: marker.stored_location_count,
            skipped: false,
        })
    }
}

// Estimate only sample-owned pages: diagnostics, locations and events survive
// independently. dbstat reads page structure without copying or pruning a DB.
// Retained rows vary in size and partially occupied pages cannot be reclaimed
// by incremental vacuum, so this remains an estimate rather than a guarantee.
fn estimate_reclaimable_bytes(
    connection: &Connection,
    removed: u64,
    stored: u64,
) -> Result<u64, PruneError> {
    if removed == 0 {
        return Ok(0);
    }
    let sample_bytes: u64 = connection.query_row(
        "SELECT COALESCE(SUM(pgsize), 0) FROM dbstat WHERE name IN ('sample', 'sample_elapsed')",
        [], |row| row.get(0),
    ).map_err(map_sqlite)?;
    sample_bytes
        .checked_mul(removed)
        .and_then(|bytes| bytes.checked_div(stored))
        .ok_or(PruneError::InvalidInput)
}

fn validate_resolution(current: u64, target: u64) -> Result<(), PruneError> {
    if current > 0 && (target <= current || !target.is_multiple_of(current)) {
        Err(PruneError::InvalidInput)
    } else {
        Ok(())
    }
}

fn read_retained_resolution(connection: &Connection) -> Result<u64, PruneError> {
    let value: Option<i64> = connection
        .query_row(
            "SELECT retained_resolution_seconds FROM session_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(map_sqlite)?;
    let value = value.unwrap_or(0);
    let value = u64::try_from(value).map_err(|_| PruneError::Incompatible)?;
    if value > MAX_TARGET_RESOLUTION_SECONDS {
        return Err(PruneError::Incompatible);
    }
    Ok(value)
}

fn validate_prune_state(connection: &Connection) -> Result<(), PruneError> {
    let marker = read_marker(connection).map_err(map_writer)?;
    read_retained_resolution(connection)?;
    let foreign_key_issue: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM pragma_foreign_key_check LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_sqlite)?;
    if foreign_key_issue.is_some() {
        return Err(PruneError::Incompatible);
    }
    validate_counts(connection, &marker)
}

fn validate_counts(connection: &Connection, marker: &CommitMarker) -> Result<(), PruneError> {
    let (samples, events, locations): (i64, i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM sample), (SELECT COUNT(*) FROM session_event), (SELECT COUNT(*) FROM location)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(map_sqlite)?;
    if samples < 0
        || events < 0
        || locations < 0
        || u64::try_from(samples).ok() != Some(marker.stored_sample_count)
        || u64::try_from(events).ok() != Some(marker.committed_event_count)
        || u64::try_from(locations).ok() != Some(marker.stored_location_count)
        || marker.stored_sample_count > marker.recorded_sample_count
        || marker.stored_location_count > marker.recorded_location_count
        || marker.stored_sample_count > MAX_PRUNE_SOURCE_ROWS as u64
    {
        return Err(PruneError::Incompatible);
    }
    Ok(())
}

fn read_protected_sequences(
    transaction: &Transaction<'_>,
    marker: &CommitMarker,
) -> Result<BTreeSet<u64>, PruneError> {
    let mut protected = BTreeSet::new();
    for sequence in [
        marker.maximum_speed_source_sequence,
        marker.maximum_duty_source_sequence,
    ]
    .into_iter()
    .flatten()
    {
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sample WHERE sequence = ?)",
                [i64::try_from(sequence).map_err(|_| PruneError::Incompatible)?],
                |row| row.get(0),
            )
            .map_err(map_sqlite)?;
        if !exists {
            return Err(PruneError::Incompatible);
        }
        protected.insert(sequence);
    }
    for sql in [
        "SELECT first_sequence FROM session_numeric_field ORDER BY first_sequence",
        "SELECT source_sample_sequence FROM session_event WHERE source_sample_sequence IS NOT NULL ORDER BY source_sample_sequence",
    ] {
        let mut statement = transaction.prepare(sql).map_err(map_sqlite)?;
        let mut rows = statement.query([]).map_err(map_sqlite)?;
        while let Some(row) = rows.next().map_err(map_sqlite)? {
            let value: i64 = row.get(0).map_err(map_sqlite)?;
            protected.insert(u64::try_from(value).map_err(|_| PruneError::Incompatible)?);
            if protected.len() > MAX_PRUNE_SOURCE_ROWS {
                return Err(PruneError::Incompatible);
            }
        }
    }
    Ok(protected)
}

fn select_rows(
    transaction: &Transaction<'_>,
    target_resolution_seconds: u64,
    protected: &BTreeSet<u64>,
) -> Result<Vec<u64>, PruneError> {
    let fields = vec!["speedKph".to_owned(), "packVoltageV".to_owned()];
    let mut selector = PruneSelector::new(PruneConfig {
        fields,
        resolution_seconds: target_resolution_seconds,
    })
    .map_err(|_| PruneError::InvalidInput)?;
    let mut statement = transaction
        .prepare("SELECT sequence, monotonic_elapsed_ms, ride_state, speed_kph, raw_pack_voltage_v FROM sample ORDER BY monotonic_elapsed_ms, sequence LIMIT ?")
        .map_err(map_sqlite)?;
    let mut rows = statement
        .query([i64::try_from(MAX_PRUNE_SOURCE_ROWS + 1).expect("small bound")])
        .map_err(map_sqlite)?;
    let mut keep = Vec::new();
    let mut count = 0_usize;
    while let Some(row) = rows.next().map_err(map_sqlite)? {
        count += 1;
        if count > MAX_PRUNE_SOURCE_ROWS {
            return Err(PruneError::InvalidInput);
        }
        let sequence = u64::try_from(row.get::<_, i64>(0).map_err(map_sqlite)?)
            .map_err(|_| PruneError::Incompatible)?;
        let elapsed_ms = u64::try_from(row.get::<_, i64>(1).map_err(map_sqlite)?)
            .map_err(|_| PruneError::Incompatible)?;
        let ride_state: String = row.get(2).map_err(map_sqlite)?;
        let ride_phase = match ride_state.as_str() {
            "riding" => RidePhase::Riding,
            "idle" | "unknown" => RidePhase::Other,
            _ => return Err(PruneError::Incompatible),
        };
        let values = [
            finite(row.get::<_, Option<f64>>(3).map_err(map_sqlite)?)?,
            finite(row.get::<_, Option<f64>>(4).map_err(map_sqlite)?)?,
        ];
        if let Some(bucket) = selector
            .append(PruneObservation {
                elapsed_ms,
                sequence,
                ride_phase,
                values: &values,
                protected: protected.contains(&sequence),
            })
            .map_err(|_| PruneError::Incompatible)?
        {
            keep.extend(bucket.retained.into_iter().map(|row| row.sequence));
        }
    }
    if let Some(bucket) = selector.finish().map_err(|_| PruneError::Incompatible)? {
        keep.extend(bucket.retained.into_iter().map(|row| row.sequence));
    }
    keep.sort_unstable();
    keep.dedup();
    if keep.len() > count {
        return Err(PruneError::Incompatible);
    }
    Ok(keep)
}

fn apply_plan(
    transaction: &Transaction<'_>,
    plan: &PrunePlan,
    committed_at_epoch_ms: i64,
) -> Result<(), PruneError> {
    transaction
        .execute_batch("CREATE TEMP TABLE prune_keep(sequence INTEGER PRIMARY KEY);")
        .map_err(map_sqlite)?;
    {
        let mut insert = transaction
            .prepare("INSERT INTO prune_keep(sequence) VALUES (?)")
            .map_err(map_sqlite)?;
        for &sequence in &plan.keep {
            insert
                .execute([i64::try_from(sequence).map_err(|_| PruneError::InvalidInput)?])
                .map_err(map_sqlite)?;
        }
    }
    transaction
        .execute(
            "DELETE FROM sample WHERE sequence NOT IN (SELECT sequence FROM prune_keep)",
            [],
        )
        .map_err(map_sqlite)?;
    let neighborhood_ms = plan
        .preview
        .target_resolution_seconds
        .checked_mul(1_000)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or(PruneError::InvalidInput)?;
    transaction
        .execute(PRUNE_LOCATIONS, [neighborhood_ms])
        .map_err(map_sqlite)?;
    transaction
        .execute(
            "UPDATE session_metadata SET pruned_at_ms = ?, retained_resolution_seconds = ?, prune_policy = ? WHERE singleton = 1",
            params![
                committed_at_epoch_ms,
                i64::try_from(plan.preview.target_resolution_seconds)
                    .map_err(|_| PruneError::InvalidInput)?,
                PRUNE_POLICY,
            ],
        )
        .map_err(map_sqlite)?;
    let changed = transaction
        .execute(
            "UPDATE commit_marker SET stored_sample_count = (SELECT COUNT(*) FROM sample), stored_location_count = (SELECT COUNT(*) FROM location), history_revision = history_revision + 1, committed_at_ms = ? WHERE singleton = 1",
            [committed_at_epoch_ms],
        )
        .map_err(map_sqlite)?;
    if changed != 1 {
        return Err(PruneError::Incompatible);
    }
    transaction
        .execute_batch("DROP TABLE prune_keep;")
        .map_err(map_sqlite)?;
    Ok(())
}

fn finite(value: Option<f64>) -> Result<Option<f64>, PruneError> {
    if value.is_some_and(|number| !number.is_finite()) {
        Err(PruneError::Incompatible)
    } else {
        Ok(value)
    }
}

fn map_writer(error: WriterError) -> PruneError {
    match error {
        WriterError::InvalidInput | WriterError::TargetExists | WriterError::Incompatible => {
            PruneError::Incompatible
        }
        WriterError::Corrupt => PruneError::Corrupt,
        WriterError::Busy => PruneError::Busy,
        WriterError::Full | WriterError::Io => PruneError::Io,
        WriterError::Database => PruneError::Database,
    }
}

fn map_open(error: rusqlite::Error) -> PruneError {
    match error.sqlite_error_code() {
        Some(ErrorCode::CannotOpen) => PruneError::NotFound,
        Some(ErrorCode::PermissionDenied) => PruneError::Io,
        _ => map_sqlite(error),
    }
}

fn map_sqlite(error: rusqlite::Error) -> PruneError {
    match error.sqlite_error_code() {
        Some(ErrorCode::OperationInterrupted) => PruneError::Cancelled,
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => PruneError::Busy,
        Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase) => PruneError::Corrupt,
        Some(ErrorCode::SystemIoFailure | ErrorCode::CannotOpen | ErrorCode::PermissionDenied) => {
            PruneError::Io
        }
        _ => PruneError::Database,
    }
}

#[cfg(test)]
mod query_plan_tests {
    use super::*;
    #[test]
    fn location_pruning_uses_sample_range_index_and_keeps_inclusive_boundaries() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(super::super::SESSION_SCHEMA).unwrap();
        db.execute_batch("INSERT INTO sample(sequence,observed_at_ms,monotonic_elapsed_ms,ride_state) VALUES (1,100,100,'idle'),(2,500,500,'idle'); INSERT INTO location(location_sequence,observed_at_ms,monotonic_elapsed_ms,latitude,longitude) VALUES (1,0,0,0,0),(2,200,200,0,0),(3,300,300,0,0),(4,400,400,0,0),(5,600,600,0,0),(6,601,601,0,0);").unwrap();
        let mut statement = db
            .prepare(&format!("EXPLAIN QUERY PLAN {PRUNE_LOCATIONS}"))
            .unwrap();
        let plan = statement
            .query_map([100], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
            .join("; ");
        assert!(
            plan.contains("SEARCH sample USING COVERING INDEX sample_elapsed"),
            "{plan}"
        );
        db.execute(PRUNE_LOCATIONS, [100]).unwrap();
        let mut statement = db
            .prepare("SELECT location_sequence FROM location ORDER BY location_sequence")
            .unwrap();
        let retained = statement
            .query_map([], |r| r.get::<_, u64>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(retained, vec![1, 2, 4, 5]);
        let mut statement = db
            .prepare(
                "EXPLAIN QUERY PLAN SELECT 1 FROM session_event WHERE source_sample_sequence=?",
            )
            .unwrap();
        let plan = statement
            .query_map([1], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
            .join("; ");
        assert!(
            plan.contains("SEARCH session_event USING COVERING INDEX session_event_source_sample"),
            "{plan}"
        );
    }
}
