//! Shared loopback transport above the application runtime.
//!
//! This candidate is unselected until the host migration gates pass. It must
//! never construct or own a second application runtime. Native workers execute
//! application operations outside transport locks through the existing bindings.

#![forbid(unsafe_code)]

pub mod exchange;
pub mod limits;

mod archives;
pub mod backend;
mod dispatch;
pub mod facts;
mod history;
pub mod host;
mod maintenance;
pub mod projection;
pub mod protocol;
pub mod server;
pub mod transfer;

pub mod completion;

mod encoding;

mod reading;
