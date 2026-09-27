//! Safe registry shared by the existing C and JNI binding families.
use crate::{
    limits::*,
    protocol::{Command, Envelope, Platform, VERSION},
    server::Server,
    transfer::Direction,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Ok = 0,
    InvalidArgument = 1,
    InvalidHandle = 2,
    InvalidEnvelope = 3,
    Failure = 4,
    BufferTooSmall = 6,
    Busy = 7,
    Internal = 8,
}
struct Registry {
    next: u64,
    servers: BTreeMap<u64, Arc<Server>>,
}
static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| {
    Mutex::new(Registry {
        next: 1,
        servers: BTreeMap::new(),
    })
});
pub fn create(platform: u32, html: &[u8]) -> Result<u64, Status> {
    let platform = match platform {
        0 => Platform::Macos,
        1 => Platform::Ios,
        2 => Platform::Android,
        _ => return Err(Status::InvalidArgument),
    };
    let server = Arc::new(Server::new(platform, html).map_err(|_| Status::InvalidArgument)?);
    let mut registry = REGISTRY.lock().map_err(|_| Status::Internal)?;
    if registry.servers.len() >= 8 {
        return Err(Status::Busy);
    }
    let id = registry.next;
    registry.next = id.checked_add(1).ok_or(Status::Internal)?;
    registry.servers.insert(id, server);
    Ok(id)
}
pub fn server(handle: u64) -> Result<Arc<Server>, Status> {
    REGISTRY
        .lock()
        .map_err(|_| Status::Internal)?
        .servers
        .get(&handle)
        .cloned()
        .ok_or(Status::InvalidHandle)
}
pub fn destroy(handle: u64) -> Status {
    let server = {
        let Ok(mut registry) = REGISTRY.lock() else {
            return Status::Internal;
        };
        if handle == 0 || handle >= registry.next {
            return Status::InvalidHandle;
        }
        registry.servers.remove(&handle)
    };
    if let Some(server) = server {
        let _ = server.stop();
        let _ = server.backend.exchange.destroy();
    }
    Status::Ok
}
pub fn command(handle: u64, input: &[u8]) -> Result<Vec<u8>, Status> {
    let envelope = Envelope::decode(input).map_err(|_| Status::InvalidEnvelope)?;
    let server = server(handle)?;
    let result: Value = match envelope.action {
        Command::Start(start) => {
            if &start.platform != server.platform() {
                return Err(Status::InvalidEnvelope);
            }
            let port = server.start(start.port).map_err(|_| Status::Failure)?;
            json!({"port":port,"generation":server.backend.generation.load(std::sync::atomic::Ordering::Acquire),"token":server.token()})
        }
        Command::Stop => json!({"cleanup":server.stop().map_err(|_|Status::Failure)?}),
        Command::Drain {
            generation,
            wait_ms,
        } => {
            server
                .backend
                .completions
                .lock()
                .map_err(|_| Status::Internal)?
                .retain(|id| server.backend.exchange.accepts_completion(id));
            let has_cleanup = !server
                .backend
                .cleanup
                .lock()
                .map_err(|_| Status::Internal)?
                .is_empty();
            let operations = server
                .backend
                .exchange
                .drain(
                    generation,
                    if !has_cleanup {
                        Duration::from_millis(wait_ms)
                    } else {
                        Duration::ZERO
                    },
                )
                .map_err(|_| Status::Failure)?;
            let cleanup = server
                .backend
                .cleanup
                .lock()
                .map_err(|_| Status::Internal)?
                .drain(..)
                .collect::<Vec<_>>();
            json!({"operations":operations,"cleanup":cleanup,"cancelled":server.backend.exchange.cancelled(generation).map_err(|_|Status::Failure)?})
        }
        Command::CompleteFragment(fragment) => {
            let id = fragment.id;
            if !server.backend.exchange.accepts_completion(id) {
                server
                    .backend
                    .completions
                    .lock()
                    .map_err(|_| Status::Internal)?
                    .remove(id);
                return Err(Status::Failure);
            }
            let result = server
                .backend
                .completions
                .lock()
                .map_err(|_| Status::Internal)?
                .append(fragment)
                .map_err(|_| Status::InvalidEnvelope)?;
            if let Some(outcome) = result {
                server
                    .backend
                    .complete(crate::protocol::CompletionRecord { id, outcome })
                    .map_err(|_| Status::Failure)?;
            }
            json!({})
        }
        Command::Complete(record) => {
            server
                .backend
                .complete(record)
                .map_err(|_| Status::Failure)?;
            json!({})
        }
        Command::Publish(publication) => {
            server
                .backend
                .publish(*publication)
                .map_err(|_| Status::InvalidEnvelope)?;
            json!({})
        }
        Command::Cancel { generation, client } => {
            server
                .backend
                .exchange
                .detach_client(generation, client)
                .map_err(|_| Status::Failure)?;
            json!({})
        }
        Command::TransferCleaned { transfer_id } => {
            server
                .backend
                .transfers
                .lock()
                .map_err(|_| Status::Internal)?
                .remove(transfer_id);
            json!({})
        }
    };
    let bytes = serde_json::to_vec(&json!({"version":VERSION,"data":result}))
        .map_err(|_| Status::Internal)?;
    if bytes.len() > MAX_HOST_ENVELOPE_BYTES {
        return Err(Status::Internal);
    }
    Ok(bytes)
}
pub fn read_chunk(handle: u64, generation: u64, transfer: u64) -> Result<Vec<u8>, Status> {
    server(handle)?
        .backend
        .transfers
        .lock()
        .map_err(|_| Status::Internal)?
        .take(generation, transfer, Direction::Import)
        .map_err(|_| Status::InvalidArgument)
}
pub fn write_chunk(
    handle: u64,
    generation: u64,
    transfer: u64,
    offset: u64,
    bytes: &[u8],
) -> Status {
    let Ok(server) = server(handle) else {
        return Status::InvalidHandle;
    };
    match server.backend.transfers.lock() {
        Ok(mut pool) => match pool.put(generation, transfer, Direction::Export, offset, bytes) {
            Ok(()) => Status::Ok,
            Err(_) => Status::InvalidArgument,
        },
        Err(_) => Status::Internal,
    }
}
