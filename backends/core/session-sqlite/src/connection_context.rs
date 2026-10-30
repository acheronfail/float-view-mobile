//! Public firmware provenance, independent of telemetry samples and private identity.
use super::*;
use serde::{Deserialize, Serialize};
pub const MAX_CONNECTION_CONTEXTS: usize = 1_000;
pub const MAX_CONTEXT_PAGE: usize = 64;
pub const MAX_CONTEXT_BYTES: usize = 4_096;
const SAFE: u64 = 9_007_199_254_740_991;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextFirmware {
    pub major: u8,
    pub minor: u8,
    pub test_version: u8,
    #[serde(deserialize_with = "required_name")]
    pub hardware_name: Option<String>,
    #[serde(deserialize_with = "required_name")]
    pub firmware_name: Option<String>,
}
fn required_name<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ContextRoute {
    Direct,
    Can {
        #[serde(rename = "canId")]
        can_id: u8,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionContext {
    pub version: u8,
    pub refloat_version: String,
    pub controller: ContextFirmware,
    pub transport: ContextFirmware,
    pub connection: ContextRoute,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionContextEvent {
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub observed_at_ms: i64,
    pub operation_id: String,
    pub context: ConnectionContext,
}
impl Debug for ConnectionContextEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("ConnectionContextEvent([redacted])")
    }
}
impl ConnectionContext {
    pub fn valid(&self) -> bool {
        self.version == 1
            && valid_refloat_version(&self.refloat_version)
            && [&self.controller, &self.transport].iter().all(|f| {
                [&f.hardware_name, &f.firmware_name]
                    .iter()
                    .all(|n| n.as_ref().is_none_or(|n| n.len() <= 128))
            })
            && !matches!(self.connection, ContextRoute::Can {can_id} if can_id > 253)
    }
}

fn valid_refloat_version(value: &str) -> bool {
    let (numbers, suffix) = value
        .split_once('-')
        .map_or((value, None), |(n, s)| (n, Some(s)));
    let parts: Vec<_> = numbers.split('.').collect();
    (2..=3).contains(&parts.len())
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.len() <= 3
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part.parse::<u8>().is_ok()
        })
        && suffix.is_none_or(|suffix| {
            parts.len() == 3
                && (1..=20).contains(&suffix.len())
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        })
}
impl ConnectionContextEvent {
    fn valid(&self) -> bool {
        self.sequence > 0
            && self.sequence <= SAFE
            && self.elapsed_ms <= SAFE
            && (0..=SAFE as i64).contains(&self.observed_at_ms)
            && RideMarker::valid_operation(&self.operation_id)
            && self.context.valid()
    }
}
pub(crate) fn read_contexts(
    db: &Connection,
    boundary: u64,
) -> Result<Vec<ConnectionContextEvent>, WriterError> {
    let mut statement = db.prepare("SELECT sequence,monotonic_elapsed_ms,observed_at_ms,source_sample_sequence,CASE WHEN length(CAST(details_json AS BLOB))<=4096 THEN details_json ELSE '' END FROM session_event WHERE kind='connectionContext' AND sequence<=? ORDER BY sequence LIMIT 1001").map_err(map_sqlite)?;
    let rows = statement
        .query_map([sqlite_integer(boundary)?], |r| {
            Ok((
                r.get::<_, u64>(0)?,
                r.get::<_, u64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<u64>>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(map_sqlite)?;
    let published = read_marker(db)?;
    let mut events: Vec<ConnectionContextEvent> = Vec::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        let (sequence, elapsed, observed, source, json) = row.map_err(map_sqlite)?;
        let event: ConnectionContextEvent =
            serde_json::from_str(&json).map_err(|_| WriterError::Incompatible)?;
        if !event.valid()
            || source.is_some()
            || event.sequence != sequence
            || event.elapsed_ms != elapsed
            || event.observed_at_ms != observed
            || elapsed > published.committed_elapsed_ms
            || !ids.insert(event.operation_id.clone())
            || events.last().is_some_and(|e| elapsed < e.elapsed_ms)
        {
            return Err(WriterError::Incompatible);
        }
        events.push(event);
    }
    if events.len() > MAX_CONNECTION_CONTEXTS {
        return Err(WriterError::Incompatible);
    }
    Ok(events)
}
impl SessionWriter {
    pub fn append_connection_context(
        &mut self,
        operation_id: &str,
        elapsed_ms: u64,
        at_ms: i64,
        context: ConnectionContext,
    ) -> Result<CommitMarker, WriterError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite)?;
        let boundary = read_marker(&tx)?;
        let existing = read_contexts(&tx, boundary.committed_event_sequence)?;
        if let Some(event) = existing.iter().find(|e| e.operation_id == operation_id) {
            return if event.context == context {
                Ok(boundary)
            } else {
                Err(WriterError::InvalidInput)
            };
        }
        if existing.len() >= MAX_CONNECTION_CONTEXTS {
            return Err(WriterError::Full);
        }
        let event = ConnectionContextEvent {
            sequence: boundary
                .committed_event_sequence
                .checked_add(1)
                .ok_or(WriterError::InvalidInput)?,
            elapsed_ms,
            observed_at_ms: at_ms,
            operation_id: operation_id.into(),
            context,
        };
        if !event.valid() || existing.last().is_some_and(|e| elapsed_ms < e.elapsed_ms) {
            return Err(WriterError::InvalidInput);
        }
        let json = serde_json::to_string(&event).map_err(|_| WriterError::InvalidInput)?;
        if json.len() > MAX_CONTEXT_BYTES {
            return Err(WriterError::InvalidInput);
        }
        tx.execute("INSERT INTO session_event(sequence,observed_at_ms,monotonic_elapsed_ms,kind,details_json) VALUES (?,?,?,'connectionContext',?)",params![event.sequence,at_ms,elapsed_ms,json]).map_err(map_sqlite)?;
        tx.execute("UPDATE commit_marker SET committed_event_sequence=?,committed_event_count=committed_event_count+1,committed_elapsed_ms=MAX(committed_elapsed_ms,?),committed_at_ms=? WHERE singleton=1",params![event.sequence,elapsed_ms,at_ms]).map_err(map_sqlite)?;
        let marker = read_marker(&tx)?;
        tx.commit().map_err(map_sqlite)?;
        Ok(marker)
    }
}
