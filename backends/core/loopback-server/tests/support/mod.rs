use float_view_loopback_server::facts::Snapshot;
use serde_json::json;
pub fn facts() -> Snapshot {
    serde_json::from_value(json!({
        "discovery":{"bluetoothState":{"radio":"powered_on","scanPermission":"granted","connectPermission":"granted"},"activeScanId":null,"knownDevices":[]},
        "active":null,"firmware":null,"packageVersion":null,
        "configuration":{"availability":"unavailable","writable":false,"packageVersion":null,"schemaFingerprint":null,"wireSignature":null,"currentLoaded":false,"defaultLoaded":false,"pendingOperation":null},
        "savedBoards":[],"settings":{"revision":0,"idleProbeIntervalMs":500,"idlePersistIntervalMs":1000,"ridingPersistIntervalMs":100,"postRideTailMs":5000,"reconnectDurationMs":300000,"minimumSessionDistance":100,"themePreference":"system","unitPreference":"metric","historyGapThresholdSeconds":null,"locationRecordingEnabled":false,"includeGpsLocationWhenExporting":false},
        "location":{"authorization":"notDetermined","backgroundCapture":"unavailable","reason":null},"maintenance":null
    })).unwrap()
}
