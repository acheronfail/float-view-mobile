use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum UnitPreference {
    Metric,
    Imperial,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsValues {
    idle_probe_interval_ms: u64,
    idle_persist_interval_ms: u64,
    riding_persist_interval_ms: u64,
    post_ride_tail_ms: u64,
    reconnect_duration_ms: u64,
    minimum_session_distance: f64,
    theme_preference: ThemePreference,
    unit_preference: UnitPreference,
    history_gap_threshold_seconds: Option<f64>,
    location_recording_enabled: bool,
    include_gps_location_when_exporting: bool,
}

impl Default for SettingsValues {
    fn default() -> Self {
        Self {
            idle_probe_interval_ms: 500,
            idle_persist_interval_ms: 1_000,
            riding_persist_interval_ms: 100,
            post_ride_tail_ms: 5_000,
            reconnect_duration_ms: 30_000,
            minimum_session_distance: 10.0,
            theme_preference: ThemePreference::System,
            unit_preference: UnitPreference::Metric,
            history_gap_threshold_seconds: None,
            location_recording_enabled: false,
            include_gps_location_when_exporting: false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Settings {
    pub(super) revision: u64,
    #[serde(flatten)]
    values: SettingsValues,
}

impl Settings {
    pub(super) fn from_stored(revision: u64, json: &str) -> Option<Self> {
        let values: SettingsValues = serde_json::from_str(json).ok()?;
        let settings = Self { revision, values };
        settings.validate().then_some(settings)
    }

    pub(super) fn stored_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.values)
    }

    pub(super) fn apply_patch(&self, patch: SettingsPatch) -> Option<Self> {
        if patch.is_empty() {
            return None;
        }
        let mut next = self.clone();
        patch
            .idle_probe_interval_ms
            .apply_to(&mut next.values.idle_probe_interval_ms);
        patch
            .idle_persist_interval_ms
            .apply_to(&mut next.values.idle_persist_interval_ms);
        patch
            .riding_persist_interval_ms
            .apply_to(&mut next.values.riding_persist_interval_ms);
        patch
            .post_ride_tail_ms
            .apply_to(&mut next.values.post_ride_tail_ms);
        patch
            .reconnect_duration_ms
            .apply_to(&mut next.values.reconnect_duration_ms);
        patch
            .minimum_session_distance
            .apply_to(&mut next.values.minimum_session_distance);
        patch
            .theme_preference
            .apply_to(&mut next.values.theme_preference);
        patch
            .unit_preference
            .apply_to(&mut next.values.unit_preference);
        patch
            .history_gap_threshold_seconds
            .apply_to(&mut next.values.history_gap_threshold_seconds);
        patch
            .location_recording_enabled
            .apply_to(&mut next.values.location_recording_enabled);
        patch
            .include_gps_location_when_exporting
            .apply_to(&mut next.values.include_gps_location_when_exporting);
        next.validate().then_some(next)
    }

    pub(super) const fn location_recording_enabled(&self) -> bool {
        self.values.location_recording_enabled
    }

    pub(super) const fn include_gps_location_when_exporting(&self) -> bool {
        self.values.include_gps_location_when_exporting
    }

    pub(super) const fn sampling(&self) -> app_coordinator::SamplingSettings {
        app_coordinator::SamplingSettings {
            idle_probe_millis: self.values.idle_probe_interval_ms,
            idle_persist_millis: self.values.idle_persist_interval_ms,
            riding_persist_millis: self.values.riding_persist_interval_ms,
            post_ride_tail_millis: self.values.post_ride_tail_ms,
            reconnect_duration_millis: self.values.reconnect_duration_ms,
        }
    }

    fn validate(&self) -> bool {
        (100..=10_000).contains(&self.values.idle_probe_interval_ms)
            && (100..=60_000).contains(&self.values.idle_persist_interval_ms)
            && (50..=10_000).contains(&self.values.riding_persist_interval_ms)
            && self.values.post_ride_tail_ms <= 300_000
            && self.values.reconnect_duration_ms <= 3_600_000
            && self.values.minimum_session_distance.is_finite()
            && (0.0..=1_000_000.0).contains(&self.values.minimum_session_distance)
            && self
                .values
                .history_gap_threshold_seconds
                .is_none_or(|value| value.is_finite() && (0.1..=3_600.0).contains(&value))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SettingsPatch {
    #[serde(default, deserialize_with = "deserialize_patch")]
    idle_probe_interval_ms: Patch<u64>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    idle_persist_interval_ms: Patch<u64>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    riding_persist_interval_ms: Patch<u64>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    post_ride_tail_ms: Patch<u64>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    reconnect_duration_ms: Patch<u64>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    minimum_session_distance: Patch<f64>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    theme_preference: Patch<ThemePreference>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    unit_preference: Patch<UnitPreference>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    history_gap_threshold_seconds: Patch<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    location_recording_enabled: Patch<bool>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    include_gps_location_when_exporting: Patch<bool>,
}

impl SettingsPatch {
    pub(super) fn location_recording_enabled(&self) -> Option<bool> {
        self.location_recording_enabled.value().copied()
    }

    fn is_empty(&self) -> bool {
        self.idle_probe_interval_ms.is_absent()
            && self.idle_persist_interval_ms.is_absent()
            && self.riding_persist_interval_ms.is_absent()
            && self.post_ride_tail_ms.is_absent()
            && self.reconnect_duration_ms.is_absent()
            && self.minimum_session_distance.is_absent()
            && self.theme_preference.is_absent()
            && self.unit_preference.is_absent()
            && self.history_gap_threshold_seconds.is_absent()
            && self.location_recording_enabled.is_absent()
            && self.include_gps_location_when_exporting.is_absent()
    }
}

#[derive(Clone, Debug, Default)]
enum Patch<T> {
    #[default]
    Absent,
    Present(T),
}

impl<T> Patch<T> {
    const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    const fn value(&self) -> Option<&T> {
        match self {
            Self::Absent => None,
            Self::Present(value) => Some(value),
        }
    }

    fn apply_to(self, target: &mut T) {
        if let Self::Present(value) = self {
            *target = value;
        }
    }
}

fn deserialize_patch<'de, D, T>(deserializer: D) -> Result<Patch<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Patch::Present)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn patch(value: Value) -> Option<Settings> {
        let patch = serde_json::from_value(value).ok()?;
        Settings::default().apply_patch(patch)
    }

    use serde_json::Value;

    #[test]
    fn patch_rejects_wrong_types_enums_ranges_unknown_keys_and_empty_objects() {
        for value in [
            json!({}),
            json!({ "automaticSessionLinkingEnabled": true }),
            json!({ "idleProbeIntervalMs": 99 }),
            json!({ "idlePersistIntervalMs": 1.5 }),
            json!({ "minimumSessionDistance": -1 }),
            json!({ "themePreference": "blue" }),
            json!({ "unitPreference": false }),
            json!({ "historyGapThresholdSeconds": 3_601 }),
            json!({ "locationRecordingEnabled": "yes" }),
            json!({ "locationRecordingEnabled": null }),
        ] {
            assert!(patch(value).is_none());
        }
    }

    #[test]
    fn patch_accepts_every_bounded_setting_family_and_nullable_history_gap() {
        let next = patch(json!({
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
        }))
        .expect("valid patch");
        assert!(next.location_recording_enabled());

        let cleared = next
            .apply_patch(
                serde_json::from_value(json!({
                    "historyGapThresholdSeconds": null
                }))
                .expect("nullable patch"),
            )
            .expect("valid clear");
        assert_eq!(cleared.values.history_gap_threshold_seconds, None);
    }

    #[test]
    fn stored_shape_excludes_revision_and_round_trips() {
        let settings = Settings {
            revision: 7,
            ..Settings::default()
        };
        let stored = settings.stored_json().expect("stored settings");
        assert!(!stored.contains("revision"));
        assert_eq!(Settings::from_stored(7, &stored), Some(settings));
    }
}
