//! Two generation-scoped binary transfer slots; archive bytes never enter JSON.
use crate::limits::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Import,
    Export,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferError {
    Full,
    Stale,
    InvalidChunk,
    Busy,
    TooLarge,
}
struct Transfer {
    generation: u64,
    direction: Direction,
    next: u64,
    chunk: Option<Vec<u8>>,
    maximum: u64,
}
#[derive(Default)]
pub struct Transfers {
    slots: BTreeMap<u64, Transfer>,
}
static NEXT: AtomicU64 = AtomicU64::new(1);
impl Transfers {
    pub fn create(
        &mut self,
        generation: u64,
        direction: Direction,
        maximum: u64,
    ) -> Result<u64, TransferError> {
        if self.slots.len() >= MAX_TRANSFERS {
            return Err(TransferError::Full);
        }
        let id = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| TransferError::Full)?;
        self.slots.insert(
            id,
            Transfer {
                generation,
                direction,
                next: 0,
                chunk: None,
                maximum,
            },
        );
        Ok(id)
    }
    pub fn put(
        &mut self,
        generation: u64,
        id: u64,
        direction: Direction,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), TransferError> {
        let transfer = self
            .slots
            .get_mut(&id)
            .filter(|t| t.generation == generation && t.direction == direction)
            .ok_or(TransferError::Stale)?;
        if transfer.chunk.is_some() {
            return Err(TransferError::Busy);
        }
        if bytes.is_empty() || bytes.len() > MAX_TRANSFER_CHUNK_BYTES || offset != transfer.next {
            return Err(TransferError::InvalidChunk);
        }
        let next = offset
            .checked_add(bytes.len() as u64)
            .filter(|n| *n <= transfer.maximum)
            .ok_or(TransferError::TooLarge)?;
        transfer.chunk = Some(bytes.to_vec());
        transfer.next = next;
        Ok(())
    }
    pub fn take(
        &mut self,
        generation: u64,
        id: u64,
        direction: Direction,
    ) -> Result<Vec<u8>, TransferError> {
        self.slots
            .get_mut(&id)
            .filter(|t| t.generation == generation && t.direction == direction)
            .ok_or(TransferError::Stale)?
            .chunk
            .take()
            .ok_or(TransferError::Busy)
    }
    pub fn remove(&mut self, id: u64) {
        self.slots.remove(&id);
    }
    pub fn clear(&mut self) -> Vec<u64> {
        let ids = self.slots.keys().copied().collect();
        self.slots.clear();
        ids
    }
}
