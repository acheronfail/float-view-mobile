//! Transport state borrowing one process-owned application service.
use crate::{
    facts::Snapshot,
    limits::*,
    protocol::{Failure, Operation},
};
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::watch;
pub struct Publisher {
    pub revision: u64,
    pub catalog_revision: u64,
    pub facts: Option<Snapshot>,
}
pub struct Backend {
    pub service: Arc<Mutex<float_view_backend_service::BackendService>>,
    pub(crate) service_permits: tokio::sync::Semaphore,
    pub(crate) service_transfers: Mutex<std::collections::BTreeMap<u64, (u64, u64)>>,
    pub history: Arc<tokio::sync::Semaphore>,
    pub transfers: Mutex<crate::transfer::Transfers>,
    pub generation: AtomicU64,
    pub publisher: Mutex<Publisher>,
    pub snapshots: watch::Sender<Option<Arc<Value>>>,
}
impl Backend {
    pub fn with_service(service: Arc<Mutex<float_view_backend_service::BackendService>>) -> Self {
        Self {
            service,
            service_permits: tokio::sync::Semaphore::new(64),
            service_transfers: Mutex::default(),
            history: Arc::new(tokio::sync::Semaphore::new(MAX_HISTORY_READS)),
            transfers: Mutex::default(),
            generation: AtomicU64::new(0),
            publisher: Mutex::new(Publisher {
                revision: 0,
                catalog_revision: 0,
                facts: None,
            }),
            snapshots: watch::channel(None).0,
        }
    }
    pub fn start(&self) -> Result<u64, Failure> {
        self.generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1))
            .map(|v| v + 1)
            .map_err(|_| Failure::Unavailable)
    }
    pub fn snapshot(&self) -> Result<Value, Failure> {
        self.snapshots
            .borrow()
            .as_ref()
            .map(|v| v.as_ref().clone())
            .ok_or(Failure::Unavailable)
    }
    pub fn catalog_revision(&self) -> Result<u64, Failure> {
        Ok(self
            .publisher
            .lock()
            .map_err(|_| Failure::Internal)?
            .catalog_revision)
    }
    pub async fn call(
        &self,
        client: u64,
        operation: Operation,
    ) -> Result<crate::facts::Completion, Failure> {
        self.call_service(client, operation).await
    }
}
