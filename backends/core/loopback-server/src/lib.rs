//! Shared loopback transport above the application runtime.
//!
//! Selected by Apple and Android through their existing binding families. It must
//! never construct a second application runtime. Dispatch calls the shared owner
//! directly; native hosts only execute bounded OS capabilities.

#![forbid(unsafe_code)]

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

mod encoding;

mod reading;

mod service;
