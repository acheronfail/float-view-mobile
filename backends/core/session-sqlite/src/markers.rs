use super::*;
use serde::{Deserialize, Serialize};

pub const MAX_RIDE_MARKERS: usize = 1_000;
pub const MAX_MARKER_PAGE: usize = 256;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarkerLocation {
    pub location_sequence: u64,
    pub elapsed_ms: u64,
    pub observed_at_ms: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RideMarker {
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub observed_at_ms: i64,
    pub operation_id: String,
    pub location: Option<MarkerLocation>,
}
impl Debug for RideMarker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("RideMarker([redacted])")
    }
}
impl RideMarker {
    pub fn valid_operation(id: &str) -> bool {
        id.len() == 36
            && id.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            })
    }
    fn valid(&self) -> bool {
        Self::valid_operation(&self.operation_id)
            && self.sequence > 0
            && self.sequence <= 9_007_199_254_740_991
            && self.elapsed_ms <= 9_007_199_254_740_991
            && (0..=9_007_199_254_740_991).contains(&self.observed_at_ms)
            && self.location.as_ref().is_none_or(|p| {
                p.location_sequence > 0
                    && p.location_sequence <= 9_007_199_254_740_991
                    && p.elapsed_ms <= self.elapsed_ms
                    && self.elapsed_ms - p.elapsed_ms <= 10_000
                    && p.observed_at_ms >= 0
                    && p.observed_at_ms <= self.observed_at_ms
                    && self.observed_at_ms - p.observed_at_ms <= 10_000
                    && p.latitude.is_finite()
                    && (-90.0..=90.0).contains(&p.latitude)
                    && p.longitude.is_finite()
                    && (-180.0..=180.0).contains(&p.longitude)
                    && p.accuracy_m.is_none_or(|v| v.is_finite() && v >= 0.)
            })
    }
}

pub(crate) fn read_markers(db: &Connection, boundary: u64) -> Result<Vec<RideMarker>, WriterError> {
    let mut statement = db.prepare("SELECT sequence, monotonic_elapsed_ms, observed_at_ms, CASE WHEN length(details_json)<=1024 THEN details_json ELSE '' END FROM session_event WHERE kind='session_marker' AND sequence<=? ORDER BY monotonic_elapsed_ms,sequence LIMIT 1001").map_err(map_sqlite)?;
    let rows = statement
        .query_map([sqlite_integer(boundary)?], |r| {
            Ok((
                r.get::<_, u64>(0)?,
                r.get::<_, u64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(map_sqlite)?;
    let published = read_marker(db)?;
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        let (sequence, elapsed, observed, json) = row.map_err(map_sqlite)?;
        if json.len() > 1024 {
            return Err(WriterError::Incompatible);
        }
        let marker: RideMarker =
            serde_json::from_str(&json).map_err(|_| WriterError::Incompatible)?;
        if !marker.valid()
            || marker.elapsed_ms > published.committed_elapsed_ms
            || marker
                .location
                .as_ref()
                .is_some_and(|p| p.location_sequence > published.committed_location_sequence)
            || marker.sequence != sequence
            || marker.elapsed_ms != elapsed
            || marker.observed_at_ms != observed
            || !ids.insert(marker.operation_id.clone())
        {
            return Err(WriterError::Incompatible);
        }
        result.push(marker);
    }
    if result.len() > MAX_RIDE_MARKERS {
        return Err(WriterError::Incompatible);
    }
    Ok(result)
}
impl SessionWriter {
    pub fn append_ride_marker(
        &mut self,
        operation_id: &str,
        elapsed_ms: u64,
        at_ms: i64,
        capture_location: bool,
    ) -> Result<(RideMarker, CommitMarker), WriterError> {
        if !RideMarker::valid_operation(operation_id) || at_ms < 0 {
            return Err(WriterError::InvalidInput);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite)?;
        let boundary = read_marker(&tx)?;
        let markers = read_markers(&tx, boundary.committed_event_sequence)?;
        if let Some(existing) = markers.iter().find(|m| m.operation_id == operation_id) {
            return Ok((existing.clone(), boundary));
        }
        if markers.len() >= MAX_RIDE_MARKERS
            || markers
                .last()
                .is_some_and(|m| elapsed_ms < m.elapsed_ms.saturating_add(500))
        {
            return Err(WriterError::InvalidInput);
        }
        let location = if capture_location {
            tx.query_row("SELECT location_sequence,monotonic_elapsed_ms,observed_at_ms,latitude,longitude,accuracy_m FROM location WHERE location_sequence<=? ORDER BY location_sequence DESC LIMIT 1", [boundary.committed_location_sequence], |r| Ok(MarkerLocation {location_sequence:r.get(0)?,elapsed_ms:r.get(1)?,observed_at_ms:r.get(2)?,latitude:r.get(3)?,longitude:r.get(4)?,accuracy_m:r.get(5)?})).optional().map_err(map_sqlite)?
                .filter(|p| p.elapsed_ms<=elapsed_ms && elapsed_ms-p.elapsed_ms<=10_000 && p.observed_at_ms<=at_ms && at_ms-p.observed_at_ms<=10_000)
        } else {
            None
        };
        let marker = RideMarker {
            sequence: boundary
                .committed_event_sequence
                .checked_add(1)
                .ok_or(WriterError::InvalidInput)?,
            elapsed_ms,
            observed_at_ms: at_ms,
            operation_id: operation_id.into(),
            location,
        };
        if !marker.valid() {
            return Err(WriterError::InvalidInput);
        }
        let json = serde_json::to_string(&marker).map_err(|_| WriterError::InvalidInput)?;
        tx.execute("INSERT INTO session_event(sequence,observed_at_ms,monotonic_elapsed_ms,kind,details_json) VALUES (?,?,?,'session_marker',?)", params![marker.sequence,at_ms,elapsed_ms,json]).map_err(map_sqlite)?;
        tx.execute("UPDATE commit_marker SET committed_event_sequence=?,committed_event_count=committed_event_count+1,committed_elapsed_ms=MAX(committed_elapsed_ms,?),committed_at_ms=? WHERE singleton=1",params![marker.sequence,elapsed_ms,at_ms]).map_err(map_sqlite)?;
        let committed = read_marker(&tx)?;
        tx.commit().map_err(map_sqlite)?;
        Ok((marker, committed))
    }
}
