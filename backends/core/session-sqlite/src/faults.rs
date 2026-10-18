//! Durable firmware fault history. Uses the existing extensible event storage.
use super::*;
use float_view_refloat_telemetry_service::{FaultHistoryObservation, reported_firmware_fault};

pub const MAX_FIRMWARE_FAULT_EVENTS: u64 = 10_000;

impl SessionWriter {
    pub fn append_fault_history(
        &mut self,
        observation: &FaultHistoryObservation,
    ) -> Result<CommitMarker, WriterError> {
        if observation.records.is_empty()
            || observation.records.len() > 20
            || observation.observed_at_epoch_ms < 0
            || observation
                .records
                .iter()
                .any(|r| r.active != (r.raw_code != 0))
        {
            return Err(WriterError::InvalidInput);
        }
        sqlite_integer(observation.elapsed_ms)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite)?;
        let marker = read_marker(&tx)?;
        let count: u64 = tx.query_row("SELECT COUNT(*) FROM (SELECT 1 FROM session_event WHERE kind='firmwareFaultHistory' LIMIT 10001)", [], |r| r.get(0)).map_err(map_sqlite)?;
        if count + observation.records.len() as u64 > MAX_FIRMWARE_FAULT_EVENTS {
            return Err(WriterError::Full);
        }
        let mut sequence = marker.committed_event_sequence;
        for record in &observation.records {
            sequence = sequence.checked_add(1).ok_or(WriterError::InvalidInput)?;
            let value = reported_firmware_fault(record.raw_code)
                .map(|v| firmware_fault(v.reason))
                .unwrap_or("none");
            let details = serde_json::json!({"value": value, "active": record.active, "rawCode": record.raw_code, "sourceTicks": record.source_ticks, "occurredElapsedMs": float_view_refloat_telemetry_service::fault_elapsed_ms(record.source_ticks, observation.source_clock, observation.elapsed_ms)}).to_string();
            tx.execute("INSERT INTO session_event(sequence,observed_at_ms,monotonic_elapsed_ms,kind,details_json) VALUES (?,?,?,'firmwareFaultHistory',?)", params![sqlite_integer(sequence)?,observation.observed_at_epoch_ms,sqlite_integer(observation.elapsed_ms)?,details]).map_err(map_sqlite)?;
        }
        tx.execute("UPDATE commit_marker SET committed_event_sequence=?,committed_event_count=committed_event_count+?,committed_elapsed_ms=MAX(committed_elapsed_ms,?),committed_at_ms=? WHERE singleton=1",params![sqlite_integer(sequence)?,observation.records.len() as u64,sqlite_integer(observation.elapsed_ms)?,observation.observed_at_epoch_ms]).map_err(map_sqlite)?;
        let committed = read_marker(&tx)?;
        tx.commit().map_err(map_sqlite)?;
        Ok(committed)
    }
}

pub(crate) fn valid_fault_details(value: &serde_json::Value) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == 4 || (o.len() == 5 && o.contains_key("occurredElapsedMs")))
        && value
            .get("occurredElapsedMs")
            .is_none_or(|v| v.is_null() || v.as_u64().is_some_and(|n| n <= 9_007_199_254_740_991))
        && value["value"].as_str().is_some_and(|s| s.len() <= 64)
        && value["sourceTicks"]
            .as_u64()
            .is_some_and(|v| v <= u32::MAX as u64)
        && value["rawCode"].as_u64().is_some_and(|code| {
            code <= 255
                && value["active"].as_bool() == Some(code != 0)
                && value["value"].as_str()
                    == Some(
                        reported_firmware_fault(code as u8)
                            .map(|v| firmware_fault(v.reason))
                            .unwrap_or("none"),
                    )
        })
}
