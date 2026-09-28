//! Closed, versioned, bounded host protocol. Never contains frontend envelopes.
use crate::{exchange::OperationId, facts::*, limits::*};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Macos,
    Ios,
    Android,
}
impl Platform {
    pub fn port(&self) -> u16 {
        match self {
            Self::Macos => 11010,
            Self::Ios => 11011,
            Self::Android => 11012,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Start {
    pub platform: Platform,
    pub port: u16,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Publication {
    pub generation: u64,
    pub sequence: u64,
    pub catalog_changed: bool,
    pub facts: Snapshot,
}
pub use float_view_backend_service::operations::Failure;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "status",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Outcome {
    Completed(Box<Completion>),
    Failed(Failure),
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionRecord {
    pub id: OperationId,
    pub outcome: Outcome,
}
pub use float_view_backend_service::operations::Operation;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishedBoundary {
    pub history_revision: u64,
    pub committed_sample_sequence: u64,
    pub committed_event_sequence: u64,
    pub committed_elapsed_ms: u64,
    pub committed_location_sequence: u64,
    pub committed_location_elapsed_ms: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Range {
    pub start_elapsed_ms: u64,
    pub end_elapsed_ms: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryQuery {
    pub kind: String,
    pub snapshot: PublishedBoundary,
    pub range: Range,
    pub fields: Vec<String>,
    pub limit: usize,
    pub buckets: usize,
    pub gap_seconds: Option<f64>,
    pub after_elapsed_ms: Option<u64>,
    pub after_sequence: Option<u64>,
    pub location_cursor: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "command",
    content = "parameters",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Command {
    TransferCleaned { transfer_id: u64 },
    Start(Start),
    Stop,
    Drain { generation: u64, wait_ms: u64 },
    Complete(CompletionRecord),
    CompleteFragment(crate::completion::Fragment),
    Publish(Box<Publication>),
    Cancel { generation: u64, client: u64 },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub version: u32,
    pub action: Command,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidEnvelope;
impl Envelope {
    pub fn decode(bytes: &[u8]) -> Result<Self, InvalidEnvelope> {
        if bytes.is_empty() || bytes.len() > MAX_HOST_ENVELOPE_BYTES {
            return Err(InvalidEnvelope);
        }
        let envelope: Self = serde_json::from_slice(bytes).map_err(|_| InvalidEnvelope)?;
        if envelope.version != VERSION {
            return Err(InvalidEnvelope);
        }
        match &envelope.action {
            Command::Start(start) if start.port != start.platform.port() => {
                return Err(InvalidEnvelope);
            }
            Command::Drain { wait_ms, .. } if *wait_ms > MAX_HOST_WAIT.as_millis() as u64 => {
                return Err(InvalidEnvelope);
            }
            Command::Publish(publication) => publication.validate()?,
            _ => {}
        }
        Ok(envelope)
    }
}
impl Publication {
    pub fn validate(&self) -> Result<(), InvalidEnvelope> {
        let f = &self.facts;
        if self.sequence == 0
            || self.sequence > 9_007_199_254_740_991
            || f.discovery.known_devices.len() > 50
            || f.saved_boards.len() > 50
        {
            return Err(InvalidEnvelope);
        }
        if let Some(active) = &f.active {
            if active.controller_candidates.len() > 16 || active.bms.candidates.len() > 16 {
                return Err(InvalidEnvelope);
            }
            if active.tracking_mode == "guest" && active.active_session.is_some() {
                return Err(InvalidEnvelope);
            }
            if let Some(sample) = &active.latest_sample
                && (sample.extension_fields.len() > 32
                    || sample
                        .extension_fields
                        .keys()
                        .any(|key| key.is_empty() || key.len() > 64))
            {
                return Err(InvalidEnvelope);
            }
            if let Some(bms) = &active.bms.latest_snapshot
                && (bms.cell_voltages_v.len() > 50
                    || bms.balancing.len() != bms.cell_voltages_v.len()
                    || bms.temperatures_c.len() > 50)
            {
                return Err(InvalidEnvelope);
            }
        }
        Ok(())
    }
}
