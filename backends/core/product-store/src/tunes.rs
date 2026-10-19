use crate::{
    Error, ProductStore, Result,
    boards::{check_revision, timestamp, validate_id},
    identifier, json, limits,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use float_view_refloat_configuration_codec as codec;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const TUNE_BLACKLIST: &[&str] = &[
    "tiltback_return_speed",
    "tiltback_duty",
    "tiltback_duty_angle",
    "tiltback_duty_speed",
    "is_dutybeep_enabled",
    "tiltback_hv_angle",
    "tiltback_hv_speed",
    "tiltback_lv_angle",
    "tiltback_lv_speed",
    "persistent_fatal_error",
    "disabled",
    "fault_adc1",
    "fault_adc2",
    "hertz",
    "inputtilt_remote_type",
    "inputtilt_deadband",
    "is_footbeep_enabled",
    "is_beeper_enabled",
    "tiltback_hv",
    "tiltback_lv",
    "tiltback_speed",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TuneIdentity {
    pub package_name: String,
    pub package_version: String,
    pub schema_fingerprint: String,
    pub wire_signature: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum TuneValue {
    Double(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
}
pub type TuneValues = BTreeMap<String, TuneValue>;

#[must_use]
pub fn is_tune_field(name: &str, schema: &codec::ConfigurationSchema) -> bool {
    !name.starts_with("haptic.")
        && !name.starts_with("hardware.")
        && !TUNE_BLACKLIST.contains(&name)
        && schema.groups.iter().any(|group| {
            group.name == "General"
                && group
                    .subgroups
                    .iter()
                    .any(|subgroup| subgroup.fields.iter().any(|field| field == name))
        })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tune {
    pub configuration_id: String,
    pub revision: u64,
    pub name: String,
    pub description: Option<String>,
    pub identity: TuneIdentity,
    pub values: TuneValues,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TunePage {
    pub items: Vec<Tune>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    identity_digest: String,
    updated: i64,
    id: String,
}
impl TuneIdentity {
    pub fn validate(&self) -> Result<()> {
        if self.package_name != "refloat"
            || self.package_version.is_empty()
            || self.package_version.len() > 128
            || self.schema_fingerprint.len() != 64
            || !self
                .schema_fingerprint
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
    fn digest(&self) -> Result<String> {
        Ok(Sha256::digest(json(self, 512)?.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
}
pub fn validate_tune_values(
    identity: &TuneIdentity,
    values: &TuneValues,
    schema: &codec::ConfigurationSchema,
) -> Result<()> {
    identity.validate()?;
    let expected: BTreeSet<_> = schema
        .fields
        .iter()
        .filter(|field| is_tune_field(&field.name, schema))
        .map(|field| field.name.as_str())
        .collect();
    if identity.schema_fingerprint != schema.fingerprint
        || identity.wire_signature != schema.wire_signature
        || values.is_empty()
        || values.len() > codec::MAX_FIELDS
        || values.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected
    {
        return Err(Error::InvalidInput);
    }
    json(values, limits::MAX_TUNE_VALUES_BYTES)?;
    let mut converted: BTreeMap<_, _> = schema
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.default_value.clone()))
        .collect();
    converted.extend(values.iter().map(|(name, value)| {
        (
            name.clone(),
            match value {
                TuneValue::Double(value) => codec::ConfigurationValue::Decimal(*value),
                TuneValue::Integer(value) => codec::ConfigurationValue::Integer(*value),
                TuneValue::Boolean(value) => codec::ConfigurationValue::Boolean(*value),
                TuneValue::String(value) => codec::ConfigurationValue::String(value.clone()),
            },
        )
    }));
    let converted: Vec<_> = converted.into_iter().collect();
    codec::encode_values(&converted, schema).map_err(|_| Error::InvalidInput)?;
    Ok(())
}
impl ProductStore {
    pub fn tune(&self, id: &str) -> Result<Tune> {
        read(&self.catalog, id)
    }
    pub fn create_tune(
        &mut self,
        name: &str,
        description: Option<&str>,
        identity: &TuneIdentity,
        values: &TuneValues,
        schema: &codec::ConfigurationSchema,
        at_ms: i64,
    ) -> Result<Tune> {
        timestamp(at_ms)?;
        validate_text(name, description)?;
        validate_tune_values(identity, values, schema)?;
        let id = identifier()?;
        self.catalog.execute("INSERT INTO refloat_configuration(configuration_id,revision,name,description,package_name,package_version,schema_fingerprint,wire_signature,values_json,created_at_ms,updated_at_ms) VALUES (?,1,?,?,?,?,?,?,?,?,?)",params![id,name,description,identity.package_name,identity.package_version,identity.schema_fingerprint,identity.wire_signature,json(values,limits::MAX_TUNE_VALUES_BYTES)?,at_ms,at_ms])?;
        self.tune(&id)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "A revision-qualified complete Tune replacement is one atomic operation"
    )]
    pub fn update_tune(
        &mut self,
        id: &str,
        expected_revision: u64,
        name: &str,
        description: Option<&str>,
        values: &TuneValues,
        schema: &codec::ConfigurationSchema,
        at_ms: i64,
    ) -> Result<Tune> {
        timestamp(at_ms)?;
        validate_text(name, description)?;
        let tx = self
            .catalog
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = read(&tx, id)?;
        check_revision(existing.revision, expected_revision)?;
        if at_ms < existing.created_at_ms {
            return Err(Error::InvalidInput);
        }
        // Metadata edits preserve already validated values, even with another board schema open.
        if values != &existing.values {
            validate_tune_values(&existing.identity, values, schema)?;
        }
        tx.execute("UPDATE refloat_configuration SET revision=?,name=?,description=?,values_json=?,updated_at_ms=? WHERE configuration_id=? AND revision=?",params![crate::next_revision(expected_revision)?,name,description,json(values,limits::MAX_TUNE_VALUES_BYTES)?,at_ms,id,expected_revision])?;
        let result = read(&tx, id)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn delete_tune(&mut self, id: &str, expected_revision: u64) -> Result<()> {
        let tx = self
            .catalog
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read(&tx, id)?;
        check_revision(current.revision, expected_revision)?;
        tx.execute(
            "DELETE FROM refloat_configuration WHERE configuration_id=? AND revision=?",
            params![id, expected_revision],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn tunes(
        &self,
        identity: &TuneIdentity,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<TunePage> {
        self.tunes_filtered(Some(identity), limit, cursor)
    }
    pub fn tunes_filtered(
        &self,
        identity: Option<&TuneIdentity>,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<TunePage> {
        if let Some(identity) = identity {
            identity.validate()?;
        }
        let identity_digest = identity
            .map(TuneIdentity::digest)
            .transpose()?
            .unwrap_or_else(|| "all".into());
        if !(1..=limits::MAX_TUNE_PAGE).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        let cursor = cursor
            .map(|s| {
                if s.len() > 512 {
                    return Err(Error::InvalidInput);
                }
                let c: Cursor =
                    serde_json::from_slice(&STANDARD.decode(s).map_err(|_| Error::InvalidInput)?)
                        .map_err(|_| Error::InvalidInput)?;
                validate_id(&c.id)?;
                timestamp(c.updated)?;
                if c.identity_digest != identity_digest {
                    return Err(Error::InvalidInput);
                }
                Ok(c)
            })
            .transpose()?;
        let cursor_filter = if cursor.is_some() {
            " AND (updated_at_ms,configuration_id)<(?5,?6)"
        } else {
            ""
        };
        let mut statement = self.catalog.prepare_cached(&format!("{SELECT} WHERE (?1 IS NULL OR (package_name=?1 AND package_version=?2 AND schema_fingerprint=?3 AND wire_signature=?4)){cursor_filter} ORDER BY updated_at_ms DESC,configuration_id DESC LIMIT ?7"))?;
        let rows = statement.query_map(
            params![
                identity.map(|i| &i.package_name),
                identity.map(|i| &i.package_version),
                identity.map(|i| &i.schema_fingerprint),
                identity.map(|i| i.wire_signature),
                cursor.as_ref().map(|c| c.updated),
                cursor.as_ref().map(|c| &c.id),
                limit + 1
            ],
            row,
        )?;
        let mut items = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        let more = items.len() > limit;
        if more {
            items.pop();
        }
        let next_cursor = if more {
            items
                .last()
                .map(|t| {
                    Ok::<String, Error>(STANDARD.encode(json(
                        &Cursor {
                            identity_digest: identity_digest.clone(),
                            updated: t.updated_at_ms,
                            id: t.configuration_id.clone(),
                        },
                        384,
                    )?))
                })
                .transpose()?
        } else {
            None
        };
        Ok(TunePage { items, next_cursor })
    }
}
fn validate_text(name: &str, description: Option<&str>) -> Result<()> {
    if name.is_empty() || name.len() > 128 || description.is_some_and(|d| d.len() > 1024) {
        Err(Error::InvalidInput)
    } else {
        Ok(())
    }
}
const SELECT: &str = "SELECT configuration_id,revision,name,description,package_name,package_version,schema_fingerprint,wire_signature,values_json,created_at_ms,updated_at_ms FROM refloat_configuration";
fn read(db: &rusqlite::Connection, id: &str) -> Result<Tune> {
    validate_id(id)?;
    db.query_row(&format!("{SELECT} WHERE configuration_id=?"), [id], row)
        .optional()?
        .ok_or(Error::NotFound)
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Tune> {
    let text: String = r.get(8)?;
    let values = if text.len() <= limits::MAX_TUNE_VALUES_BYTES {
        serde_json::from_str::<TuneValues>(&text).ok()
    } else {
        None
    }
    .filter(|v| !v.is_empty() && v.len() <= codec::MAX_FIELDS)
    .ok_or(rusqlite::Error::InvalidQuery)?;
    Ok(Tune {
        configuration_id: r.get(0)?,
        revision: r.get(1)?,
        name: r.get(2)?,
        description: r.get(3)?,
        identity: TuneIdentity {
            package_name: r.get(4)?,
            package_version: r.get(5)?,
            schema_fingerprint: r.get(6)?,
            wire_signature: r.get(7)?,
        },
        values,
        created_at_ms: r.get(9)?,
        updated_at_ms: r.get(10)?,
    })
}
