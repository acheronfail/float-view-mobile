//! Versioned portable app data. Validation runs off-owner; catalog attachment is atomic.
use super::*;
use crate::{Settings, Tune, TuneValue, settings};
use rusqlite::OptionalExtension;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Seek, SeekFrom},
    path::PathBuf,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AppManifest {
    format: String,
    version: u32,
    settings: Settings,
    auto_connect_history_id: Option<String>,
    boards: Vec<TransferBoard>,
    tunes: Vec<Tune>,
    sessions: Vec<TransferSession>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TransferBoard {
    device_history_id: String,
    name: String,
    custom_name: Option<String>,
    created_at_ms: i64,
    profile: Map<String, Value>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TransferSession {
    session_id: String,
    device_history_id: String,
    entry: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppImportSummary {
    pub boards_added: usize,
    pub boards_skipped: usize,
    pub sessions_added: usize,
    pub sessions_skipped: usize,
    pub tunes_added: usize,
    pub tunes_skipped: usize,
    pub settings_imported: bool,
}
struct PendingSession {
    path: PathBuf,
    manifest: Manifest,
    marker: CommitMarker,
    owned: bool,
}
impl Drop for PendingSession {
    fn drop(&mut self) {
        if self.owned {
            let _ = crate::sessions::remove_session_files(&self.path);
        }
    }
}
pub struct AppImport {
    input: StagedFile,
    manifest: AppManifest,
    sessions: Vec<PendingSession>,
    apply_settings: bool,
}

fn validate_tune(tune: &Tune) -> Result<()> {
    boards::validate_id(&tune.configuration_id)?;
    tune.identity.validate()?;
    if tune.name.trim().is_empty()
        || tune.name.len() > 128
        || tune.description.as_ref().is_some_and(|v| v.len() > 1024)
        || tune.created_at_ms < 0
        || tune.updated_at_ms < tune.created_at_ms
        || tune.values.is_empty()
        || tune.values.len() > 512
    {
        return Err(Error::InvalidInput);
    }
    for (name, value) in &tune.values {
        if name.is_empty()
            || name.len() > 128
            || match value {
                TuneValue::Double(v) => !v.is_finite(),
                TuneValue::Integer(v) => v.unsigned_abs() > limits::MAX_REVISION,
                TuneValue::String(v) => v.len() > 4096,
                TuneValue::Boolean(_) => false,
            }
        {
            return Err(Error::InvalidInput);
        }
    }
    json(&tune.values, limits::MAX_TUNE_VALUES_BYTES)?;
    Ok(())
}
impl AppManifest {
    fn validate(&self) -> Result<()> {
        if self.format != "float-view-app-transfer"
            || self.version != 1
            || self.boards.len() > limits::MAX_BOARDS
            || self.sessions.len() > limits::MAX_BOARD_EXPORT_SESSIONS
            || self.tunes.len() > limits::MAX_APP_TUNES
            || self.settings.auto_connect_saved_device_id.is_some()
        {
            return Err(Error::InvalidInput);
        }
        settings::validate_imported(&self.settings)?;
        let mut boards_seen = BTreeSet::new();
        for board in &self.boards {
            boards::validate_history(&board.device_history_id)?;
            if !boards_seen.insert(&board.device_history_id)
                || board.name.is_empty()
                || board.name.len() > 128
                || board
                    .custom_name
                    .as_ref()
                    .is_some_and(|v| v.is_empty() || v.len() > 128)
                || board.created_at_ms < 0
            {
                return Err(Error::InvalidInput);
            }
            json(&board.profile, limits::MAX_PROFILE_BYTES)?;
            boards::validate_profile(&board.profile)?;
        }
        if self
            .auto_connect_history_id
            .as_ref()
            .is_some_and(|id| !boards_seen.contains(id))
        {
            return Err(Error::InvalidInput);
        }
        let mut ids = BTreeSet::new();
        for (index, session) in self.sessions.iter().enumerate() {
            if !root::is_uuid(&session.session_id)
                || !ids.insert(&session.session_id)
                || !boards_seen.contains(&session.device_history_id)
                || session.entry != format!("sessions/{:05}.floatview", index + 1)
            {
                return Err(Error::InvalidInput);
            }
        }
        let mut tunes_seen = BTreeSet::new();
        for tune in &self.tunes {
            validate_tune(tune)?;
            if !tunes_seen.insert(&tune.configuration_id) {
                return Err(Error::InvalidInput);
            }
        }
        Ok(())
    }
}
impl ProductStore {
    pub fn require_app_idle(&self) -> Result<()> {
        if self.catalog.query_row(
            "SELECT EXISTS(SELECT 1 FROM session_catalog WHERE state='active')",
            [],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(Error::ActiveSession);
        }
        Ok(())
    }
    pub fn prepare_app_export(&self) -> Result<BoardExport> {
        self.require_app_idle()?;
        let boards = self.boards()?;
        let mut settings = self.settings()?;
        let auto_connect_history_id = boards
            .iter()
            .find(|b| Some(&b.saved_device_id) == settings.auto_connect_saved_device_id.as_ref())
            .map(|b| b.device_history_id.clone());
        settings.auto_connect_saved_device_id = None;
        settings.revision = 0;
        settings.safety_disclaimer_accepted = false;
        let mut manifest = AppManifest {
            format: "float-view-app-transfer".into(),
            version: 1,
            settings,
            auto_connect_history_id,
            boards: Vec::new(),
            tunes: Vec::new(),
            sessions: Vec::new(),
        };
        for b in boards {
            let profile = self.profile(&b.saved_device_id)?;
            let name = b
                .custom_display_name
                .as_ref()
                .or(b.advertised_name.as_ref())
                .cloned()
                .unwrap_or_else(|| b.device_history_id.clone());
            manifest.boards.push(TransferBoard {
                device_history_id: b.device_history_id,
                name,
                custom_name: b.custom_display_name,
                created_at_ms: b.created_at_ms,
                profile: profile.values,
            });
        }
        let ids=self.catalog.prepare("SELECT configuration_id FROM refloat_configuration ORDER BY configuration_id LIMIT ?")?.query_map([limits::MAX_APP_TUNES+1],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;
        if ids.len() > limits::MAX_APP_TUNES {
            return Err(Error::Capacity);
        }
        for id in ids {
            manifest.tunes.push(self.tune(&id)?);
        }
        let ids=self.catalog.prepare("SELECT session_id FROM session_catalog WHERE pending_delete=0 AND state!='active' ORDER BY started_at_ms,session_id LIMIT ?")?.query_map([limits::MAX_BOARD_EXPORT_SESSIONS+1],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;
        if ids.len() > limits::MAX_BOARD_EXPORT_SESSIONS {
            return Err(Error::Capacity);
        }
        let mut sessions = Vec::new();
        let mut size = 0u64;
        let mut largest = 0;
        for (index, id) in ids.iter().enumerate() {
            let session = self.prepare_export_source(id, ExportFormat::Archive, true)?;
            let bytes = session.source.path().metadata()?.len();
            size = size.saturating_add(bytes);
            largest = largest.max(bytes);
            manifest.sessions.push(TransferSession {
                session_id: id.clone(),
                device_history_id: session.manifest.device_history_id.clone(),
                entry: format!("sessions/{:05}.floatview", index + 1),
            });
            sessions.push(session);
        }
        manifest.validate()?;
        let bytes = json(&manifest, limits::MAX_APP_MANIFEST_BYTES)?.into_bytes();
        require_export_space(
            &self.root,
            size.saturating_mul(2)
                .min(limits::MAX_BOARD_EXPORT_BYTES)
                .saturating_add(largest * 2)
                .saturating_add(bytes.len() as u64)
                .saturating_add(limits::EXPORT_FREE_SPACE_HEADROOM),
        )?;
        Ok(BoardExport {
            app_manifest: Some(bytes),
            sessions,
            stage: self.stage(true, limits::MAX_BOARD_EXPORT_BYTES)?,
            scratch: self.stage(true, limits::MAX_EXPORT_BYTES * 2)?,
        })
    }
    pub fn attach_app_import(&mut self, mut import: AppImport) -> Result<AppImportSummary> {
        self.require_app_idle()?;
        if import.input.root != self.root || !Arc::ptr_eq(&import.input.budget, &self.file_budget) {
            return Err(Error::InvalidInput);
        }
        let revision = crate::next_revision(self.catalog_revision)?;
        let tx = self
            .catalog
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut summary = AppImportSummary::default();
        let mut board_ids = BTreeMap::new();
        for board in &import.manifest.boards {
            let existing: Option<String> = tx
                .query_row(
                    "SELECT saved_device_id FROM saved_device WHERE device_history_id=?",
                    [&board.device_history_id],
                    |r| r.get(0),
                )
                .optional()?;
            let id = if let Some(id) = existing {
                summary.boards_skipped += 1;
                id
            } else {
                let b = boards::import_board_on(
                    &tx,
                    &board.device_history_id,
                    Some(&board.name),
                    &json(&board.profile, limits::MAX_PROFILE_BYTES)?,
                    board.created_at_ms,
                )?;
                tx.execute(
                    "UPDATE saved_device SET custom_display_name=? WHERE saved_device_id=?",
                    params![board.custom_name, b.saved_device_id],
                )?;
                summary.boards_added += 1;
                b.saved_device_id
            };
            board_ids.insert(board.device_history_id.clone(), id);
        }
        for tune in &import.manifest.tunes {
            if tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM refloat_configuration WHERE configuration_id=?)",
                [&tune.configuration_id],
                |r| r.get::<_, bool>(0),
            )? {
                summary.tunes_skipped += 1;
                continue;
            }
            tx.execute("INSERT INTO refloat_configuration(configuration_id,revision,name,description,package_name,package_version,schema_fingerprint,wire_signature,values_json,created_at_ms,updated_at_ms) VALUES (?,1,?,?,?,?,?,?,?,?,?)",params![tune.configuration_id,tune.name,tune.description,tune.identity.package_name,tune.identity.package_version,tune.identity.schema_fingerprint,tune.identity.wire_signature,json(&tune.values,limits::MAX_TUNE_VALUES_BYTES)?,tune.created_at_ms,tune.updated_at_ms])?;
            summary.tunes_added += 1;
        }
        for session in &mut import.sessions {
            let m = &session.manifest;
            let existing:Option<(String,bool)>=tx.query_row("SELECT device_history_id,pending_delete FROM session_catalog WHERE session_id=?",[&m.session_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            if let Some((history, pending)) = existing {
                if pending || history != m.device_history_id {
                    return Err(Error::InvalidInput);
                }
                summary.sessions_skipped += 1;
                continue;
            }
            let file = format!("session-{}.sqlite", m.session_id);
            let destination = self.root.join(&file);
            if destination.try_exists()? {
                return Err(Error::Capacity);
            }
            root::require_kind(&session.path, false)?;
            fs::rename(&session.path, &destination)?;
            session.path = destination;
            attach_session_row(
                &tx,
                &m.session_id,
                &identifier()?,
                &file,
                m,
                &session.marker,
            )?;
            summary.sessions_added += 1;
        }
        if import.apply_settings {
            let current = settings::read(&tx)?;
            let mut incoming = import.manifest.settings;
            incoming.revision = crate::next_revision(current.revision)?;
            incoming.safety_disclaimer_accepted = current.safety_disclaimer_accepted;
            incoming.auto_connect_saved_device_id = import
                .manifest
                .auto_connect_history_id
                .as_ref()
                .and_then(|id| board_ids.get(id))
                .cloned();
            tx.execute(
                "UPDATE settings SET revision=?,json=? WHERE singleton=1",
                params![
                    incoming.revision,
                    json(&incoming, limits::MAX_SETTINGS_BYTES)?
                ],
            )?;
            summary.settings_imported = true;
        }
        File::open(&self.root)?.sync_all()?;
        // Once commit is attempted its durable outcome can be uncertain. Never
        // delete possibly committed session files; startup reconciles true orphans.
        for session in &mut import.sessions {
            session.owned = false;
        }
        let result = tx.commit();
        // Skipped files are still unreferenced and can be reclaimed immediately.
        for session in &import.sessions {
            let referenced: bool = self.catalog.query_row(
                "SELECT EXISTS(SELECT 1 FROM session_catalog WHERE file_name=?)",
                [session
                    .path
                    .file_name()
                    .and_then(|p| p.to_str())
                    .ok_or(Error::UnsafePath)?],
                |r| r.get(0),
            )?;
            if !referenced {
                let _ = crate::sessions::remove_session_files(&session.path);
            }
        }
        result?;
        self.catalog_revision = revision;
        Ok(summary)
    }
    pub fn clear_app_data(&mut self) -> Result<()> {
        self.require_app_idle()?;
        if !self.leases.lock().map_err(|_| Error::Database)?.is_empty() {
            return Err(Error::Busy);
        }
        let revision = crate::next_revision(self.catalog_revision)?;
        let mut defaults = Settings::default();
        let current = self.settings()?;
        defaults.revision = crate::next_revision(current.revision)?;
        defaults.safety_disclaimer_accepted = current.safety_disclaimer_accepted;
        let tx = self
            .catalog
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("UPDATE session_catalog SET pending_delete=1", [])?;
        tx.execute_batch("DELETE FROM ride_checkpoint; DELETE FROM saved_device; DELETE FROM refloat_configuration; DELETE FROM refloat_editor_draft; DELETE FROM session_maintenance_job;")?;
        tx.execute(
            "UPDATE settings SET revision=?,json=? WHERE singleton=1",
            params![
                defaults.revision,
                json(&defaults, limits::MAX_SETTINGS_BYTES)?
            ],
        )?;
        tx.commit()?;
        self.catalog_revision = revision;
        self.cleanup_deletes()
    }
}

// Admit the ZIP's central directory before ZipArchive allocates its entry index.
// Version 1 uses ordinary single-disk ZIPs (our bounds need no ZIP64).
fn check_zip_index(file: &mut File) -> Result<usize> {
    let length = file.metadata()?.len();
    let count = length.min(65_557) as usize;
    file.seek(SeekFrom::End(-(count as i64)))?;
    let mut tail = vec![0; count];
    file.read_exact(&mut tail)?;
    let index = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|i| {
            tail[*i..].starts_with(b"PK\x05\x06")
                && *i + 22 + u16::from_le_bytes([tail[*i + 20], tail[*i + 21]]) as usize
                    == tail.len()
        })
        .ok_or(Error::InvalidInput)?;
    let u16at = |i| u16::from_le_bytes([tail[index + i], tail[index + i + 1]]) as usize;
    let u32at = |i| u32::from_le_bytes(tail[index + i..index + i + 4].try_into().unwrap()) as u64;
    let entries = u16at(10);
    if u16at(4) != 0
        || u16at(6) != 0
        || entries != u16at(8)
        || entries == 0
        || entries > limits::MAX_BOARD_EXPORT_SESSIONS + 1
        || u32at(12) > 4 * 1024 * 1024
        || u32at(12) + u32at(16) != length - count as u64 + index as u64
    {
        return Err(Error::InvalidInput);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(entries)
}
impl ImportSink {
    pub fn validate_app(
        self,
        cancel: &FileCancellation,
        apply_settings: bool,
    ) -> Result<AppImport> {
        let input = self.finish()?;
        cancel.check()?;
        let mut file = File::open(input.path())?;
        let entries = check_zip_index(&mut file)?;
        let mut zip = zip::ZipArchive::new(file).map_err(|_| Error::InvalidInput)?;
        if zip.len() != entries {
            return Err(Error::InvalidInput);
        }
        let manifest: AppManifest = {
            let mut entry = zip
                .by_name("manifest.json")
                .map_err(|_| Error::InvalidInput)?;
            if entry.size() > limits::MAX_APP_MANIFEST_BYTES as u64 {
                return Err(Error::Capacity);
            }
            let mut bytes = Vec::new();
            entry
                .by_ref()
                .take(limits::MAX_APP_MANIFEST_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > limits::MAX_APP_MANIFEST_BYTES {
                return Err(Error::Capacity);
            }
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidInput)?
        };
        manifest.validate()?;
        if zip.len() != manifest.sessions.len() + 1 {
            return Err(Error::InvalidInput);
        }
        let allowed: BTreeSet<_> = manifest.sessions.iter().map(|s| s.entry.as_str()).collect();
        let mut names = BTreeSet::new();
        let mut extracted = 0u64;
        for index in 0..zip.len() {
            let entry = zip.by_index(index).map_err(|_| Error::InvalidInput)?;
            if entry.name().len() > 128
                || !names.insert(entry.name().to_owned())
                || (entry.name() != "manifest.json" && !allowed.contains(entry.name()))
            {
                return Err(Error::InvalidInput);
            }
            if entry.name() != "manifest.json" {
                if entry.size() > limits::MAX_IMPORT_BYTES {
                    return Err(Error::Capacity);
                }
                extracted = extracted.checked_add(entry.size()).ok_or(Error::Capacity)?;
            }
        }
        if extracted > limits::MAX_BOARD_EXPORT_BYTES {
            return Err(Error::Capacity);
        }
        require_export_space(
            &input.root,
            extracted
                .saturating_add(limits::MAX_IMPORT_BYTES * 2)
                .saturating_add(limits::EXPORT_FREE_SPACE_HEADROOM),
        )?;
        let mut sessions = Vec::new();
        let mut actual_extracted = 0u64;
        let mut bytes = [0u8; 65_536];
        for session in &manifest.sessions {
            cancel.check()?;
            let mut entry = zip
                .by_name(&session.entry)
                .map_err(|_| Error::InvalidInput)?;
            let mut scratch = input.scratch_import()?;
            let mut offset = 0;
            loop {
                cancel.check()?;
                let n = entry.read(&mut bytes)?;
                if n == 0 {
                    break;
                }
                actual_extracted = actual_extracted
                    .checked_add(n as u64)
                    .filter(|v| *v <= limits::MAX_BOARD_EXPORT_BYTES)
                    .ok_or(Error::Capacity)?;
                scratch.write(offset, &bytes[..n])?;
                offset += n as u64;
            }
            let mut archive = scratch.validate(cancel)?;
            if archive.manifest.session_id != session.session_id
                || archive.manifest.device_history_id != session.device_history_id
            {
                return Err(Error::InvalidInput);
            }
            let path = input.root.join(format!("session-{}.sqlite", identifier()?));
            if path.try_exists()? {
                return Err(Error::Capacity);
            }
            fs::rename(&archive.stage.path, &path)?;
            archive.stage.owned = false;
            sessions.push(PendingSession {
                path,
                manifest: archive.manifest,
                marker: archive.marker,
                owned: true,
            });
        }
        cancel.check()?;
        Ok(AppImport {
            input,
            manifest,
            sessions,
            apply_settings,
        })
    }
}
