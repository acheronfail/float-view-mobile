//! Closed, versioned, bounded host protocol. Never contains frontend envelopes.
use crate::limits::*;
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
pub use float_view_backend_service::operations::Failure;
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
    Start(Start),
    Stop,
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
            _ => {}
        }
        Ok(envelope)
    }
}
