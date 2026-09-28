//! Safe process registry shared by the existing C/JNI binding families.
#![forbid(unsafe_code)]
pub mod service;
pub const ABI_VERSION: u32 = 1;
pub const MAX_COMMAND_BYTES: usize = 1_048_576;
pub const MAX_RESULT_BYTES: usize = 1_048_576;
pub fn abi_version() -> u32 {
    ABI_VERSION
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Ok = 0,
    InvalidArgument = 1,
    InvalidHandle = 2,
    InvalidEnvelope = 3,
    RuntimeFailure = 4,
    ResultPending = 5,
    BufferTooSmall = 6,
    Busy = 7,
    Internal = 8,
    InvalidState = 9,
    Conflict = 10,
    Unavailable = 11,
}
