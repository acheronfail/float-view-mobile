//! Safe registry shared by the existing C and JNI binding families.
use crate::{
    limits::*,
    protocol::{Command, Envelope, Platform, VERSION},
    server::Server,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, LazyLock, Mutex},
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
pub fn create_shared(
    platform: u32,
    html: &[u8],
    service: Arc<Mutex<float_view_backend_service::BackendService>>,
) -> Result<u64, Status> {
    let platform = match platform {
        0 => Platform::Macos,
        1 => Platform::Ios,
        2 => Platform::Android,
        _ => return Err(Status::InvalidArgument),
    };
    let server = Arc::new(
        Server::with_backend(
            platform,
            html,
            Arc::new(crate::backend::Backend::with_service(service)),
        )
        .map_err(|_| Status::InvalidArgument)?,
    );
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
    };
    let bytes = serde_json::to_vec(&json!({"version":VERSION,"data":result}))
        .map_err(|_| Status::Internal)?;
    if bytes.len() > MAX_HOST_ENVELOPE_BYTES {
        return Err(Status::Internal);
    }
    Ok(bytes)
}
