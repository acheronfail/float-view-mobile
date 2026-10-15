//! Portable service operations; these are never native OS effects.
use crate::facts::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    TestAnnouncement {
        kind: crate::announcements::TestKind,
    },
    AddRideMarker {
        session_id: String,
        operation_id: String,
    },
    RecoverPermission {
        permission: crate::permissions::Id,
        action: crate::permissions::Action,
    },
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
        refresh: bool,
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
    CancelMaintenance {
        id: String,
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
