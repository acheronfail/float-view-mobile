//! Portable BLE policy. OS objects and reconnect decisions stay outside this reducer.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_CHUNKS: usize = 128;
const MAX_BYTES: usize = 65_536;
pub(crate) const DEADLINE_MS: u64 = 10_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteMode {
    WithResponse,
    WithoutResponse,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub service: bool,
    pub notify: bool,
    pub with_response: usize,
    pub without_response: usize,
}
impl Profile {
    pub(crate) fn select(self) -> Option<(WriteMode, usize)> {
        if !self.service || !self.notify {
            return None;
        }
        let (mode, ceiling) = if self.with_response > 0 {
            (WriteMode::WithResponse, self.with_response)
        } else {
            (WriteMode::WithoutResponse, self.without_response)
        };
        (ceiling > 0 && ceiling <= 512).then_some((mode, ceiling.min(244)))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Progress {
    Submitted,
    Acknowledged,
    Available,
    Failed,
}

pub(crate) fn eligible(uart: bool, id: &str, saved: &[String]) -> bool {
    uart || saved.iter().any(|s| s == id)
}
fn fits(chunks: usize, bytes: usize, added_chunks: usize, added_bytes: usize) -> bool {
    chunks <= MAX_CHUNKS
        && bytes <= MAX_BYTES
        && added_chunks <= MAX_CHUNKS - chunks
        && added_bytes <= MAX_BYTES - bytes
}

#[derive(Default)]
pub(crate) struct Transport {
    pub generation: u64,
    pub setup: u64,
    pub ready: bool,
    selected: Option<(WriteMode, usize)>,
    queue: VecDeque<Vec<u8>>,
    bytes: usize,
    current: Option<(u64, bool, bool)>,
}
impl Transport {
    pub fn begin(&mut self, generation: u64) {
        *self = Self {
            generation,
            ..Self::default()
        };
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn profile(
        &mut self,
        generation: u64,
        profile: Profile,
        operation: u64,
    ) -> Result<Option<WriteMode>, ()> {
        if generation != self.generation || generation == 0 || self.selected.is_some() {
            return Ok(None);
        }
        let (mode, ceiling) = profile.select().ok_or(())?;
        self.selected = Some((mode, ceiling));
        self.setup = operation;
        Ok(Some(mode))
    }
    pub fn notifications(
        &mut self,
        generation: u64,
        operation: u64,
        success: bool,
    ) -> Result<bool, ()> {
        if generation != self.generation
            || self.selected.is_none()
            || operation != self.setup
            || self.ready
        {
            return Ok(false);
        }
        if !success {
            return Err(());
        }
        self.ready = true;
        Ok(true)
    }
    /// Validate all arithmetic before allocating or modifying the queue.
    pub fn admit(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let (_, ceiling) = self.selected.filter(|_| self.ready).ok_or(())?;
        let count = bytes.len().checked_add(ceiling - 1).ok_or(())? / ceiling;
        if !fits(self.queue.len(), self.bytes, count, bytes.len()) {
            return Err(());
        }
        self.queue.extend(bytes.chunks(ceiling).map(<[u8]>::to_vec));
        self.bytes += bytes.len();
        Ok(())
    }
    pub fn next(&mut self, operation: u64) -> Option<(WriteMode, Vec<u8>)> {
        if self.current.is_some() || !self.ready {
            return None;
        }
        let bytes = self.queue.front()?.clone();
        self.current = Some((operation, false, false));
        Some((self.selected?.0, bytes))
    }
    pub fn progress(
        &mut self,
        generation: u64,
        operation: u64,
        result: Progress,
    ) -> Result<bool, ()> {
        if generation != self.generation {
            return Ok(false);
        }
        let Some((id, submitted, completed)) = self.current.as_mut() else {
            return Ok(false);
        };
        if *id != operation {
            return Ok(false);
        }
        match result {
            Progress::Failed => return Err(()),
            Progress::Submitted => *submitted = true,
            Progress::Acknowledged if self.selected.unwrap().0 == WriteMode::WithResponse => {
                *completed = true
            }
            Progress::Available if self.selected.unwrap().0 == WriteMode::WithoutResponse => {
                *completed = true
            }
            _ => {}
        }
        if !*submitted || !*completed {
            return Ok(false);
        }
        self.bytes -= self.queue.pop_front().unwrap().len();
        self.current = None;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
