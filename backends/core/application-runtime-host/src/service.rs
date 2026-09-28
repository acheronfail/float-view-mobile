//! Safe process registry for the existing C/JNI families.
use crate::Status;
use float_view_backend_service::{BackendService, Process, os};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, LazyLock, Mutex},
};
struct Entry {
    process: Process,
    pending: Mutex<Option<Vec<u8>>>,
}
struct Registry {
    next: u64,
    entries: BTreeMap<u64, Arc<Entry>>,
}
static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| {
    Mutex::new(Registry {
        next: 1 << 40,
        entries: BTreeMap::new(),
    })
});
#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Drain {},
    Fact { envelope: os::Envelope },
}
fn entry(handle: u64) -> Result<Arc<Entry>, Status> {
    REGISTRY
        .lock()
        .map_err(|_| Status::Internal)?
        .entries
        .get(&handle)
        .cloned()
        .ok_or(Status::InvalidHandle)
}
pub fn create(root: &[u8]) -> Result<u64, Status> {
    if root.is_empty() || root.len() > 4096 {
        return Err(Status::InvalidArgument);
    }
    let root = std::str::from_utf8(root).map_err(|_| Status::InvalidArgument)?;
    let mut registry = REGISTRY.lock().map_err(|_| Status::Internal)?;
    if registry.entries.len() >= 8 {
        return Err(Status::Busy);
    }
    let process = Process::open(Path::new(root)).map_err(|_| Status::RuntimeFailure)?;
    let handle = registry.next;
    registry.next = handle.checked_add(1).ok_or(Status::Internal)?;
    registry.entries.insert(
        handle,
        Arc::new(Entry {
            process,
            pending: Mutex::new(None),
        }),
    );
    Ok(handle)
}
pub fn owner(handle: u64) -> Result<Arc<Mutex<BackendService>>, Status> {
    Ok(entry(handle)?.process.service.clone())
}
pub fn destroy(handle: u64) -> Status {
    let value = match entry(handle) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if value.process.shutdown().is_err() {
        return Status::Unavailable;
    }
    match REGISTRY.lock() {
        Ok(mut r) => {
            r.entries.remove(&handle);
            Status::Ok
        }
        Err(_) => Status::Internal,
    }
}
pub fn apply(handle: u64, input: &[u8]) -> Result<usize, Status> {
    if input.is_empty() || input.len() > os::MAX_ENVELOPE_BYTES {
        return Err(Status::InvalidArgument);
    }
    let command: Command = serde_json::from_slice(input).map_err(|_| Status::InvalidEnvelope)?;
    let entry = entry(handle)?;
    let mut pending = entry.pending.lock().map_err(|_| Status::Internal)?;
    if pending.is_some() {
        return Err(Status::ResultPending);
    }
    let mut service = entry.process.service.lock().map_err(|_| Status::Internal)?;
    match command {
        Command::Drain {} => {}
        Command::Fact { envelope } => service.apply_os(envelope).map_err(|e| match e {
            float_view_backend_service::operations::Failure::InvalidRequest => {
                Status::InvalidEnvelope
            }
            float_view_backend_service::operations::Failure::InvalidState => Status::InvalidState,
            _ => Status::RuntimeFailure,
        })?,
    }
    let bytes = serde_json::to_vec(&service.drain_os()).map_err(|_| Status::Internal)?;
    if bytes.len() > os::MAX_ENVELOPE_BYTES {
        return Err(Status::RuntimeFailure);
    }
    let size = bytes.len();
    *pending = Some(bytes);
    Ok(size)
}
pub fn copy_result(handle: u64, output: &mut [u8]) -> Result<usize, (Status, usize)> {
    if output.len() > os::MAX_ENVELOPE_BYTES {
        return Err((Status::InvalidArgument, 0));
    }
    let entry = entry(handle).map_err(|e| (e, 0))?;
    let mut pending = entry.pending.lock().map_err(|_| (Status::Internal, 0))?;
    let bytes = pending.as_ref().ok_or((Status::InvalidState, 0))?;
    let size = bytes.len();
    if output.len() < size {
        return Err((Status::BufferTooSmall, size));
    }
    output[..size].copy_from_slice(bytes);
    *pending = None;
    Ok(size)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn result_copy_is_retryable_and_service_owner_survives_listener_borrows() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("float-view-service-host-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let handle = create(root.to_str().unwrap().as_bytes()).unwrap();
        let service = owner(handle).unwrap();
        let address = Arc::as_ptr(&service);
        drop(service);
        let required = apply(handle, br#"{"command":"drain"}"#).unwrap();
        assert_eq!(
            apply(handle, br#"{"command":"drain"}"#),
            Err(Status::ResultPending)
        );
        assert_eq!(
            copy_result(handle, &mut []),
            Err((Status::BufferTooSmall, required))
        );
        let mut bytes = vec![0; required];
        assert_eq!(copy_result(handle, &mut bytes), Ok(required));
        assert_eq!(Arc::as_ptr(&owner(handle).unwrap()), address);
        assert_eq!(
            apply(handle, br#"{"command":"drain","unknown":true}"#),
            Err(Status::InvalidEnvelope)
        );
        assert_eq!(destroy(handle), Status::Ok);
        assert!(owner(handle).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
