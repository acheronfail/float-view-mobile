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
    Metadata => "metadata", Link => "link", Foreground => "foreground", Native => "native",
    Disconnect => "disconnect", Activated => "activated", ConfigurationBootstrap => "configuration_bootstrap",
    ConfigurationFailure => "configuration_failure", ProtocolDiagnostic => "protocol_diagnostic",
    Failure => "failure", Request => "request", BleWriteStart => "ble_write_start", BleWrite => "ble_write",
    Recording => "recording", Sample => "sample", Summary => "summary", UiDelivery => "ui_delivery",
    TelemetryGap => "telemetry_gap", Discontinuity => "clock_or_sequence_discontinuity",
    RideMarker => "ride_marker", UiError => "ui_error", Bluetooth => "bluetooth", Discovery => "discovery"
});
vocabulary!(Reason {
    Manual => "manual", Configuration => "configuration_failure", Protocol => "protocol_failure",
    Connection => "connection_failure", Storage => "storage_failure", Ui => "ui_failure",
    Request => "request_delay", Telemetry => "telemetry_gap", SlowStorage => "slow_storage",
    Native => "native_queue", Reconnect => "reconnect", Marker => "ride_marker"
});
vocabulary!(Key {
    PackageMajor => "package_major", PackageMinor => "package_minor", PackagePatch => "package_patch",
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
vocabulary!(Code {
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
    UnsupportedController => "unsupported_controller", UnsupportedRefloat => "unsupported_refloat",
    NoController => "no_controller_found", Transport => "TransportSetupFailed"
});
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Datum {
    Count(u64),
    Flag(bool),
    Code(Code),
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
