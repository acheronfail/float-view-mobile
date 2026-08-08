use super::{ContractError, Map, TuneRecord, Value, invalid, iso8601_or_empty, json};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TuneIdentityDto {
    package_name: String,
    package_version: String,
    schema_fingerprint: String,
    wire_signature: u32,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "lowercase",
    deny_unknown_fields
)]
enum ConfigurationValueDto {
    Double(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
}

pub(super) fn configuration_summary_value(
    summary: &app_coordinator::ConfigurationSummary,
) -> Value {
    let identity = summary.identity.as_ref();
    json!({
        "availability": match summary.availability {
            app_coordinator::ConfigurationAvailability::Unavailable => "unavailable",
            app_coordinator::ConfigurationAvailability::Loading => "loading",
            app_coordinator::ConfigurationAvailability::Ready => "ready",
        },
        "writable": summary.writable,
        "packageVersion": identity.map(|value| value.package_version.as_str()),
        "schemaFingerprint": identity.map(|value| value.schema_fingerprint.as_str()),
        "wireSignature": identity.map(|value| value.wire_signature),
        "currentLoaded": summary.current_loaded,
        "defaultLoaded": summary.default_loaded,
        "pendingOperation": summary.pending.map(|operation| match operation {
            app_coordinator::ConfigurationOperation::Schema => "schema",
            app_coordinator::ConfigurationOperation::ReadCurrent => "readCurrent",
            app_coordinator::ConfigurationOperation::ReadDefault => "readDefault",
            app_coordinator::ConfigurationOperation::Write
            | app_coordinator::ConfigurationOperation::Verify => "write",
        }),
    })
}

pub(super) fn tune_identity_value(identity: &protocol_refloat::TuneIdentity) -> Value {
    json!({
        "packageName": "refloat",
        "packageVersion": identity.package_version,
        "schemaFingerprint": identity.schema_fingerprint,
        "wireSignature": identity.wire_signature,
    })
}

pub(super) fn parse_tune_identity(
    value: Option<&Value>,
) -> Result<protocol_refloat::TuneIdentity, ContractError> {
    let identity: TuneIdentityDto = serde_json::from_value(value.cloned().unwrap_or(Value::Null))
        .map_err(|_| invalid("Configuration identity is invalid"))?;
    if identity.package_name != "refloat"
        || identity.package_version.is_empty()
        || identity.schema_fingerprint.len() != 64
        || !identity
            .schema_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid("Configuration identity is invalid"));
    }
    Ok(protocol_refloat::TuneIdentity {
        package_version: identity.package_version,
        schema_fingerprint: identity.schema_fingerprint,
        wire_signature: identity.wire_signature,
    })
}

pub(super) fn configuration_schema_pages(
    schema: &protocol_refloat::ConfigurationSchema,
) -> Vec<(String, String, Vec<&protocol_refloat::ConfigurationField>)> {
    let by_name = schema
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field))
        .collect::<std::collections::HashMap<_, _>>();
    let mut pages = Vec::new();
    for group in &schema.groups {
        for subgroup in &group.subgroups {
            let fields = subgroup
                .fields
                .iter()
                .filter_map(|name| by_name.get(name.as_str()).copied())
                .collect::<Vec<_>>();
            if !fields.is_empty() {
                pages.push((group.name.clone(), subgroup.name.clone(), fields));
            }
        }
    }
    if pages.is_empty() && !schema.fields.is_empty() {
        pages.push((
            "General".into(),
            "General".into(),
            schema.fields.iter().collect(),
        ));
    }
    pages
}

pub(super) fn configuration_field_value(field: &protocol_refloat::ConfigurationField) -> Value {
    json!({
        "name": field.name,
        "label": field.long_name,
        "type": match field.field_type {
            protocol_refloat::ConfigurationFieldType::Double => "double",
            protocol_refloat::ConfigurationFieldType::Integer => "integer",
            protocol_refloat::ConfigurationFieldType::Enumeration => "enum",
            protocol_refloat::ConfigurationFieldType::Boolean => "boolean",
            protocol_refloat::ConfigurationFieldType::String => "string",
            protocol_refloat::ConfigurationFieldType::Bitfield => "bitfield",
        },
        "defaultValue": configuration_value(&field.default_value),
        "minimum": field.minimum,
        "maximum": field.maximum,
        "step": field.step,
        "precision": field.precision,
        "suffix": field.suffix,
        "choices": field.choices,
        "description": field.description,
    })
}

pub(super) fn configuration_value(value: &protocol_refloat::ConfigurationValue) -> Value {
    match value {
        protocol_refloat::ConfigurationValue::Double(value) => {
            json!({ "type": "double", "value": value })
        }
        protocol_refloat::ConfigurationValue::Integer(value) => {
            json!({ "type": "integer", "value": value })
        }
        protocol_refloat::ConfigurationValue::Boolean(value) => {
            json!({ "type": "boolean", "value": value })
        }
        protocol_refloat::ConfigurationValue::String(value) => {
            json!({ "type": "string", "value": value })
        }
    }
}

pub(super) fn parse_configuration_values(
    value: Option<&Value>,
) -> Result<std::collections::HashMap<String, protocol_refloat::ConfigurationValue>, ContractError>
{
    let values: std::collections::HashMap<String, ConfigurationValueDto> =
        serde_json::from_value(value.cloned().unwrap_or(Value::Null))
            .map_err(|_| invalid("Configuration values are invalid"))?;
    if values.len() > protocol_refloat::MAX_CONFIGURATION_FIELDS {
        return Err(invalid("Configuration values are invalid"));
    }
    values
        .iter()
        .map(|(name, value)| {
            let value = match value {
                ConfigurationValueDto::Double(value) if value.is_finite() => {
                    protocol_refloat::ConfigurationValue::Double(*value)
                }
                ConfigurationValueDto::Integer(value) => {
                    protocol_refloat::ConfigurationValue::Integer(*value)
                }
                ConfigurationValueDto::Boolean(value) => {
                    protocol_refloat::ConfigurationValue::Boolean(*value)
                }
                ConfigurationValueDto::String(value)
                    if value.len() <= protocol_refloat::MAX_STRING_VALUE_BYTES =>
                {
                    protocol_refloat::ConfigurationValue::String(value.clone())
                }
                ConfigurationValueDto::Double(_) | ConfigurationValueDto::String(_) => {
                    return Err(invalid("Configuration value is invalid"));
                }
            };
            Ok((name.clone(), value))
        })
        .collect()
}

pub(super) fn configuration_values_value(
    identity: &protocol_refloat::TuneIdentity,
    revision: u64,
    defaults: bool,
    values: &std::collections::HashMap<String, protocol_refloat::ConfigurationValue>,
) -> Value {
    json!({
        "identity": tune_identity_value(identity),
        "revision": revision,
        "source": if defaults { "default" } else { "current" },
        "values": values.iter().map(|(name, value)| (name.clone(), configuration_value(value))).collect::<Map<_, _>>(),
    })
}

pub(super) fn encode_configuration_values_json(
    values: &std::collections::HashMap<String, protocol_refloat::ConfigurationValue>,
) -> Result<String, ContractError> {
    serde_json::to_string(
        &values
            .iter()
            .map(|(name, value)| (name.clone(), configuration_value(value)))
            .collect::<Map<_, _>>(),
    )
    .map_err(|_| ContractError::new("internal", "Configuration encoding failed"))
}

pub(super) fn tune_record_identity(record: &TuneRecord) -> protocol_refloat::TuneIdentity {
    protocol_refloat::TuneIdentity {
        package_version: record.package_version.clone(),
        schema_fingerprint: record.schema_fingerprint.clone(),
        wire_signature: record.wire_signature,
    }
}

pub(super) fn tune_summary_value(record: &TuneRecord) -> Value {
    json!({
        "tuneId": record.configuration_id,
        "revision": record.revision,
        "name": record.name,
        "description": record.description,
        "identity": tune_identity_value(&tune_record_identity(record)),
        "createdAt": iso8601_or_empty(record.created_at_millis),
        "updatedAt": iso8601_or_empty(record.updated_at_millis),
    })
}

pub(super) fn tune_value(record: &TuneRecord) -> Value {
    let mut value = tune_summary_value(record);
    value["values"] = serde_json::from_str(&record.values_json).unwrap_or_else(|_| json!({}));
    value
}

pub(super) fn tune_name(value: Option<&Value>) -> Result<String, ContractError> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(str::to_owned)
        .ok_or_else(|| invalid("Tune name is invalid"))
}

pub(super) fn tune_description(value: Option<&Value>) -> Result<Option<String>, ContractError> {
    match value {
        Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value.len() <= 1024 => Ok(Some(value.clone())),
        _ => Err(invalid("Tune description is invalid")),
    }
}
