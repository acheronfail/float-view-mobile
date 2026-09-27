//! Bounded framing for large typed domain results, independent of archive bytes.
use crate::{
    exchange::OperationId,
    limits::*,
    protocol::{Failure, Outcome},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    pub id: OperationId,
    pub offset: usize,
    pub text: String,
    pub last: bool,
}
#[derive(Default)]
pub struct Completions {
    buffers: BTreeMap<OperationId, Vec<u8>>,
}
impl Completions {
    pub fn remove(&mut self, id: OperationId) {
        self.buffers.remove(&id);
    }
    pub fn retain(&mut self, accepts: impl Fn(OperationId) -> bool) {
        self.buffers.retain(|id, _| accepts(*id));
    }
    pub fn clear(&mut self) {
        self.buffers.clear();
    }
    pub fn append(&mut self, fragment: Fragment) -> Result<Option<Outcome>, Failure> {
        if fragment.text.is_empty() || fragment.text.len() > MAX_TRANSFER_CHUNK_BYTES {
            return Err(Failure::InvalidRequest);
        }
        if !self.buffers.contains_key(&fragment.id) {
            if fragment.offset != 0 {
                return Err(Failure::InvalidRequest);
            }
            if self.buffers.len() >= MAX_HISTORY_READS {
                return Err(Failure::Unavailable);
            }
            self.buffers.insert(fragment.id, Vec::new());
        }
        let buffer = self
            .buffers
            .get_mut(&fragment.id)
            .ok_or(Failure::Internal)?;
        if fragment.offset != buffer.len() {
            return Err(Failure::InvalidRequest);
        }
        if buffer.len().saturating_add(fragment.text.len()) > 8 * 1024 * 1024 {
            self.buffers.remove(&fragment.id);
            return Err(Failure::InvalidRequest);
        }
        buffer.extend_from_slice(fragment.text.as_bytes());
        if !fragment.last {
            return Ok(None);
        }
        let bytes = self.buffers.remove(&fragment.id).ok_or(Failure::Internal)?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| Failure::InvalidRequest)
    }
}
