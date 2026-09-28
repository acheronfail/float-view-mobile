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
use std::collections::BTreeMap;

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
    if identity.schema_fingerprint != schema.fingerprint
        || identity.wire_signature != schema.wire_signature
        || values.is_empty()
        || values.len() > codec::MAX_FIELDS
    {
        return Err(Error::InvalidInput);
    }
    json(values, limits::MAX_TUNE_VALUES_BYTES)?;
    let converted: Vec<_> = values
        .iter()
        .map(|(name, v)| {
            (
                name.clone(),
                match v {
                    TuneValue::Double(v) => codec::ConfigurationValue::Decimal(*v),
                    TuneValue::Integer(v) => codec::ConfigurationValue::Integer(*v),
                    TuneValue::Boolean(v) => codec::ConfigurationValue::Boolean(*v),
                    TuneValue::String(v) => codec::ConfigurationValue::String(v.clone()),
                },
            )
        })
        .collect();
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
        validate_tune_values(&existing.identity, values, schema)?;
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
        identity.validate()?;
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
                if c.identity_digest != identity.digest()? {
                    return Err(Error::InvalidInput);
                }
                Ok(c)
            })
            .transpose()?;
        let mut statement=self.catalog.prepare(&format!("{SELECT} WHERE package_name=? AND package_version=? AND schema_fingerprint=? AND wire_signature=? AND (? IS NULL OR updated_at_ms<? OR (updated_at_ms=? AND configuration_id<?)) ORDER BY updated_at_ms DESC,configuration_id DESC LIMIT ?"))?;
        let updated = cursor.as_ref().map(|c| c.updated);
        let id = cursor.as_ref().map(|c| &c.id);
        let rows = statement.query_map(
            params![
                identity.package_name,
                identity.package_version,
                identity.schema_fingerprint,
                identity.wire_signature,
                updated,
                updated,
                updated,
                id,
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
                            identity_digest: identity.digest()?,
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
