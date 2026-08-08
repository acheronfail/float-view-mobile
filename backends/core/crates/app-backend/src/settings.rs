use serde_json::{Map, Value};

const KEYS: [&str; 11] = [
    "idleProbeIntervalMs",
    "idlePersistIntervalMs",
    "ridingPersistIntervalMs",
    "postRideTailMs",
    "reconnectDurationMs",
    "minimumSessionDistance",
    "themePreference",
    "unitPreference",
    "historyGapThresholdSeconds",
    "locationRecordingEnabled",
    "includeGpsLocationWhenExporting",
];

pub(super) fn apply_patch(current: &Value, patch: &Map<String, Value>) -> Option<Value> {
    if patch.is_empty() || patch.keys().any(|key| !KEYS.contains(&key.as_str())) {
        return None;
    }
    let mut next = current.clone();
    for (key, value) in patch {
        next[key] = value.clone();
    }
    validate(&next).then_some(next)
}

fn validate(settings: &Value) -> bool {
    integer(settings, "idleProbeIntervalMs", 100..=10_000)
        && integer(settings, "idlePersistIntervalMs", 100..=60_000)
        && integer(settings, "ridingPersistIntervalMs", 50..=10_000)
        && integer(settings, "postRideTailMs", 0..=300_000)
        && integer(settings, "reconnectDurationMs", 0..=3_600_000)
        && finite_number(settings, "minimumSessionDistance", 0.0..=1_000_000.0)
        && matches!(
            settings.get("themePreference").and_then(Value::as_str),
            Some("system" | "light" | "dark")
        )
        && matches!(
            settings.get("unitPreference").and_then(Value::as_str),
            Some("metric" | "imperial")
        )
        && settings
            .get("historyGapThresholdSeconds")
            .is_some_and(|value| {
                value.is_null()
                    || value.as_f64().is_some_and(|number| {
                        number.is_finite() && (0.1..=3_600.0).contains(&number)
                    })
            })
        && settings
            .get("locationRecordingEnabled")
            .is_some_and(Value::is_boolean)
        && settings
            .get("includeGpsLocationWhenExporting")
            .is_some_and(Value::is_boolean)
}

fn integer(settings: &Value, key: &str, range: std::ops::RangeInclusive<u64>) -> bool {
    settings
        .get(key)
        .and_then(Value::as_u64)
        .is_some_and(|value| range.contains(&value))
}

fn finite_number(settings: &Value, key: &str, range: std::ops::RangeInclusive<f64>) -> bool {
    settings
        .get(key)
        .and_then(Value::as_f64)
        .is_some_and(|value| value.is_finite() && range.contains(&value))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn settings() -> Value {
        json!({
            "revision": 0,
            "idleProbeIntervalMs": 500,
            "idlePersistIntervalMs": 1_000,
            "ridingPersistIntervalMs": 100,
            "postRideTailMs": 5_000,
            "reconnectDurationMs": 30_000,
            "minimumSessionDistance": 10,
            "themePreference": "system",
            "unitPreference": "metric",
            "historyGapThresholdSeconds": null,
            "locationRecordingEnabled": false,
            "includeGpsLocationWhenExporting": false
        })
    }

    #[test]
    fn patch_rejects_wrong_types_enums_ranges_and_unknown_keys() {
        for patch in [
            json!({}),
            json!({ "automaticSessionLinkingEnabled": true }),
            json!({ "idleProbeIntervalMs": 99 }),
            json!({ "idlePersistIntervalMs": 1.5 }),
            json!({ "minimumSessionDistance": -1 }),
            json!({ "themePreference": "blue" }),
            json!({ "unitPreference": false }),
            json!({ "historyGapThresholdSeconds": 3_601 }),
            json!({ "locationRecordingEnabled": "yes" }),
        ] {
            assert!(apply_patch(&settings(), patch.as_object().expect("patch")).is_none());
        }
    }

    #[test]
    fn patch_accepts_every_bounded_setting_family() {
        let patch = json!({
            "idleProbeIntervalMs": 100,
            "idlePersistIntervalMs": 60_000,
            "ridingPersistIntervalMs": 50,
            "postRideTailMs": 0,
            "reconnectDurationMs": 0,
            "minimumSessionDistance": 0,
            "themePreference": "dark",
            "unitPreference": "imperial",
            "historyGapThresholdSeconds": 0.1,
            "locationRecordingEnabled": true,
            "includeGpsLocationWhenExporting": true
        });
        assert!(apply_patch(&settings(), patch.as_object().expect("patch")).is_some());
    }
}
