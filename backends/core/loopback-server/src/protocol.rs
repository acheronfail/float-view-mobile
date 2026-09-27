//! Closed, versioned, bounded host protocol. Never contains frontend envelopes.
use crate::{exchange::OperationId, facts::*, limits::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    InvalidRequest,
    InvalidState,
    PermissionRequired,
    NotFound,
    Conflict,
    Unavailable,
    Internal,
}
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    RequestBluetoothPermission,
    RequestLocationPermission,
    OpenLocationSettings,
    Scan {
        timeout_ms: u32,
    },
    Connect {
        device_id: String,
        saved: bool,
    },
    SelectController {
        device_id: String,
        can_id: u8,
    },
    Disconnect,
    ShutdownController,
    SetControl {
        control: String,
        enabled: bool,
    },
    ClearFatalError,
    ConfigurationSchema,
    ReadConfiguration {
        defaults: bool,
    },
    WriteConfiguration {
        package_version: String,
        fingerprint: String,
        signature: u32,
        revision: u64,
        values: BTreeMap<String, ConfigurationValue>,
    },
    ListTunes {
        identity: ConfigurationIdentity,
        limit: usize,
        cursor: Option<String>,
    },
    GetTune {
        id: String,
    },
    CreateTune {
        name: String,
        description: Option<String>,
        identity: ConfigurationIdentity,
        revision: u64,
        values: BTreeMap<String, ConfigurationValue>,
    },
    UpdateTune {
        id: String,
        revision: u64,
        name: String,
        description: Option<String>,
        values: BTreeMap<String, ConfigurationValue>,
    },
    DeleteTune {
        id: String,
        revision: u64,
    },
    ApplyTune {
        id: String,
        revision: u64,
        base_revision: u64,
    },
    UpdateSavedBoard {
        id: String,
        revision: u64,
        name: Option<String>,
    },
    DeleteSavedBoard {
        id: String,
    },
    UpdateSettings {
        revision: u64,
        patch_json: String,
    },
    GetProfile {
        id: String,
    },
    UpdateProfile {
        id: String,
        revision: u64,
        patch_json: String,
    },
    ListSessions {
        history_id: Option<String>,
        limit: usize,
        cursor: Option<String>,
        completed_only: bool,
    },
    GetSession {
        id: String,
    },
    ExportSession {
        id: String,
        format: String,
        present: bool,
        transfer_id: u64,
    },
    DeleteSession {
        id: String,
    },
    CancelDelete {
        id: String,
    },
    GetMaintenance,
    SaveMaintenance {
        job: Maintenance,
    },
    CancelMaintenance {
        id: String,
    },
    PruneSession {
        id: String,
        resolution: u64,
        preview: bool,
    },
    WriteTransfer {
        transfer_id: u64,
        offset: u64,
    },
    ReadTransfer {
        transfer_id: u64,
        offset: u64,
        length: usize,
    },
    BeginImport {
        transfer_id: u64,
    },
    FinishImport {
        transfer_id: u64,
    },
    CleanupTransfer {
        transfer_id: u64,
    },
}
impl Operation {
    pub fn is_read(&self) -> bool {
        matches!(
            self,
            Self::ConfigurationSchema
                | Self::ListTunes { .. }
                | Self::GetTune { .. }
                | Self::GetProfile { .. }
                | Self::ListSessions { .. }
                | Self::GetSession { .. }
                | Self::GetMaintenance
        )
    }
}
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
