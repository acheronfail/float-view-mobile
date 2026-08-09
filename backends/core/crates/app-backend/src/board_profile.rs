use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

const MAX_PROFILE_FIELDS: usize = 128;
const MAX_PROFILE_FIELD_NAME_BYTES: usize = 128;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BoardProfileValues {
    #[serde(skip_serializing_if = "Option::is_none")]
    series_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    motor: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_nullable",
        skip_serializing_if = "Nullable::is_missing"
    )]
    preferred_can_id: Nullable<u8>,
    #[serde(
        default,
        deserialize_with = "deserialize_nullable",
        skip_serializing_if = "Nullable::is_missing"
    )]
    preferred_bms_can_id: Nullable<u8>,
    #[serde(flatten)]
    extensions: Map<String, Value>,
}

impl BoardProfileValues {
    pub(super) fn preferred_can_id(&self) -> Option<u8> {
        self.preferred_can_id
            .value()
            .and_then(Option::as_ref)
            .copied()
    }

    fn validate(&self) -> bool {
        self.preferred_can_id().is_none_or(|value| value <= 253)
            && self
                .preferred_bms_can_id
                .value()
                .and_then(Option::as_ref)
                .is_none_or(|value| *value <= 253)
            && self.extensions.len() <= MAX_PROFILE_FIELDS
            && self
                .extensions
                .keys()
                .all(|key| !key.is_empty() && key.len() <= MAX_PROFILE_FIELD_NAME_BYTES)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BoardProfile {
    pub(super) saved_device_id: String,
    pub(super) revision: u64,
    pub(super) values: BoardProfileValues,
    pub(super) missing_fields: Vec<String>,
}

impl BoardProfile {
    pub(super) fn from_stored(profile: infra_sqlite_storage::BoardProfile) -> Option<Self> {
        let values: BoardProfileValues = serde_json::from_str(&profile.json).ok()?;
        if !values.validate() {
            return None;
        }
        let missing_fields = Self::missing_fields(&values);
        Some(Self {
            saved_device_id: profile.saved_device_id,
            revision: profile.revision,
            values,
            missing_fields,
        })
    }

    pub(super) fn apply_patch(&mut self, patch: BoardProfilePatch) -> bool {
        if patch.is_empty() {
            return false;
        }
        patch
            .series_count
            .apply_optional(&mut self.values.series_count);
        patch
            .parallel_count
            .apply_optional(&mut self.values.parallel_count);
        patch.motor.apply_optional(&mut self.values.motor);
        patch
            .preferred_can_id
            .apply_nullable(&mut self.values.preferred_can_id);
        patch
            .preferred_bms_can_id
            .apply_nullable(&mut self.values.preferred_bms_can_id);
        self.values.extensions.extend(patch.extensions);
        self.missing_fields = Self::missing_fields(&self.values);
        self.values.validate()
    }

    pub(super) fn values_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.values)
    }

    pub(super) fn complete(&self) -> bool {
        self.missing_fields.is_empty()
    }

    fn missing_fields(values: &BoardProfileValues) -> Vec<String> {
        [
            ("seriesCount", values.series_count.is_some()),
            ("parallelCount", values.parallel_count.is_some()),
            ("motor", values.motor.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, present)| (!present).then_some(name.to_owned()))
        .collect()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BoardProfilePatch {
    #[serde(default, deserialize_with = "deserialize_patch")]
    series_count: Patch<u32>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    parallel_count: Patch<u32>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    motor: Patch<String>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    preferred_can_id: Patch<Option<u8>>,
    #[serde(default, deserialize_with = "deserialize_patch")]
    preferred_bms_can_id: Patch<Option<u8>>,
    #[serde(flatten)]
    extensions: Map<String, Value>,
}

impl BoardProfilePatch {
    fn is_empty(&self) -> bool {
        self.series_count.is_absent()
            && self.parallel_count.is_absent()
            && self.motor.is_absent()
            && self.preferred_can_id.is_absent()
            && self.preferred_bms_can_id.is_absent()
            && self.extensions.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
enum Nullable<T> {
    #[default]
    Missing,
    Present(Option<T>),
}

impl<T> Nullable<T> {
    const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }

    const fn value(&self) -> Option<&Option<T>> {
        match self {
            Self::Missing => None,
            Self::Present(value) => Some(value),
        }
    }
}

impl<T: Serialize> Serialize for Nullable<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Missing | Self::Present(None) => serializer.serialize_none(),
            Self::Present(Some(value)) => value.serialize(serializer),
        }
    }
}

fn deserialize_nullable<'de, D, T>(deserializer: D) -> Result<Nullable<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Nullable::Present)
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

    fn apply_optional(self, target: &mut Option<T>) {
        if let Self::Present(value) = self {
            *target = Some(value);
        }
    }
}

impl<T> Patch<Option<T>> {
    fn apply_nullable(self, target: &mut Nullable<T>) {
        if let Self::Present(value) = self {
            *target = Nullable::Present(value);
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

    fn stored(json: &str) -> infra_sqlite_storage::BoardProfile {
        infra_sqlite_storage::BoardProfile {
            saved_device_id: "saved".into(),
            revision: 3,
            json: json.into(),
            updated_at_millis: 0,
        }
    }

    #[test]
    fn known_fields_are_typed_and_unknown_fields_round_trip() {
        let profile = BoardProfile::from_stored(stored(
            r#"{"seriesCount":18,"parallelCount":2,"motor":"superflux","preferredCanId":42,"future":{"enabled":true}}"#,
        ))
        .expect("profile");
        assert_eq!(profile.values.preferred_can_id(), Some(42));
        assert!(profile.complete());
        let encoded: Value =
            serde_json::from_str(&profile.values_json().expect("json")).expect("value");
        assert_eq!(encoded["future"], json!({ "enabled": true }));
    }

    #[test]
    fn nullable_known_fields_distinguish_clear_from_absence() {
        let mut profile =
            BoardProfile::from_stored(stored(r#"{"preferredCanId":42}"#)).expect("profile");
        let patch: BoardProfilePatch =
            serde_json::from_value(json!({ "preferredCanId": null })).expect("patch");
        assert!(profile.apply_patch(patch));
        let encoded: Value =
            serde_json::from_str(&profile.values_json().expect("json")).expect("value");
        assert!(encoded["preferredCanId"].is_null());
    }

    #[test]
    fn invalid_known_fields_are_rejected() {
        assert!(BoardProfile::from_stored(stored(r#"{"preferredCanId":254}"#)).is_none());
        assert!(
            serde_json::from_value::<BoardProfilePatch>(json!({
                "preferredCanId": "42"
            }))
            .is_err()
        );
    }
}
