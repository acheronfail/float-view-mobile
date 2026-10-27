//! Closed diagnostic vocabulary: never serialize peer text or Error objects.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_EVENT: usize = 1024;
pub const MAX_EVENTS: usize = 256;
pub const MAX_INCIDENTS: usize = 3;
pub const MAX_INCIDENT_BYTES: usize = 1_048_576;
pub const RETENTION_MS: i64 = 7 * 86_400_000;
pub const REPORT_TTL: u64 = 600_000;
pub const MAX_REPORT_BYTES: u64 = 5 * 1_048_576;

macro_rules! vocabulary {
    ($name:ident { $($variant:ident => $wire:literal),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
        pub enum $name { $(#[serde(rename=$wire)] $variant),* }
        impl $name {
            pub fn parse(s: &str) -> Option<Self> { match s { $($wire => Some(Self::$variant),)* _ => None } }
        }
    }
}
vocabulary!(Kind {
    ApiRequestFailure => "backend_request_failure", Metadata => "metadata", Link => "link", Foreground => "foreground", Native => "native",
    Disconnect => "disconnect", Activated => "activated", ConfigurationBootstrap => "configuration_bootstrap",
    ConfigurationFailure => "configuration_failure", ProtocolDiagnostic => "protocol_diagnostic",
    Failure => "failure", Request => "request", BleWriteStart => "ble_write_start", BleWrite => "ble_write",
    Recording => "recording", Sample => "sample", Summary => "summary", UiDelivery => "ui_delivery",
    TelemetryGap => "telemetry_gap", Discontinuity => "clock_or_sequence_discontinuity",
    RideMarker => "ride_marker", UiError => "ui_error", Bluetooth => "bluetooth", Discovery => "discovery"
});
vocabulary!(Reason {
    BackendRequest => "backend_request_failure", Manual => "manual", Configuration => "configuration_failure", Protocol => "protocol_failure",
    Connection => "connection_failure", Storage => "storage_failure", Ui => "ui_failure",
    Request => "request_delay", Telemetry => "telemetry_gap", SlowStorage => "slow_storage",
    Native => "native_queue", Reconnect => "reconnect", Marker => "ride_marker"
});
vocabulary!(Key {
    ApplicationGeneration => "application_generation", Method => "method", PackageMajor => "package_major", PackageMinor => "package_minor", PackagePatch => "package_patch",
    Metric => "metric", Percentiles => "count_p50_p95_max", OsMajor => "os_major", OsMinor => "os_minor", OsPatch => "os_patch", RequestKind => "kind",
    Hardware => "hardware", Route => "route", Generation => "generation", Sequence => "sequence", PreviousSequence => "previous_sequence",
    Operation => "operation", Bytes => "bytes", SinceStart => "since_start_ms", Queue => "queue_ms",
    Response => "response_ms", Interval => "interval_ms", Elapsed => "elapsed_ms",
    PreviousElapsed => "previous_elapsed_ms", SampleElapsed => "sample_elapsed_ms",
    Foreground => "foreground", NewSession => "new_session", CachedSchema => "cached_schema",
    ConfigCount => "custom_configuration_count", Major => "firmware_major", Minor => "firmware_minor",
    RidingPoll => "riding_poll_ms", IdlePoll => "idle_poll_ms", RidingPersist => "riding_persist_ms",
    IdlePersist => "idle_persist_ms", Callbacks => "callbacks", QueueDelay => "max_queue_delay_ms",
    QueueDepth => "max_queue_depth", Overflows => "overflows", ExchangeFailures => "exchange_failures",
    Snapshots => "snapshots", SampleAge => "max_sample_age_ms", DeliveryGap => "max_delivery_gap_ms",
    Handler => "max_handler_ms", Accepted => "accepted", Stored => "stored", Window => "window_ms",
    Dropped => "dropped_records", State => "state", Progress => "progress", Outcome => "outcome",
    Platform => "platform", Code => "code", Feature => "feature", Category => "category",
    Line => "line", Column => "column", Radio => "radio", Scan => "scan", Connect => "connect"
});
vocabulary!(ApiMethod {
    Unknown => "unknown",
    ApiStateGet => "state.get",
    ApiUiStateGet => "uiState.get",
    ApiUiStateSet => "uiState.set",
    ApiPermissionsRecover => "permissions.recover",
    ApiBluetoothRequestPermission => "bluetooth.requestPermission",
    ApiBluetoothScan => "bluetooth.scan",
    ApiLocationRequestPermission => "location.requestPermission",
    ApiLocationOpenSettings => "location.openSettings",
    ApiDeviceConnect => "device.connect",
    ApiDeviceSelectController => "device.selectController",
    ApiDeviceDisconnect => "device.disconnect",
    ApiControllerShutdown => "controller.shutdown",
    ApiRefloatControlSet => "refloat.control.set",
    ApiRefloatFatalErrorClear => "refloat.fatalError.clear",
    ApiRefloatConfigurationDraftGet => "refloat.configuration.draft.get",
    ApiRefloatConfigurationDraftSet => "refloat.configuration.draft.set",
    ApiRefloatConfigurationOpen => "refloat.configuration.open",
    ApiRefloatConfigurationSchema => "refloat.configuration.schema",
    ApiRefloatConfigurationRead => "refloat.configuration.read",
    ApiRefloatConfigurationWrite => "refloat.configuration.write",
    ApiRefloatTuneList => "refloat.tune.list",
    ApiRefloatTuneGet => "refloat.tune.get",
    ApiRefloatTuneCreate => "refloat.tune.create",
    ApiRefloatTuneUpdate => "refloat.tune.update",
    ApiRefloatTuneDelete => "refloat.tune.delete",
    ApiRefloatTuneApply => "refloat.tune.apply",
    ApiSavedDeviceUpdate => "savedDevice.update",
    ApiSavedDeviceDelete => "savedDevice.delete",
    ApiAnnouncementsOpenVoiceSettings => "announcements.openVoiceSettings",
    ApiAnnouncementsTest => "announcements.test",
    ApiSettingsUpdate => "settings.update",
    ApiBoardProfileGet => "boardProfile.get",
    ApiBoardProfileUpdate => "boardProfile.update",
    ApiSessionList => "session.list",
    ApiSessionMarkerAdd => "session.marker.add",
    ApiSessionEnd => "session.end",
    ApiSessionExtendPause => "session.extendPause",
    ApiSessionGet => "session.get",
    ApiSessionExport => "session.export",
    ApiSessionDelete => "session.delete",
    ApiSessionDeleteCancel => "session.delete.cancel",
    ApiSessionPrunePreview => "session.prune.preview",
    ApiSessionPruneStart => "session.prune.start",
    ApiSessionPruneCancel => "session.prune.cancel",
    ApiHttpSessionData => "http.sessionData",
    ApiHttpSessionImport => "http.sessionImport",
    ApiHttpSessionExport => "http.sessionExport",
});
vocabulary!(Code {
    SentOnly => "sent_only", QueueFull => "queue_full", StaleRequest => "stale",
    ApiInvalidRequest => "invalid_request",
    ApiInvalidState => "invalid_state",
    ApiPermissionRequired => "permission_required",
    ApiNotFound => "not_found",
    ApiConflict => "conflict",
    ApiUnavailable => "unavailable",
    ApiInternal => "internal",

    Fast => "fast", Slow => "slow", Fault => "fault", Telemetry => "telemetry", Lighting => "lighting", Bms => "bms", Limits => "limits", DiscoveryControl => "discovery_or_control",
    TimerLateness => "timer_lateness_ms", NotificationBytes => "notification_bytes", AppendUs => "append_us", CatalogUs => "catalog_us", RuntimeUs => "runtime_us", SnapshotUs => "snapshot_us", SampleInterval => "sample_interval_ms", Gap500 => "gaps_over_500_ms", Gap1000 => "gaps_over_1000_ms", RequestQueue => "request_queue_ms", RequestResponse => "request_response_ms", BleWriteMs => "ble_write_ms",
    InvalidState => "InvalidState", Unavailable => "Unavailable", Stale => "StaleResponse", Conflict => "Conflict", Codec => "Codec", StaleEvent => "StaleEvent", UnexpectedResponse => "UnexpectedResponse", Scheduler => "Scheduler", EnqueueRejected => "EnqueueRejected", ConfigurationStale => "ConfigurationStale", Mutation => "Mutation",
    TooManyCan => "too_many_can_devices", TooManyControllers => "too_many_controllers", PreferredUnavailable => "preferred_controller_unavailable", ControllerChanged => "controller_changed",
    Internal => "Internal", Thor300 => "THOR300", Express => "VESC Express T", Vbms => "VBMS", Vbms32 => "VBMS32",
    Unknown => "unknown", Connecting => "connecting", Services => "discoveringServices",
    Notifications => "enablingNotifications", Ready => "ready", Disconnected => "disconnected",
    Failed => "failed", Submitted => "submitted", Completed => "completed", Sent => "sent",
    Timeout => "timeout", Cancelled => "cancelled", Response => "response",
    Android => "android", Ios => "ios", Macos => "macos", Direct => "direct", Can => "can",
    PoweredOn => "poweredOn", PoweredOff => "poweredOff", Unauthorized => "unauthorized",
    Granted => "granted", Denied => "denied", NotDetermined => "notDetermined", NotRequired => "notRequired",
    Restricted => "restricted", Unsupported => "unsupported", Resetting => "resetting",
    Exception => "exception", Rejection => "rejection", Protocol => "protocol", Request => "request",
    App => "app", Connection => "connection", Configuration => "configuration", Sessions => "sessions",
    Storage => "storage", Schema => "UnauditedSchema", Malformed => "MalformedResponse",
    Limit => "StructuralLimit", IdentificationTimeout => "identification_timeout",
    UnsupportedController => "unsupported_controller", UnsupportedRefloat => "unsupported_refloat", RefloatInfoInvalid => "refloat_info_invalid",
    NoController => "no_controller_found", Transport => "TransportSetupFailed"
});
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Datum {
    Count(u64),
    Flag(bool),
    Code(Code),
    Method(ApiMethod),
    Summary([u64; 4]),
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    #[serde(default)]
    pub run: u64,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub kind: Kind,
    pub facts: BTreeMap<Key, Datum>,
}
impl Record {
    pub fn valid(&self) -> bool {
        self.facts.len() <= 32 && serde_json::to_vec(self).is_ok_and(|b| b.len() <= MAX_EVENT)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Incident {
    pub id: u64,
    pub created_ms: i64,
    pub reason: Reason,
    pub complete: bool,
    pub clipped: bool,
    pub occurrences: u64,
    pub events: Vec<Record>,
    #[serde(default)]
    pub context: Vec<Record>,
}
impl Incident {
    pub fn valid(&self) -> bool {
        self.id > 0
            && self.created_ms >= 0
            && self.created_ms <= 253_402_300_799_999
            && self.occurrences > 0
            && self.occurrences <= 9_007_199_254_740_991
            && self.context.len() <= 3
            && self.context.iter().all(Record::valid)
            && self.events.len() <= 768
            && self.events.iter().all(Record::valid)
            && serde_json::to_vec(self).is_ok_and(|b| b.len() <= MAX_INCIDENT_BYTES)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UiError {
    pub category: UiCategory,
    pub feature: Feature,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UiCategory {
    Exception,
    Rejection,
    Protocol,
    Request,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Feature {
    App,
    Connection,
    Configuration,
    Sessions,
    Storage,
}
