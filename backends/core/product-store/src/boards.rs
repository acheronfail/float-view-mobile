use crate::{Error, ProductStore, Result, identifier, json, limits, next_revision, object};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub saved_device_id: String,
    pub device_history_id: String,
    pub revision: u64,
    #[serde(skip_serializing)]
    pub local_identifier: Option<String>,
    pub advertised_name: Option<String>,
    pub custom_display_name: Option<String>,
    pub created_at_ms: i64,
    pub last_seen_at_ms: Option<i64>,
    pub last_connected_at_ms: Option<i64>,
}
impl Board {
    pub fn display_name(&self) -> &str {
        self.custom_display_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(self.advertised_name.as_deref().filter(|s| !s.is_empty()))
            .or(self.local_identifier.as_deref().filter(|s| !s.is_empty()))
            .unwrap_or(&self.device_history_id)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub saved_device_id: String,
    pub revision: u64,
    pub values: Map<String, Value>,
}
impl Profile {
    pub fn missing_fields(&self) -> Vec<&'static str> {
        ["seriesCount", "parallelCount", "motor"]
            .into_iter()
            .filter(|key| self.values.get(*key).is_none_or(Value::is_null))
            .collect()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutingHint {
    pub can_id: u8,
    pub controller_uuid: Option<[u8; 12]>,
}

/// Versioned identity of a meaningful endpoint. No legacy hash or alias path.
pub fn board_identity(uuid: Option<&[u8]>) -> Option<String> {
    let uuid =
        uuid.filter(|u| u.len() == 12 && u.iter().any(|b| *b != 0) && u.iter().any(|b| *b != 255))?;
    let mut hash = Sha256::new();
    hash.update(b"float-view-board-v1\0");
    hash.update(uuid);
    Some(format!(
        "vesc-v1-{}",
        hash.finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ))
}
impl ProductStore {
    pub fn boards(&self) -> Result<Vec<Board>> {
        let mut statement = self.catalog.prepare(&format!("{SELECT_BOARD} ORDER BY COALESCE(last_connected_at_ms,created_at_ms) DESC,saved_device_id DESC LIMIT ?"))?;
        let boards = statement
            .query_map([limits::MAX_BOARDS + 1], row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if boards.len() > limits::MAX_BOARDS {
            return Err(Error::Capacity);
        }
        Ok(boards)
    }
    pub fn board(&self, id: &str) -> Result<Board> {
        board(&self.catalog, id)
    }
    pub fn ensure_board(
        &mut self,
        local: &str,
        advertised: Option<&str>,
        uuid: Option<&[u8]>,
        at_ms: i64,
    ) -> Result<Board> {
        validate_id(local)?;
        timestamp(at_ms)?;
        let advertised = name(advertised)?;
        let identity = board_identity(uuid);
        let tx = self
            .catalog
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stable: Option<String> = match &identity {
            Some(identity) => tx
                .query_row(
                    "SELECT saved_device_id FROM saved_device WHERE device_history_id=?",
                    [identity],
                    |r| r.get(0),
                )
                .optional()?,
            None => None,
        };
        let local_match: Option<String> = tx
            .query_row(
                "SELECT saved_device_id FROM saved_device WHERE opaque_platform_id=?",
                [local],
                |r| r.get(0),
            )
            .optional()?;
        if stable.is_some() && local_match.is_some() && stable != local_match {
            // Never merge two independently owned histories as a side effect of connecting.
            return Err(Error::Conflict {
                revision: board(&tx, local_match.as_deref().ok_or(Error::Database)?)?.revision,
            });
        }
        let id = if let Some(id) = stable.or(local_match) {
            tx.execute("UPDATE saved_device SET opaque_platform_id=?,advertised_name=COALESCE(?,advertised_name),last_seen_at_ms=?,last_connected_at_ms=? WHERE saved_device_id=?",params![local,advertised,at_ms,at_ms,id])?;
            id
        } else {
            admit_board(&tx)?;
            let id = identifier()?;
            let history = identity.unwrap_or(identifier()?);
            tx.execute("INSERT INTO saved_device(saved_device_id,device_history_id,opaque_platform_id,advertised_name,revision,created_at_ms,last_seen_at_ms,last_connected_at_ms) VALUES (?,?,?,?,0,?,?,?)", params![id,history,local,advertised,at_ms,at_ms,at_ms])?;
            tx.execute("INSERT INTO board_profile(saved_device_id,revision,json,updated_at_ms) VALUES (?,0,?,?)",params![id,r#"{"preferredBmsCanId":null}"#,at_ms])?;
            id
        };
        let result = board(&tx, &id)?;
        tx.commit()?;
        Ok(result)
    }
    /// Imported history may have no local Bluetooth association. It stays private
    /// to this store and becomes connectable only after explicit discovery/save.
    pub fn import_board(
        &mut self,
        history: &str,
        display: Option<&str>,
        profile: &str,
        at_ms: i64,
    ) -> Result<Board> {
        let tx = self
            .catalog
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = import_board_on(&tx, history, display, profile, at_ms)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn update_board(
        &mut self,
        id: &str,
        expected_revision: u64,
        display: Option<&str>,
    ) -> Result<Board> {
        let display = name(display)?;
        let tx = self
            .catalog
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = board(&tx, id)?;
        check_revision(current.revision, expected_revision)?;
        tx.execute("UPDATE saved_device SET custom_display_name=?,revision=? WHERE saved_device_id=? AND revision=?",params![display,next_revision(current.revision)?,id,expected_revision])?;
        let result = board(&tx, id)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn profile(&self, id: &str) -> Result<Profile> {
        profile(&self.catalog, id)
    }
    pub fn update_profile(
        &mut self,
        id: &str,
        expected_revision: u64,
        patch: &str,
        at_ms: i64,
    ) -> Result<Profile> {
        timestamp(at_ms)?;
        let patch = object(patch, limits::MAX_PROFILE_BYTES)?;
        let tx = self
            .catalog
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut value = profile(&tx, id)?;
        check_revision(value.revision, expected_revision)?;
        let previous = value.values.get("preferredCanId").cloned();
        let changes_route = patch.contains_key("preferredCanId");
        value.values.extend(patch);
        validate_profile(&value.values)?;
        if changes_route
            && (value.values.get("preferredCanId") != previous.as_ref()
                || value
                    .values
                    .get("preferredCanId")
                    .is_none_or(Value::is_null))
        {
            tx.execute(
                "DELETE FROM controller_route_hint WHERE saved_device_id=?",
                [id],
            )?;
        }
        value.revision = next_revision(value.revision)?;
        tx.execute("UPDATE board_profile SET revision=?,json=?,updated_at_ms=? WHERE saved_device_id=? AND revision=?",params![value.revision,json(&value.values,limits::MAX_PROFILE_BYTES)?,at_ms,id,expected_revision])?;
        tx.commit()?;
        Ok(value)
    }
    pub fn routing_hint(&self, id: &str) -> Result<Option<RoutingHint>> {
        board(&self.catalog, id)?;
        let raw: Option<(u8, Option<String>)> = self
            .catalog
            .query_row(
                "SELECT can_id,controller_uuid FROM controller_route_hint WHERE saved_device_id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        raw.map(|(can_id, uuid)| {
            let controller_uuid = uuid
                .map(|s| {
                    if s.len() != 24 || !s.is_ascii() {
                        return Err(Error::Corrupt);
                    }
                    let mut bytes = [0; 12];
                    for (i, b) in bytes.iter_mut().enumerate() {
                        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                            .map_err(|_| Error::Corrupt)?;
                    }
                    Ok(bytes)
                })
                .transpose()?;
            Ok(RoutingHint {
                can_id,
                controller_uuid,
            })
        })
        .transpose()
    }
    pub fn set_routing_hint(
        &mut self,
        id: &str,
        hint: Option<&RoutingHint>,
        at_ms: i64,
    ) -> Result<()> {
        timestamp(at_ms)?;
        board(&self.catalog, id)?;
        if let Some(hint) = hint {
            if hint.can_id > 253 {
                return Err(Error::InvalidInput);
            }
            let uuid = hint
                .controller_uuid
                .filter(|v| board_identity(Some(v)).is_some())
                .map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());
            self.catalog.execute("INSERT INTO controller_route_hint(saved_device_id,can_id,controller_uuid,updated_at_ms) VALUES (?,?,?,?) ON CONFLICT(saved_device_id) DO UPDATE SET can_id=excluded.can_id,controller_uuid=excluded.controller_uuid,updated_at_ms=excluded.updated_at_ms",params![id,hint.can_id,uuid,at_ms])?;
        } else {
            self.catalog.execute(
                "DELETE FROM controller_route_hint WHERE saved_device_id=?",
                [id],
            )?;
        }
        Ok(())
    }
}
const SELECT_BOARD: &str = "SELECT saved_device_id,device_history_id,revision,opaque_platform_id,advertised_name,custom_display_name,created_at_ms,last_seen_at_ms,last_connected_at_ms FROM saved_device";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Board> {
    Ok(Board {
        saved_device_id: r.get(0)?,
        device_history_id: r.get(1)?,
        revision: r.get(2)?,
        local_identifier: r.get(3)?,
        advertised_name: r.get(4)?,
        custom_display_name: r.get(5)?,
        created_at_ms: r.get(6)?,
        last_seen_at_ms: r.get(7)?,
        last_connected_at_ms: r.get(8)?,
    })
}
fn board(db: &Connection, id: &str) -> Result<Board> {
    validate_id(id)?;
    db.query_row(
        &format!("{SELECT_BOARD} WHERE saved_device_id=?"),
        [id],
        row,
    )
    .optional()?
    .ok_or(Error::NotFound)
}
fn profile(db: &Connection, id: &str) -> Result<Profile> {
    board(db, id)?;
    let (revision, text): (u64, String) = db.query_row(
        "SELECT revision,json FROM board_profile WHERE saved_device_id=?",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let values = object(&text, limits::MAX_PROFILE_BYTES).map_err(|_| Error::Corrupt)?;
    validate_profile(&values).map_err(|_| Error::Corrupt)?;
    Ok(Profile {
        saved_device_id: id.into(),
        revision,
        values,
    })
}
pub(crate) fn admit_board(db: &Connection) -> Result<()> {
    let count: usize = db.query_row(
        "SELECT COUNT(*) FROM (SELECT 1 FROM saved_device LIMIT ?)",
        [limits::MAX_BOARDS],
        |r| r.get(0),
    )?;
    if count >= limits::MAX_BOARDS {
        return Err(Error::Capacity);
    }
    Ok(())
}
fn name(value: Option<&str>) -> Result<Option<&str>> {
    let value = value.map(str::trim).filter(|v| !v.is_empty());
    if value.is_some_and(|s| s.len() > limits::MAX_NAME_BYTES) {
        return Err(Error::InvalidInput);
    }
    Ok(value)
}
pub(crate) fn validate_id(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > limits::MAX_IDENTIFIER_BYTES || value.contains('\0') {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
pub(crate) fn validate_history(value: &str) -> Result<()> {
    validate_id(value)?;
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b))
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
pub(crate) fn timestamp(at_ms: i64) -> Result<()> {
    if at_ms < 0 {
        Err(Error::InvalidInput)
    } else {
        Ok(())
    }
}
pub(crate) fn check_revision(actual: u64, expected: u64) -> Result<()> {
    if actual != expected {
        Err(Error::Conflict { revision: actual })
    } else {
        Ok(())
    }
}
pub(crate) fn validate_profile(values: &Map<String, Value>) -> Result<()> {
    for (key, min, max) in [
        ("seriesCount", 1., 36.),
        ("parallelCount", 1., 20.),
        ("preferredCanId", 0., 253.),
        ("preferredBmsCanId", 0., 253.),
    ] {
        if let Some(value) = values.get(key).filter(|v| !v.is_null()) {
            let n = value
                .as_f64()
                .filter(|n| n.is_finite() && n.fract() == 0.)
                .ok_or(Error::InvalidInput)?;
            if n < min || n > max {
                return Err(Error::InvalidInput);
            }
        }
    }
    if let Some(value) = values.get("motor").filter(|v| !v.is_null()) {
        let s = value.as_str().ok_or(Error::InvalidInput)?;
        if s.is_empty() || s.len() > limits::MAX_NAME_BYTES {
            return Err(Error::InvalidInput);
        }
    }
    json(values, limits::MAX_PROFILE_BYTES)?;
    Ok(())
}

pub(crate) fn import_board_on(
    db: &Connection,
    history: &str,
    display: Option<&str>,
    profile: &str,
    at_ms: i64,
) -> Result<Board> {
    validate_history(history)?;
    timestamp(at_ms)?;
    let display = name(display)?;
    let values = object(profile, limits::MAX_PROFILE_BYTES)?;
    validate_profile(&values)?;
    let existing: Option<String> = db
        .query_row(
            "SELECT saved_device_id FROM saved_device WHERE device_history_id=?",
            [history],
            |r| r.get(0),
        )
        .optional()?;
    let id = if let Some(id) = existing {
        id
    } else {
        admit_board(db)?;
        let id = identifier()?;
        db.execute("INSERT INTO saved_device(saved_device_id,device_history_id,advertised_name,created_at_ms) VALUES (?,?,?,?)",params![id,history,display,at_ms])?;
        db.execute("INSERT INTO board_profile(saved_device_id,revision,json,updated_at_ms) VALUES (?,0,?,?)",params![id,json(&values,limits::MAX_PROFILE_BYTES)?,at_ms])?;
        id
    };
    board(db, &id)
}

#[derive(Clone, Debug, Default)]
pub struct BoardHistory {
    pub session_count: u64,
    pub total_riding_elapsed_ms: u64,
    pub total_distance_m: Option<f64>,
    pub most_recent_session_at_ms: Option<i64>,
}
impl ProductStore {
    pub fn board_history(&self, history: &str) -> Result<BoardHistory> {
        validate_history(history)?;
        Ok(self.catalog.query_row("SELECT COUNT(*),COALESCE(SUM(riding_elapsed_ms),0),SUM(distance_m),MAX(started_at_ms) FROM session_catalog WHERE device_history_id=? AND pending_delete=0",[history],|r|Ok(BoardHistory{session_count:r.get(0)?,total_riding_elapsed_ms:r.get(1)?,total_distance_m:r.get(2)?,most_recent_session_at_ms:r.get(3)?}))?)
    }
}
