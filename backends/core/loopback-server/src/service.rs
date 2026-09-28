//! Direct service selection. Removed native dispatch remains temporarily selectable
//! only until both native cutovers have passed their checkpoints.
use crate::{
    backend::Backend,
    facts::*,
    protocol::{Failure, Operation},
    transfer::Direction,
};
use float_view_backend_service::{BackendService, Dispatch, ExportFormat, FileResult};
use std::sync::{Arc, Mutex, atomic::Ordering};

impl Backend {
    pub(crate) async fn prune_service(
        &self,
        id: Option<&str>,
        resolution: u64,
        preview: bool,
    ) -> Result<serde_json::Value, Failure> {
        let _permit = self
            .service_permits
            .try_acquire()
            .map_err(|_| Failure::Unavailable)?;
        let service = self.service.as_ref().ok_or(Failure::Internal)?;
        let mut changes = service.lock().map_err(|_| Failure::Internal)?.subscribe();
        let mut job = {
            let mut s = service.lock().map_err(|_| Failure::Internal)?;
            let epoch = s.epoch_ms();
            s.begin_maintenance(id, resolution, preview, epoch)
                .map_err(|_| Failure::InvalidState)?
        };
        if preview {
            loop {
                if matches!(job.state.as_str(), "completed" | "cancelled" | "failed") {
                    break;
                }
                tokio::time::timeout(crate::limits::IO_TIMEOUT, changes.changed())
                    .await
                    .map_err(|_| Failure::Unavailable)?
                    .map_err(|_| Failure::Unavailable)?;
                job = service
                    .lock()
                    .map_err(|_| Failure::Internal)?
                    .maintenance()
                    .map_err(|_| Failure::Internal)?
                    .filter(|j| j.operation_id == job.operation_id)
                    .ok_or(Failure::Conflict)?;
            }
        }
        self.sync_service()?;
        let record = crate::facts::Maintenance {
            operation_id: job.operation_id,
            kind: job.kind,
            scope: job.scope,
            state: job.state,
            session_id: job.session_id,
            target_resolution_seconds: job.target_resolution_seconds,
            upper_session_id: job.upper_session_id,
            last_session_id: job.last_session_id,
            completed_sessions: job.completed_sessions,
            skipped_sessions: job.skipped_sessions,
            retained_samples: job.retained_samples,
            reclaimable_bytes: job.reclaimable_bytes,
            error: job.error,
        };
        let operation = crate::projection::maintenance(&record);
        if preview {
            let recorded = service
                .lock()
                .map_err(|_| Failure::Internal)?
                .maintenance_recorded_samples()
                .map_err(|_| Failure::Internal)?;
            Ok(
                serde_json::json!({"operation":operation,"recordedSampleCount":recorded,"estimatedStoredSampleCount":record.retained_samples,"estimatedReclaimableBytes":record.reclaimable_bytes,"skipped":record.completed_sessions==record.skipped_sessions,"skipReason":if record.completed_sessions==record.skipped_sessions{Some("already_at_or_below_target")}else{None}}),
            )
        } else {
            Ok(operation)
        }
    }

    pub(crate) fn service_lease(
        &self,
        id: &str,
    ) -> Result<Option<float_view_backend_service::SessionLease>, Failure> {
        self.service
            .as_ref()
            .map(|s| {
                s.lock()
                    .map_err(|_| Failure::Internal)?
                    .source(id)
                    .map(|s| s.1)
            })
            .transpose()
    }

    pub fn with_service(service: Arc<Mutex<BackendService>>) -> Self {
        Self {
            service: Some(service),
            ..Self::default()
        }
    }
    pub fn sync_service(&self) -> Result<(), Failure> {
        let Some(service) = &self.service else {
            return Ok(());
        };
        let service = service.lock().map_err(|_| Failure::Internal)?;
        let facts = service.published_snapshot();
        let revision = service.service_revision();
        let catalog_revision = service.catalog_revision();
        let value = Arc::new(crate::projection::state(&facts, revision, catalog_revision));
        let mut publisher = self.publisher.lock().map_err(|_| Failure::Internal)?;
        if publisher.facts.is_some()
            && publisher.revision == revision
            && publisher.catalog_revision == catalog_revision
        {
            return Ok(());
        }
        publisher.facts = Some(facts);
        publisher.revision = revision;
        publisher.catalog_revision = catalog_revision;
        self.snapshots.send_replace(Some(value));
        Ok(())
    }
    pub(crate) fn watch_service(self: &Arc<Self>) {
        let Some(service) = &self.service else {
            return;
        };
        let Ok(service) = service.lock() else {
            return;
        };
        let mut changes = service.subscribe();
        let weak = Arc::downgrade(self);
        tokio::spawn(async move {
            while changes.changed().await.is_ok() {
                let Some(backend) = weak.upgrade() else {
                    break;
                };
                let _ = backend.sync_service();
            }
        });
    }
    pub(crate) fn disconnect_service_client(&self, client: u64) {
        if let Some(service) = &self.service
            && let Ok(mut service) = service.lock()
        {
            service.disconnect_client(client);
        }
        if let Ok(mut mapping) = self.service_transfers.lock() {
            let ids: Vec<_> = mapping
                .iter()
                .filter(|(_, v)| v.0 == client)
                .map(|(id, _)| *id)
                .collect();
            for id in ids {
                mapping.remove(&id);
                if let Ok(mut t) = self.transfers.lock() {
                    t.remove(id);
                }
            }
        }
    }
    pub(crate) fn cleanup_service_transfer(&self, id: u64) {
        if let Ok(mut mapping) = self.service_transfers.lock()
            && let Some((owner, local)) = mapping.remove(&id)
            && let Some(service) = &self.service
            && let Ok(mut service) = service.lock()
        {
            let _ = service.cancel_file_transfer(owner, local);
        }
        if let Ok(mut transfers) = self.transfers.lock() {
            transfers.remove(id);
        }
    }
    pub(crate) async fn call_service(
        &self,
        client: u64,
        operation: Operation,
    ) -> Result<Completion, Failure> {
        let _permit = self
            .service_permits
            .try_acquire()
            .map_err(|_| Failure::Unavailable)?;
        let service = self.service.as_ref().ok_or(Failure::Internal)?;
        match operation {
            Operation::BeginImport { transfer_id } => {
                let mut mapping = self
                    .service_transfers
                    .lock()
                    .map_err(|_| Failure::Internal)?;
                if mapping.len() >= 2 || mapping.contains_key(&transfer_id) {
                    return Err(Failure::Unavailable);
                }
                let mut s = service.lock().map_err(|_| Failure::Internal)?;
                let now = s.monotonic_ms();
                let id = s
                    .begin_import_transfer(client, None, now)
                    .map_err(|_| Failure::Unavailable)?;
                mapping.insert(transfer_id, (client, id));
                Ok(Completion::TransferReady)
            }
            Operation::WriteTransfer {
                transfer_id,
                offset,
            } => {
                let id = self.local_transfer(client, transfer_id)?;
                let bytes = self
                    .transfers
                    .lock()
                    .map_err(|_| Failure::Internal)?
                    .take(
                        self.generation.load(Ordering::Acquire),
                        transfer_id,
                        Direction::Import,
                    )
                    .map_err(|_| Failure::InvalidState)?;
                let mut s = service.lock().map_err(|_| Failure::Internal)?;
                let now = s.monotonic_ms();
                s.write_import(client, id, offset, &bytes, now)
                    .map_err(|_| Failure::InvalidRequest)?;
                Ok(Completion::TransferReady)
            }
            Operation::FinishImport { transfer_id } => {
                let id = self.local_transfer(client, transfer_id)?;
                let work = service
                    .lock()
                    .map_err(|_| Failure::Internal)?
                    .validate_import(client, id)
                    .map_err(|_| Failure::InvalidRequest)?;
                let completion = tokio::task::spawn_blocking(move || work.run())
                    .await
                    .map_err(|_| Failure::Internal)?;
                let result = {
                    let mut s = service.lock().map_err(|_| Failure::Internal)?;
                    let now = s.monotonic_ms();
                    s.complete_file_work(completion, now)
                        .map_err(|_| Failure::InvalidRequest)?
                };
                self.service_transfers
                    .lock()
                    .map_err(|_| Failure::Internal)?
                    .remove(&transfer_id);
                self.sync_service()?;
                match result {
                    FileResult::Imported(id) => Ok(Completion::Imported(id)),
                    _ => Err(Failure::Internal),
                }
            }
            Operation::ExportSession {
                id,
                format,
                present,
                transfer_id,
            } => {
                let format = match format.as_str() {
                    "csv" => ExportFormat::Csv,
                    "float-view-archive" => ExportFormat::Archive,
                    _ => return Err(Failure::InvalidRequest),
                };
                let (local, work) = {
                    let mut s = service.lock().map_err(|_| Failure::Internal)?;
                    let now = s.monotonic_ms();
                    s.begin_export(client, &id, format, now)
                        .map_err(|_| Failure::Unavailable)?
                };
                self.service_transfers
                    .lock()
                    .map_err(|_| Failure::Internal)?
                    .insert(transfer_id, (client, local));
                let completion = tokio::task::spawn_blocking(move || work.run())
                    .await
                    .map_err(|_| Failure::Internal)?;
                let mut s = service.lock().map_err(|_| Failure::Internal)?;
                let now = s.monotonic_ms();
                let FileResult::Exported {
                    file_name, length, ..
                } = s
                    .complete_file_work(completion, now)
                    .map_err(|_| Failure::Unavailable)?
                else {
                    return Err(Failure::Internal);
                };
                if present {
                    s.present_file(client, local, &file_name)
                        .map_err(|_| Failure::Unavailable)?;
                }
                Ok(Completion::Export {
                    file_name,
                    presented: present,
                    transfer_id,
                    length,
                })
            }
            Operation::ReadTransfer {
                transfer_id,
                offset,
                length,
            } => {
                if length == 0 || length > 65_536 {
                    return Err(Failure::InvalidRequest);
                }
                let id = self.local_transfer(client, transfer_id)?;
                let mut bytes = vec![0; length];
                let n = {
                    let mut s = service.lock().map_err(|_| Failure::Internal)?;
                    let now = s.monotonic_ms();
                    s.read_export(client, id, offset, &mut bytes, now)
                        .map_err(|_| Failure::Unavailable)?
                };
                bytes.truncate(n);
                self.transfers
                    .lock()
                    .map_err(|_| Failure::Internal)?
                    .put(
                        self.generation.load(Ordering::Acquire),
                        transfer_id,
                        Direction::Export,
                        offset,
                        &bytes,
                    )
                    .map_err(|_| Failure::InvalidState)?;
                Ok(Completion::TransferReady)
            }
            Operation::CleanupTransfer { transfer_id } => {
                self.cleanup_service_transfer(transfer_id);
                Ok(Completion::Unit)
            }
            operation => {
                let reply = service
                    .lock()
                    .map_err(|_| Failure::Internal)?
                    .dispatch(client, operation)?;
                self.sync_service()?;
                let result = match reply {
                    Dispatch::Complete(result) => Ok(*result),
                    Dispatch::Pending(receiver) => {
                        tokio::time::timeout(crate::limits::IO_TIMEOUT, receiver)
                            .await
                            .map_err(|_| Failure::Unavailable)?
                            .map_err(|_| Failure::Unavailable)?
                    }
                };
                self.sync_service()?;
                result
            }
        }
    }
    fn local_transfer(&self, client: u64, id: u64) -> Result<u64, Failure> {
        self.service_transfers
            .lock()
            .map_err(|_| Failure::Internal)?
            .get(&id)
            .filter(|v| v.0 == client)
            .map(|v| v.1)
            .ok_or(Failure::InvalidState)
    }
}
