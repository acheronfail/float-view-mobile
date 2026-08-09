use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::settings::Settings;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AppState {
    pub(super) revision: u64,
    pub(super) bluetooth: BluetoothState,
    pub(super) discovered_devices: Vec<DiscoveredDevice>,
    pub(super) saved_devices: Vec<SavedDevice>,
    pub(super) active_connection: Option<ActiveConnection>,
    pub(super) session_catalog_revision: u64,
    pub(super) settings: Settings,
    pub(super) location: LocationState,
    pub(super) session_operation: Option<SessionOperationState>,
    pub(super) document_operation: Option<DocumentOperationState>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            revision: 0,
            bluetooth: BluetoothState::default(),
            discovered_devices: Vec::new(),
            saved_devices: Vec::new(),
            active_connection: None,
            session_catalog_revision: 0,
            settings: Settings::default(),
            location: LocationState::disabled(),
            session_operation: None,
            document_operation: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BluetoothState {
    pub(super) radio: String,
    pub(super) scan_permission: String,
    pub(super) connect_permission: String,
    pub(super) scanning: bool,
}

impl Default for BluetoothState {
    fn default() -> Self {
        Self {
            radio: "powered_on".into(),
            scan_permission: "granted".into(),
            connect_permission: "granted".into(),
            scanning: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DiscoveredDevice {
    pub(super) device_id: String,
    pub(super) name: Option<String>,
    pub(super) state: String,
    pub(super) last_seen_at: Option<String>,
    pub(super) rssi: Option<i16>,
    pub(super) saved_device_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BoardHistorySummary {
    pub(super) session_count: usize,
    pub(super) total_riding_elapsed_ms: u64,
    pub(super) total_distance_m: Option<f64>,
    pub(super) most_recent_session_at: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SavedDevice {
    #[serde(rename = "savedDeviceId")]
    pub(super) id: String,
    pub(super) device_history_id: String,
    pub(super) vesc_uuid: Option<String>,
    pub(super) revision: u64,
    pub(super) display_name: String,
    pub(super) custom_display_name: Option<String>,
    pub(super) advertised_name: Option<String>,
    pub(super) bluetooth_identifier: Option<String>,
    pub(super) last_seen_at: Option<String>,
    pub(super) last_connected_at: Option<String>,
    pub(super) profile_complete: bool,
    pub(super) summary: BoardHistorySummary,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LocationState {
    pub(super) status: String,
    pub(super) permission_action: Option<String>,
    pub(super) background_capture: String,
    pub(super) reason: Option<String>,
}

impl LocationState {
    pub(super) fn disabled() -> Self {
        Self {
            status: "disabled".into(),
            permission_action: Some("requestPermission".into()),
            background_capture: "unavailable".into(),
            reason: None,
        }
    }

    pub(super) fn recording_not_determined() -> Self {
        Self {
            status: "notDetermined".into(),
            permission_action: Some("requestPermission".into()),
            background_capture: "unavailable".into(),
            reason: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SessionSummary {
    pub(super) session_id: String,
    pub(super) device_history_id: Option<String>,
    pub(super) display_name: String,
    pub(super) started_at: String,
    pub(super) ended_at: Option<String>,
    pub(super) state: String,
    pub(super) integrity: String,
    pub(super) sample_count: u64,
    pub(super) event_count: u64,
    pub(super) riding_elapsed_ms: u64,
    pub(super) total_elapsed_ms: u64,
    pub(super) distance_m: Option<f64>,
    pub(super) max_speed_mps: Option<f64>,
    pub(super) max_duty_cycle: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SessionOperationState {
    pub(super) operation_id: String,
    pub(super) kind: String,
    pub(super) status: String,
    pub(super) session_id: Option<String>,
    pub(super) scope: String,
    pub(super) target_resolution_seconds: Option<u64>,
    pub(super) completed_sessions: u64,
    pub(super) eligible_sessions: Option<u64>,
    pub(super) error: Option<DocumentError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) retained_samples: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reclaimable_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DocumentError {
    pub(super) code: String,
    pub(super) message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DocumentOperationState {
    pub(super) operation_id: String,
    pub(super) kind: String,
    pub(super) status: String,
    pub(super) error: Option<DocumentError>,
    pub(super) export_display_name: Option<String>,
    pub(super) session_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ActiveConnection {
    pub(super) device_id: String,
    pub(super) name: Option<String>,
    pub(super) display_name: Option<String>,
    pub(super) state: String,
    pub(super) last_seen_at: Option<String>,
    pub(super) device_kind: Option<String>,
    pub(super) firmware: Option<FirmwareInfo>,
    pub(super) tracking_mode: String,
    pub(super) saved_device_id: Option<String>,
    pub(super) controller_connection: Option<ControllerConnection>,
    pub(super) controller_candidates: Vec<ControllerCandidate>,
    pub(super) connection_issue: Option<ConnectionIssue>,
    pub(super) bms: BmsState,
    pub(super) refloat_controls: Option<RefloatControls>,
    pub(super) refloat_configuration: Option<RefloatConfigurationSummary>,
    pub(super) controller_actions: ControllerActions,
    pub(super) active_session: Option<SessionSummary>,
    /// A complete contract projection produced from typed protocol telemetry.
    pub(super) latest_sample: Option<Value>,
    pub(super) latest_location: Option<LiveLocation>,
    pub(super) current_limits: Option<ControllerCurrentLimits>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FirmwareInfo {
    uuid: Option<String>,
    vesc_version: String,
    package_name: String,
    package_version: String,
    hardware_name: Option<String>,
    access_mode: String,
    supported: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub(super) enum ControllerConnection {
    Direct,
    Can {
        #[serde(rename = "canId")]
        can_id: u8,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ControllerCandidate {
    can_id: u8,
    uuid: Option<String>,
    hardware_name: Option<String>,
    firmware_name: Option<String>,
    vesc_version: String,
    vesc_supported: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ConnectionIssue {
    code: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BmsState {
    status: String,
    uuid: Option<String>,
    connection: Option<BmsConnectionValue>,
    candidates: Vec<BmsCandidateValue>,
    latest_snapshot: Option<BmsSnapshotValue>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum BmsConnectionValue {
    Direct,
    Can {
        #[serde(rename = "canId")]
        can_id: u8,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BmsCandidateValue {
    connection: BmsConnectionValue,
    uuid: Option<String>,
    hardware_name: Option<String>,
    vesc_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BmsSnapshotValue {
    observed_at: String,
    total_voltage_v: f64,
    charge_voltage_v: f64,
    input_current_a: f64,
    input_ic_current_a: f64,
    session_amp_hours_ah: f64,
    session_watt_hours_wh: f64,
    cell_voltages_v: Vec<f64>,
    balancing: Vec<bool>,
    temperatures_c: Vec<f64>,
    ic_temperature_c: f64,
    humidity_sensor_temperature_c: f64,
    humidity_percent: f64,
    highest_cell_temperature_c: f64,
    state_of_charge: f64,
    state_of_health: f64,
    reported_can_id: u8,
    lifetime_charge_amp_hours_ah: f64,
    lifetime_charge_watt_hours_wh: f64,
    lifetime_discharge_amp_hours_ah: f64,
    lifetime_discharge_watt_hours_wh: f64,
    pressure_pa: f64,
    data_version: Option<u8>,
    status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct RefloatControls {
    status: String,
    writable: bool,
    package_enabled: Option<bool>,
    flywheel_enabled: Option<bool>,
    lighting_available: bool,
    leds_enabled: Option<bool>,
    headlights_enabled: Option<bool>,
    fatal_error_clear_available: bool,
    fatal_error_latched: Option<bool>,
    fatal_error_active: Option<bool>,
    fatal_error_clearable: bool,
    pending_action: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RefloatConfigurationSummary {
    availability: String,
    writable: bool,
    package_version: Option<String>,
    schema_fingerprint: Option<String>,
    wire_signature: Option<u32>,
    current_loaded: bool,
    default_loaded: bool,
    pending_operation: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ControllerActions {
    shutdown_available: bool,
    pending_action: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LiveLocation {
    pub(super) latitude: f64,
    pub(super) longitude: f64,
    pub(super) accuracy_m: Option<f64>,
    pub(super) observed_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ControllerCurrentLimits {
    motor_current_max_a: f64,
    motor_current_min_a: f64,
    battery_current_max_a: f64,
    battery_current_min_a: f64,
    battery_series_cells: Option<u32>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn active_connection_contract_projection_has_one_typed_shape() {
        let value = json!({
            "deviceId": "device",
            "name": "VESC",
            "displayName": "VESC",
            "state": "ready",
            "lastSeenAt": "2026-01-01T00:00:00.000Z",
            "deviceKind": "board",
            "firmware": null,
            "trackingMode": "saved",
            "savedDeviceId": "saved",
            "controllerConnection": { "kind": "can", "canId": 42 },
            "controllerCandidates": [],
            "connectionIssue": null,
            "bms": {
                "status": "not_found",
                "connection": { "kind": "can", "canId": 12 },
                "candidates": [],
                "latestSnapshot": null
            },
            "refloatControls": null,
            "refloatConfiguration": null,
            "controllerActions": { "shutdownAvailable": false, "pendingAction": null },
            "activeSession": null,
            "latestSample": null,
            "latestLocation": null,
            "currentLimits": null
        });
        let connection: ActiveConnection = serde_json::from_value(value).expect("typed connection");
        let encoded = serde_json::to_value(connection).expect("encoded connection");
        assert_eq!(encoded["controllerConnection"]["canId"], 42);
        assert_eq!(encoded["bms"]["connection"]["canId"], 12);
    }
}
