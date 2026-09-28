//! Process-owned composition, independent of listeners and native views.
#![forbid(unsafe_code)]

use float_view_application_runtime::ApplicationRuntime;
use float_view_product_store::{Maintenance, MaintenanceCancellation, ProductStore, Result};
use std::path::Path;
pub mod facts;
mod files;
pub mod operations;
pub use files::{
    FileCompletion, FileResult, FileWork, MaintenanceJobCompletion, MaintenanceJobWork,
};

/// One store and one runtime. Listener handles must borrow this owner, never
/// construct another runtime. OS exchange and command admission live here.
pub struct BackendService {
    runtime: ApplicationRuntime,
    store: ProductStore,
    files: files::Files,
    maintenance_worker: Option<MaintenanceCancellation>,
}
impl BackendService {
    pub fn open(root: &Path) -> Result<Self> {
        let store = ProductStore::open(root)?;
        Ok(Self {
            runtime: ApplicationRuntime::new(),
            store,
            files: files::Files::default(),
            maintenance_worker: None,
        })
    }
    pub fn begin_maintenance(
        &mut self,
        session_id: Option<&str>,
        resolution: u64,
        preview: bool,
        at_ms: i64,
    ) -> Result<Maintenance> {
        self.store
            .begin_maintenance(session_id, resolution, preview, at_ms)
    }
    pub fn maintenance(&self) -> Result<Option<Maintenance>> {
        self.store.maintenance()
    }
    pub fn cancel_maintenance(&mut self, id: &str, at_ms: i64) -> Result<Maintenance> {
        let job = self.store.cancel_maintenance(id, at_ms)?;
        if let Some(worker) = &self.maintenance_worker {
            worker.cancel();
        }
        Ok(job)
    }
    pub fn snapshot(&self) -> float_view_application_runtime::Snapshot {
        self.runtime.snapshot()
    }
    pub fn settings(&self) -> Result<float_view_product_store::Settings> {
        self.store.settings()
    }
}

impl Drop for BackendService {
    fn drop(&mut self) {
        if let Some(worker) = &self.maintenance_worker {
            worker.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    #[test]
    fn shared_handles_retain_one_runtime_and_catalog_owner() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("float-view-service-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let owner = Arc::new(Mutex::new(BackendService::open(&root).unwrap()));
        let runtime_address = std::ptr::from_ref(&owner.lock().unwrap().runtime) as usize;
        let listener = owner.clone();
        drop(listener);
        let replacement = owner.clone();
        assert_eq!(
            std::ptr::from_ref(&replacement.lock().unwrap().runtime) as usize,
            runtime_address
        );
        assert!(!replacement.lock().unwrap().snapshot().session_attached);
        assert_eq!(replacement.lock().unwrap().settings().unwrap().revision, 0);
        drop(replacement);
        drop(owner);
        std::fs::remove_dir_all(root).unwrap();
    }
}
