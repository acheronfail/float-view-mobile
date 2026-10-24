//! Durable causes for recording discontinuities, including detach/reopen.
use super::*;
use serde::{Deserialize, Serialize};

const MAX_INTERRUPTION_EVENTS: usize = 2_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InterruptionReason {
    Pause,
    ConnectionLoss,
    #[default]
    Unknown,
}

// A cause is pending only until another sample has been committed. Never
// infer intent from a ride checkpoint, elapsed time or a connection context.
pub(crate) fn pending_reason(db: &Connection) -> Result<Option<InterruptionReason>, WriterError> {
    let row: Option<String> = db.query_row(
        "SELECT CASE WHEN length(CAST(details_json AS BLOB))<=128 THEN details_json ELSE '' END
         FROM session_event WHERE kind='recordingInterrupted'
         AND source_sample_sequence=(SELECT MAX(sequence) FROM sample)
         AND sequence>COALESCE((SELECT MAX(sequence) FROM session_event WHERE kind='recordingBoundary'),0)
         ORDER BY sequence DESC LIMIT 1", [], |r| r.get(0),
    ).optional().map_err(map_sqlite)?;
    row.map(|json| {
        let details: serde_json::Value =
            serde_json::from_str(&json).map_err(|_| WriterError::Incompatible)?;
        serde_json::from_value(details["reason"].clone()).map_err(|_| WriterError::Incompatible)
    })
    .transpose()
}

impl SessionWriter {
    pub fn interrupt_recording(
        &mut self,
        reason: InterruptionReason,
        at_ms: i64,
    ) -> Result<CommitMarker, WriterError> {
        if !(0..=9_007_199_254_740_991).contains(&at_ms) {
            return Err(WriterError::InvalidInput);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite)?;
        let marker = read_marker(&tx)?;
        let pending = pending_reason(&tx)?;
        // The first cause wins, except that deliberate user intervention can
        // supersede recovery which has not produced another sample yet.
        if marker.committed_sample_sequence == 0
            || pending.is_some_and(|p| {
                p == reason || p == InterruptionReason::Pause || reason != InterruptionReason::Pause
            })
        {
            return Ok(marker);
        }
        let count: usize = tx.query_row("SELECT COUNT(*) FROM (SELECT 1 FROM session_event WHERE kind='recordingInterrupted' LIMIT 2001)", [], |r| r.get(0)).map_err(map_sqlite)?;
        if count >= MAX_INTERRUPTION_EVENTS {
            return Err(WriterError::Full);
        }
        let sequence = marker
            .committed_event_sequence
            .checked_add(1)
            .ok_or(WriterError::InvalidInput)?;
        tx.execute("INSERT INTO session_event(sequence,source_sample_sequence,observed_at_ms,monotonic_elapsed_ms,kind,details_json) VALUES (?,?,?,?,'recordingInterrupted',?)", params![sqlite_integer(sequence)?,sqlite_integer(marker.committed_sample_sequence)?,at_ms,sqlite_integer(marker.committed_elapsed_ms)?,serde_json::json!({"reason":reason}).to_string()]).map_err(map_sqlite)?;
        tx.execute("UPDATE commit_marker SET committed_event_sequence=?,committed_event_count=committed_event_count+1,committed_at_ms=? WHERE singleton=1",params![sqlite_integer(sequence)?,at_ms]).map_err(map_sqlite)?;
        let marker = read_marker(&tx)?;
        tx.commit().map_err(map_sqlite)?;
        self.recording_gap = true;
        self.interruption_reason = reason;
        Ok(marker)
    }
}
