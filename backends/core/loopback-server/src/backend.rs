//! Process-owned publication and host exchange, independent of listener lifetime.
use crate::{
    exchange::{Exchange, OperationKind},
    facts::*,
    limits::*,
    projection,
    protocol::*,
};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, watch};

pub struct Publisher {
    pub sequence: u64,
    pub revision: u64,
    pub catalog_revision: u64,
    pub facts: Option<Snapshot>,
}
pub struct Backend {
    pub service: Option<Arc<Mutex<float_view_backend_service::BackendService>>>,
    pub(crate) service_permits: tokio::sync::Semaphore,
    pub(crate) service_transfers: Mutex<std::collections::BTreeMap<u64, (u64, u64)>>,
    pub exchange: Exchange<Operation, Outcome>,
    pub history: Arc<tokio::sync::Semaphore>,
    pub changed: Notify,
    pub completions: Mutex<crate::completion::Completions>,
    pub transfers: Mutex<crate::transfer::Transfers>,
    pub cleanup: Mutex<Vec<u64>>,
    pub generation: AtomicU64,
    pub publisher: Mutex<Publisher>,
    pub snapshots: watch::Sender<Option<Arc<Value>>>,
}
impl Default for Backend {
    fn default() -> Self {
        Self {
            service: None,
            service_permits: tokio::sync::Semaphore::new(64),
            service_transfers: Mutex::default(),
            exchange: Exchange::default(),
            history: Arc::new(tokio::sync::Semaphore::new(MAX_HISTORY_READS)),
            changed: Notify::new(),
            completions: Mutex::default(),
            transfers: Mutex::new(crate::transfer::Transfers::default()),
            cleanup: Mutex::new(Vec::new()),
            generation: AtomicU64::new(0),
            publisher: Mutex::new(Publisher {
                sequence: 0,
                revision: 0,
                catalog_revision: 0,
                facts: None,
            }),
            snapshots: watch::channel(None).0,
        }
    }
}
impl Backend {
    pub fn start(&self) -> Result<u64, Failure> {
        let generation = self.exchange.start().map_err(|_| Failure::InvalidState)?;
        self.generation.store(generation, Ordering::Release);
        Ok(generation)
    }
    pub fn stop(&self) {
        let _ = self.exchange.stop();
        if let Ok(mut completions) = self.completions.lock() {
            completions.clear();
        }
        self.changed.notify_waiters();
    }
    pub fn complete(&self, record: CompletionRecord) -> Result<(), Failure> {
        self.completions
            .lock()
            .map_err(|_| Failure::Internal)?
            .remove(record.id);
        self.exchange
            .complete(record.id, record.outcome)
            .map_err(|_| Failure::InvalidState)?;
        self.changed.notify_waiters();
        Ok(())
    }
    pub fn publish(&self, publication: Publication) -> Result<(), Failure> {
        if publication.generation != self.generation.load(Ordering::Acquire) {
            return Err(Failure::InvalidState);
        }
        publication
            .validate()
            .map_err(|_| Failure::InvalidRequest)?;
        let mut publisher = self.publisher.lock().map_err(|_| Failure::Internal)?;
        if publication.sequence <= publisher.sequence {
            return Err(Failure::Conflict);
        }
        if !publication.catalog_changed
            && let Some(previous) = &publisher.facts
            && serde_json::to_vec(previous).map_err(|_| Failure::Internal)?
                == serde_json::to_vec(&publication.facts).map_err(|_| Failure::Internal)?
        {
            publisher.sequence = publication.sequence;
            return Ok(());
        }
        let revision = publisher
            .revision
            .checked_add(1)
            .filter(|v| *v <= 9_007_199_254_740_991)
            .ok_or(Failure::Internal)?;
        let catalog_revision = publisher
            .catalog_revision
            .checked_add(u64::from(publication.catalog_changed))
            .ok_or(Failure::Internal)?;
        let snapshot = projection::state(&publication.facts, revision, catalog_revision);
        let bytes = serde_json::to_vec(&snapshot).map_err(|_| Failure::Internal)?;
        if bytes.len() + 40 > MAX_MESSAGE_BYTES {
            return Err(Failure::InvalidRequest);
        }
        publisher.sequence = publication.sequence;
        publisher.revision = revision;
        publisher.catalog_revision = catalog_revision;
        publisher.facts = Some(publication.facts);
        self.snapshots.send_replace(Some(Arc::new(snapshot)));
        Ok(())
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
    pub async fn call(&self, client: u64, operation: Operation) -> Result<Completion, Failure> {
        if self.service.is_some() {
            return self.call_service(client, operation).await;
        }
        let generation = self.generation.load(Ordering::Acquire);
        let kind = if operation.is_read() {
            OperationKind::Read
        } else {
            OperationKind::Mutation
        };
        let id = self
            .exchange
            .submit(generation, client, kind, operation)
            .map_err(|_| Failure::Unavailable)?;
        let result = tokio::time::timeout(IO_TIMEOUT, async {
            loop {
                let notified = self.changed.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                match self
                    .exchange
                    .take_completion(id)
                    .map_err(|_| Failure::Unavailable)?
                {
                    Some(Outcome::Completed(completion)) => return Ok(*completion),
                    Some(Outcome::Failed(failure)) => return Err(failure),
                    None => notified.await,
                }
            }
        })
        .await;
        match result {
            Ok(result) => result,
            Err(_) => {
                let _ = self.exchange.detach_client(generation, client);
                Err(Failure::Unavailable)
            }
        }
    }
}
