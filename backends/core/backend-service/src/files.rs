//! Correlated file leases and shared bounded worker admission.
use crate::BackendService;
use float_view_product_store::{
    ArchiveExport, ArchiveImport, Error, ExportFormat, FileCancellation, ImportSink, Maintenance,
    MaintenanceCompletion, MaintenanceWork, Result, StagedFile, limits,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
const IDLE_MS: u64 = 30_000;
const PRESENTATION_MS: u64 = 86_400_000;
static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Default)]
pub(crate) struct Files {
    entries: BTreeMap<u64, Transfer>,
    workers: Arc<AtomicUsize>,
}
struct Transfer {
    owner: u64,
    deadline: u64,
    cancelled: bool,
    state: State,
    presentation_reply: Option<tokio::sync::oneshot::Sender<bool>>,
}
enum State {
    Import(ImportSink),
    Working {
        cancel: FileCancellation,
        live: Weak<()>,
    },
    Export(ExportFile),
    Presenting(ExportFile),
}
enum ExportFile {
    Product(StagedFile),
    Diagnostic(std::sync::Arc<crate::diagnostics::ReportFile>),
}
impl From<StagedFile> for ExportFile {
    fn from(f: StagedFile) -> Self {
        Self::Product(f)
    }
}
impl ExportFile {
    fn path(&self) -> &std::path::Path {
        match self {
            Self::Product(f) => f.path(),
            Self::Diagnostic(f) => f.path(),
        }
    }
    fn read(&self, offset: u64, target: &mut [u8]) -> Result<usize> {
        match self {
            Self::Product(f) => f.read(offset, target),
            Self::Diagnostic(f) => f.read(offset, target),
        }
    }
}
pub(crate) struct Permit(Arc<AtomicUsize>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
impl Files {
    pub(crate) fn permit(&self) -> Result<Permit> {
        self.workers
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| {
                if v < 2 { Some(v + 1) } else { None }
            })
            .map_err(|_| Error::Capacity)?;
        Ok(Permit(self.workers.clone()))
    }
    fn admit(&self) -> Result<u64> {
        if self.entries.len() >= limits::MAX_TRANSFERS {
            return Err(Error::Capacity);
        }
        NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
            v.checked_add(1).filter(|n| *n <= limits::MAX_REVISION)
        })
        .map_err(|_| Error::Capacity)
    }
    fn owned(&mut self, owner: u64, id: u64) -> Result<&mut Transfer> {
        self.entries
            .get_mut(&id)
            .filter(|t| t.owner == owner && !t.cancelled)
            .ok_or(Error::NotFound)
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        for t in self.entries.values() {
            if let State::Working { cancel, .. } = &t.state {
                cancel.cancel();
            }
        }
    }
}
enum FileTask {
    Import(ImportSink),
    Export(Box<ArchiveExport>),
}
pub struct FileWork {
    id: u64,
    task: FileTask,
    cancel: FileCancellation,
    permit: Permit,
    live: Arc<()>,
}
pub struct FileCompletion {
    id: u64,
    result: Result<FileValue>,
    _permit: Permit,
    _live: Arc<()>,
}
enum FileValue {
    Import(Box<ArchiveImport>),
    Export(StagedFile),
}
#[derive(Debug, PartialEq, Eq)]
pub enum FileResult {
    Imported(String),
    Exported {
        transfer_id: u64,
        file_name: String,
        length: u64,
    },
}
impl FileWork {
    pub fn run(self) -> FileCompletion {
        let result = match self.task {
            FileTask::Import(s) => s
                .validate(&self.cancel)
                .map(Box::new)
                .map(FileValue::Import),
            FileTask::Export(e) => e.run(&self.cancel).map(FileValue::Export),
        };
        FileCompletion {
            id: self.id,
            result,
            _permit: self.permit,
            _live: self.live,
        }
    }
}
pub struct MaintenanceJobWork {
    work: MaintenanceWork,
    permit: Permit,
}
pub struct MaintenanceJobCompletion {
    work: MaintenanceCompletion,
    _permit: Permit,
}
impl MaintenanceJobWork {
    pub fn run(self, at_ms: i64) -> MaintenanceJobCompletion {
        self.run_with_reclaiming(at_ms, || {})
    }
    pub fn run_with_reclaiming(
        self,
        at_ms: i64,
        on_reclaiming: impl FnOnce(),
    ) -> MaintenanceJobCompletion {
        MaintenanceJobCompletion {
            work: self.work.run_with_reclaiming(at_ms, on_reclaiming),
            _permit: self.permit,
        }
    }
}
impl BackendService {
    pub fn begin_import_transfer(
        &mut self,
        owner: u64,
        length: Option<u64>,
        now_ms: u64,
    ) -> Result<u64> {
        let id = self.files.admit()?;
        let sink = match length {
            Some(n) => self.store.begin_import(n)?,
            None => self.store.begin_import_stream()?,
        };
        self.files.entries.insert(
            id,
            Transfer {
                owner,
                deadline: now_ms.saturating_add(IDLE_MS),
                cancelled: false,
                presentation_reply: None,
                state: State::Import(sink),
            },
        );
        Ok(id)
    }
    pub fn write_import(
        &mut self,
        owner: u64,
        id: u64,
        offset: u64,
        bytes: &[u8],
        now_ms: u64,
    ) -> Result<()> {
        let transfer = self.files.owned(owner, id)?;
        let State::Import(sink) = &mut transfer.state else {
            return Err(Error::Busy);
        };
        sink.write(offset, bytes)?;
        transfer.deadline = now_ms.saturating_add(IDLE_MS);
        Ok(())
    }
    pub fn validate_import(&mut self, owner: u64, id: u64) -> Result<FileWork> {
        let permit = self.files.permit()?;
        let transfer = self.files.owned(owner, id)?;
        if !matches!(transfer.state, State::Import(_)) {
            return Err(Error::Busy);
        }
        let cancel = FileCancellation::default();
        let live = Arc::new(());
        let State::Import(sink) = std::mem::replace(
            &mut transfer.state,
            State::Working {
                cancel: cancel.clone(),
                live: Arc::downgrade(&live),
            },
        ) else {
            unreachable!()
        };
        Ok(FileWork {
            id,
            task: FileTask::Import(sink),
            cancel,
            permit,
            live,
        })
    }
    pub(crate) fn adopt_diagnostic_file(
        &mut self,
        owner: u64,
        file: std::sync::Arc<crate::diagnostics::ReportFile>,
    ) -> Result<u64> {
        let id = self.files.admit()?;
        self.files.entries.insert(
            id,
            Transfer {
                owner,
                deadline: self.policy.exchange.now.saturating_add(IDLE_MS),
                cancelled: false,
                presentation_reply: None,
                state: State::Export(ExportFile::Diagnostic(file)),
            },
        );
        Ok(id)
    }
    pub fn begin_export(
        &mut self,
        owner: u64,
        session: &str,
        format: ExportFormat,
        now_ms: u64,
    ) -> Result<(u64, FileWork)> {
        let id = self.files.admit()?;
        let permit = self.files.permit()?;
        let export = self.store.prepare_export(session, format)?;
        let cancel = FileCancellation::default();
        let live = Arc::new(());
        self.files.entries.insert(
            id,
            Transfer {
                owner,
                deadline: now_ms.saturating_add(IDLE_MS),
                cancelled: false,
                presentation_reply: None,
                state: State::Working {
                    cancel: cancel.clone(),
                    live: Arc::downgrade(&live),
                },
            },
        );
        Ok((
            id,
            FileWork {
                id,
                task: FileTask::Export(Box::new(export)),
                cancel,
                permit,
                live,
            },
        ))
    }
    pub fn complete_file_work(
        &mut self,
        completion: FileCompletion,
        now_ms: u64,
    ) -> Result<FileResult> {
        let mut transfer = self
            .files
            .entries
            .remove(&completion.id)
            .ok_or(Error::NotFound)?;
        if !matches!(transfer.state, State::Working { .. }) {
            self.files.entries.insert(completion.id, transfer);
            return Err(Error::NotFound);
        }
        if transfer.cancelled {
            return Err(Error::Cancelled);
        }
        match completion.result? {
            FileValue::Import(value) => {
                let id = self.store.attach_import(*value)?.session_id;
                self.refresh_boards()?;
                self.changed().map_err(|_| Error::Capacity)?;
                Ok(FileResult::Imported(id))
            }
            FileValue::Export(file) => {
                let length = file.len()?;
                let file_name = file
                    .path()
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or(Error::UnsafePath)?
                    .to_owned();
                transfer.state = State::Export(file.into());
                transfer.deadline = now_ms.saturating_add(IDLE_MS);
                self.files.entries.insert(completion.id, transfer);
                Ok(FileResult::Exported {
                    transfer_id: completion.id,
                    file_name,
                    length,
                })
            }
        }
    }
    pub fn read_export(
        &mut self,
        owner: u64,
        id: u64,
        offset: u64,
        bytes: &mut [u8],
        now_ms: u64,
    ) -> Result<usize> {
        let transfer = self.files.owned(owner, id)?;
        let State::Export(file) = &transfer.state else {
            return Err(Error::Busy);
        };
        let n = file.read(offset, bytes)?;
        transfer.deadline = now_ms.saturating_add(IDLE_MS);
        Ok(n)
    }
    /// Only this service-issued path may be sent to an OS presenter. The service
    /// retains the file until acknowledgment or bounded abandonment.
    pub fn present_export(
        &mut self,
        owner: u64,
        id: u64,
        now_ms: u64,
    ) -> Result<std::path::PathBuf> {
        let transfer = self.files.owned(owner, id)?;
        let State::Export(file) = &transfer.state else {
            return Err(Error::Busy);
        };
        let path = file.path().to_owned();
        let placeholder = State::Working {
            cancel: FileCancellation::default(),
            live: Weak::new(),
        };
        let State::Export(file) = std::mem::replace(&mut transfer.state, placeholder) else {
            unreachable!()
        };
        transfer.state = State::Presenting(file);
        transfer.deadline = now_ms.saturating_add(PRESENTATION_MS);
        Ok(path)
    }
    pub fn present_file(
        &mut self,
        owner: u64,
        id: u64,
        file_name: &str,
    ) -> Result<tokio::sync::oneshot::Receiver<bool>> {
        if file_name.is_empty() || file_name.len() > 128 || file_name.contains(['/', '\\', '\0']) {
            return Err(Error::InvalidInput);
        }
        self.policy.exchange.admit().map_err(|_| Error::Capacity)?;
        let path = self.present_export(owner, id, self.policy.exchange.now)?;
        self.policy
            .exchange
            .push(crate::os::Effect::PresentFile {
                id,
                path: path.to_str().ok_or(Error::UnsafePath)?.into(),
                file_name: file_name.into(),
            })
            .map_err(|_| Error::Capacity)?;
        let (reply, receiver) = tokio::sync::oneshot::channel();
        let transfer = self.files.owned(owner, id)?;
        transfer.presentation_reply = Some(reply);
        transfer.deadline = self.policy.exchange.now.saturating_add(IDLE_MS);
        Ok(receiver)
    }
    pub fn presentation_started(&mut self, id: u64, accepted: bool) -> Result<()> {
        let transfer = self.files.entries.get_mut(&id).ok_or(Error::NotFound)?;
        let reply = transfer.presentation_reply.take().ok_or(Error::NotFound)?;
        if !accepted {
            let state = std::mem::replace(
                &mut transfer.state,
                State::Working {
                    cancel: FileCancellation::default(),
                    live: Weak::new(),
                },
            );
            let State::Presenting(file) = state else {
                return Err(Error::InvalidInput);
            };
            transfer.state = State::Export(file);
        }
        transfer.deadline = self.policy.exchange.now.saturating_add(if accepted {
            PRESENTATION_MS
        } else {
            IDLE_MS
        });
        let _ = reply.send(accepted);
        Ok(())
    }
    pub fn finish_presentation(&mut self, id: u64) -> Result<()> {
        if !self
            .files
            .entries
            .get(&id)
            .is_some_and(|t| matches!(t.state, State::Presenting(_)))
        {
            return Err(Error::NotFound);
        }
        self.files.entries.remove(&id);
        Ok(())
    }
    pub fn cancel_file_transfer(&mut self, owner: u64, id: u64) -> Result<()> {
        let transfer = self.files.owned(owner, id)?;
        match &transfer.state {
            State::Working { cancel, .. } => {
                cancel.cancel();
                transfer.cancelled = true;
            }
            State::Presenting(file) => {
                let _ = file.path();
            }
            _ => {
                self.files.entries.remove(&id);
            }
        }
        Ok(())
    }
    pub fn disconnect_file_client(&mut self, owner: u64) {
        self.diagnostics.disconnect_client(owner);
        let ids: Vec<_> = self
            .files
            .entries
            .iter()
            .filter(|(_, t)| t.owner == owner)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            let _ = self.cancel_file_transfer(owner, id);
        }
    }
    pub fn expire_file_transfers(&mut self, now_ms: u64) {
        self.files.entries.retain(|_, t| {
            if now_ms < t.deadline {
                return true;
            }
            if let State::Working { cancel, live } = &t.state {
                cancel.cancel();
                t.cancelled = true;
                live.strong_count() > 0
            } else {
                false
            }
        });
    }
    pub(crate) fn active_workers(&self) -> usize {
        self.files.workers.load(Ordering::Acquire)
    }
    pub(crate) fn cancel_workers(&mut self) {
        if let Some(worker) = &self.maintenance_worker {
            worker.cancel();
        }
        for transfer in self.files.entries.values_mut() {
            if let State::Working { cancel, .. } = &transfer.state {
                cancel.cancel();
                transfer.cancelled = true;
            }
        }
    }
    pub fn next_maintenance_work(&mut self, at_ms: i64) -> Result<Option<MaintenanceJobWork>> {
        let permit = self.files.permit()?;
        let previous = self.store.maintenance()?;
        let work = self.store.next_maintenance_work(at_ms)?;
        if previous != self.store.maintenance()? {
            self.changed().map_err(|_| Error::Capacity)?;
        }
        self.maintenance_worker = work.as_ref().map(MaintenanceWork::cancellation);
        Ok(work.map(|work| MaintenanceJobWork { work, permit }))
    }
    pub fn complete_maintenance_work(
        &mut self,
        completion: MaintenanceJobCompletion,
        at_ms: i64,
    ) -> Result<Maintenance> {
        let job = self.store.complete_maintenance_work(completion.work, at_ms);
        self.maintenance_worker = None;
        self.refresh_boards()?;
        self.changed().map_err(|_| Error::Capacity)?;
        job
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_view_application_runtime::{session::SessionWriter, telemetry::RideState};
    mod support {
        use float_view_application_runtime::telemetry as float_view_refloat_telemetry_service;
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../session-sqlite/tests/support/mod.rs"
        ));
    }
    fn setup() -> (support::TestDirectory, BackendService, String) {
        let dir = support::TestDirectory::new();
        let mut service = BackendService::open(dir.path()).unwrap();
        service
            .store
            .update_settings(0, r#"{"minimumSessionDistance":0}"#)
            .unwrap();
        let board = service
            .store
            .ensure_board("local", Some("Board"), Some(&[1; 12]), 1)
            .unwrap();
        let session = service
            .store
            .create_session(&board.saved_device_id, 1, u64::MAX)
            .unwrap();
        let mut writer =
            SessionWriter::open(service.store.session_path(&session.session_id).unwrap()).unwrap();
        let mut sample = support::sample(1, 0);
        sample.ride_state = RideState::Riding;
        writer.append_samples(&[sample]).unwrap();
        writer.close().unwrap();
        service
            .store
            .close_session(&session.session_id, 10, "disconnect")
            .unwrap();
        (dir, service, session.session_id)
    }
    fn export(service: &mut BackendService, id: &str, owner: u64) -> u64 {
        let (transfer, work) = service
            .begin_export(owner, id, ExportFormat::Archive, 0)
            .unwrap();
        assert!(matches!(
            service.complete_file_work(work.run(), 1).unwrap(),
            FileResult::Exported { .. }
        ));
        transfer
    }
    #[test]
    fn reclaiming_is_published_and_cancellation_takes_precedence() {
        let (_dir, mut service, id) = setup();
        service.begin_maintenance(Some(&id), 10, false, 1).unwrap();
        let work = service.next_maintenance_work(2).unwrap().unwrap();
        let changes = service.subscribe();
        let completion = work.run_with_reclaiming(3, || {
            service.changed().unwrap();
            assert!(changes.has_changed().unwrap());
            assert_eq!(service.maintenance().unwrap().unwrap().state, "reclaiming");
            assert_eq!(
                service.published_snapshot().maintenance.unwrap().state,
                "reclaiming"
            );
            let job = service.maintenance().unwrap().unwrap();
            service.cancel_maintenance(&job.operation_id, 4).unwrap();
            assert_eq!(
                service.published_snapshot().maintenance.unwrap().state,
                "cancel_requested"
            );
        });
        let job = service.complete_maintenance_work(completion, 5).unwrap();
        assert_eq!(job.state, "cancelled");
        assert_eq!(
            service.published_snapshot().maintenance.unwrap().state,
            "cancelled"
        );
    }
    #[test]
    fn presentation_outlives_disconnect_and_ack_or_expiry_reclaims_it() {
        let (dir, mut s, id) = setup();
        let transfer = export(&mut s, &id, 1);
        assert!(matches!(
            s.present_export(2, transfer, 2),
            Err(Error::NotFound)
        ));
        let path = s.present_export(1, transfer, 2).unwrap();
        s.disconnect_file_client(1);
        assert!(path.exists());
        s.expire_file_transfers(30_001);
        assert!(path.exists());
        s.finish_presentation(transfer).unwrap();
        assert!(!path.exists());
        assert_eq!(s.finish_presentation(transfer), Err(Error::NotFound));
        let transfer = export(&mut s, &id, 1);
        let path = s.present_export(1, transfer, 2).unwrap();
        s.expire_file_transfers(PRESENTATION_MS + 2);
        assert!(!path.exists());
        assert!(dir.file("catalog.sqlite").exists());
    }
    #[test]
    fn presentation_acceptance_is_correlated_single_use_and_bounded() {
        let (_dir, mut s, id) = setup();
        let transfer = export(&mut s, &id, 1);
        let mut reply = s.present_file(1, transfer, "session.csv").unwrap();
        assert!(matches!(
            reply.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        assert_eq!(
            s.presentation_started(transfer + 1, true),
            Err(Error::NotFound)
        );
        s.presentation_started(transfer, false).unwrap();
        assert!(!reply.try_recv().unwrap());
        assert_eq!(s.presentation_started(transfer, true), Err(Error::NotFound));
        assert_eq!(s.finish_presentation(transfer), Err(Error::NotFound));
        s.cancel_file_transfer(1, transfer).unwrap();

        let transfer = export(&mut s, &id, 1);
        let mut reply = s.present_file(1, transfer, "session.csv").unwrap();
        s.expire_file_transfers(IDLE_MS);
        assert!(matches!(
            reply.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Closed)
        ));
        assert_eq!(s.presentation_started(transfer, true), Err(Error::NotFound));

        let transfer = export(&mut s, &id, 1);
        let mut reply = s.present_file(1, transfer, "session.csv").unwrap();
        s.presentation_started(transfer, true).unwrap();
        assert!(reply.try_recv().unwrap());
        s.disconnect_file_client(1);
        s.expire_file_transfers(IDLE_MS);
        s.finish_presentation(transfer).unwrap();
    }
    #[test]
    fn cancellation_retains_admission_until_worker_actually_finishes() {
        let (_dir, mut s, id) = setup();
        s.begin_maintenance(Some(&id), 10, true, 1).unwrap();
        let (first, a) = s.begin_export(1, &id, ExportFormat::Archive, 0).unwrap();
        let (_, b) = s.begin_export(1, &id, ExportFormat::Archive, 0).unwrap();
        assert!(matches!(s.next_maintenance_work(2), Err(Error::Capacity)));
        s.cancel_file_transfer(1, first).unwrap();
        assert!(matches!(
            s.begin_import_transfer(2, Some(1), 3),
            Err(Error::Capacity)
        ));
        assert_eq!(s.complete_file_work(a.run(), 4), Err(Error::Cancelled));
        let t = s.begin_import_transfer(2, None, 5).unwrap();
        s.write_import(2, t, 0, b"bad", 6).unwrap();
        assert!(s.write_import(1, t, 3, b"x", 7).is_err());
        s.complete_file_work(b.run(), 8).unwrap();
        let work = s.validate_import(2, t).unwrap();
        assert!(s.complete_file_work(work.run(), 9).is_err());
    }
    #[test]
    fn foreign_completions_and_expired_transfers_cannot_attach_or_read() {
        let (_dir, mut s, id) = setup();
        let (_other_dir, mut other, _) = setup();
        let (_, work) = s.begin_export(1, &id, ExportFormat::Csv, 0).unwrap();
        assert_eq!(
            other.complete_file_work(work.run(), 1),
            Err(Error::NotFound)
        );
        let transfer = other.begin_import_transfer(1, None, 0).unwrap();
        other.expire_file_transfers(IDLE_MS);
        assert!(other.write_import(1, transfer, 0, b"x", 1).is_err());
    }
}
