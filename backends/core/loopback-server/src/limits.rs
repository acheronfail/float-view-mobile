//! Public contract ceilings and internal admission limits.

use std::time::Duration;

pub const MAX_MESSAGE_BYTES: usize = 65_536;
pub const MAX_SESSION_DATA_REQUEST_BYTES: usize = 16_384;
pub const MAX_SESSION_DATA_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SESSION_IMPORT_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_CONNECTIONS: usize = 16;
pub const MAX_CLIENT_OPERATIONS: usize = 8;
pub const MAX_HOST_ENVELOPE_BYTES: usize = 1024 * 1024;
pub const MAX_HISTORY_READS: usize = 2;
pub const MAX_TRANSFERS: usize = 2;
pub const MAX_TRANSFER_CHUNK_BYTES: usize = 64 * 1024;
pub const MAX_RESPONSE_QUEUE: usize = 8;
pub const MAX_BLOCKING_WORKERS: usize = 2;
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
pub const IO_TIMEOUT: Duration = Duration::from_secs(30);
