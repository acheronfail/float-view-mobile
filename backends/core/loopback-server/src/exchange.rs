//! Bounded, callback-free operation lifecycle, independent of native executors.
//!
//! Draining transfers owned operations to the host. Admission remains occupied
//! until completion, including after a client abandons an accepted mutation.
//! No method invokes host code or enters the application runtime.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::limits::{MAX_CLIENT_OPERATIONS, MAX_HOST_BATCH, MAX_HOST_WAIT, MAX_OPERATIONS};

// Process-wide uniqueness also rejects completions routed to a foreign handle.
static GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct OperationId {
    pub generation: u64,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    Read,
    Mutation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExchangeError {
    Stopped,
    Destroyed,
    Full,
    Stale,
    InvalidState,
    InvalidWait,
    Exhausted,
    Poisoned,
}

#[derive(serde::Serialize)]
pub struct HostOperation<T> {
    pub id: OperationId,
    pub operation: T,
}

struct Pending<T, R> {
    client: u64,
    kind: OperationKind,
    operation: Option<T>,
    completion: Option<R>,
    detached: bool,
    cancelled: bool,
}

struct State<T, R> {
    generation: u64,
    next_sequence: u64,
    running: bool,
    destroyed: bool,
    queued: VecDeque<u64>,
    pending: BTreeMap<u64, Pending<T, R>>,
}

pub struct Exchange<T, R> {
    state: Mutex<State<T, R>>,
    changed: Condvar,
}

impl<T, R> Default for Exchange<T, R> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                generation: 0,
                next_sequence: 0,
                running: false,
                destroyed: false,
                queued: VecDeque::new(),
                pending: BTreeMap::new(),
            }),
            changed: Condvar::new(),
        }
    }
}

impl<T, R> Exchange<T, R> {
    /// Start only after binding succeeds. Failed admission never advances IDs.
    pub fn start(&self) -> Result<u64, ExchangeError> {
        let mut state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        if state.destroyed {
            return Err(ExchangeError::Destroyed);
        }
        if state.running {
            return Err(ExchangeError::InvalidState);
        }
        state.generation = GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ExchangeError::Exhausted)?
            + 1;
        state.next_sequence = 0;
        state.running = true;
        Ok(state.generation)
    }

    pub fn cancelled(&self, generation: u64) -> Result<Vec<OperationId>, ExchangeError> {
        let state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        state.validate(generation)?;
        Ok(state
            .pending
            .iter()
            .filter(|(_, p)| p.cancelled)
            .map(|(sequence, _)| OperationId {
                generation,
                sequence: *sequence,
            })
            .collect())
    }

    pub fn submit(
        &self,
        generation: u64,
        client: u64,
        kind: OperationKind,
        operation: T,
    ) -> Result<OperationId, ExchangeError> {
        let mut state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        state.validate(generation)?;
        if state.pending.len() >= MAX_OPERATIONS
            || state
                .pending
                .values()
                .filter(|p| p.client == client)
                .count()
                >= MAX_CLIENT_OPERATIONS
        {
            return Err(ExchangeError::Full);
        }
        let sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or(ExchangeError::Exhausted)?;
        state.next_sequence = sequence;
        state.pending.insert(
            sequence,
            Pending {
                client,
                kind,
                operation: Some(operation),
                completion: None,
                detached: false,
                cancelled: false,
            },
        );
        state.queued.push_back(sequence);
        self.changed.notify_all();
        Ok(OperationId {
            generation,
            sequence,
        })
    }

    /// A dedicated native worker may wait; UI/application executors must not.
    /// Returned records are owned, so the host cannot retain a transport lock.
    pub fn drain(
        &self,
        generation: u64,
        wait: Duration,
    ) -> Result<Vec<HostOperation<T>>, ExchangeError> {
        if wait > MAX_HOST_WAIT {
            return Err(ExchangeError::InvalidWait);
        }
        let deadline = Instant::now() + wait;
        let mut state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        loop {
            state.validate(generation)?;
            if !state.queued.is_empty() || Instant::now() >= deadline {
                break;
            }
            (state, _) = self
                .changed
                .wait_timeout(state, deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| ExchangeError::Poisoned)?;
        }
        let mut operations = Vec::with_capacity(MAX_HOST_BATCH);
        while operations.len() < MAX_HOST_BATCH {
            let Some(sequence) = state.queued.pop_front() else {
                break;
            };
            if let Some(operation) = state
                .pending
                .get_mut(&sequence)
                .and_then(|p| p.operation.take())
            {
                operations.push(HostOperation {
                    id: OperationId {
                        generation,
                        sequence,
                    },
                    operation,
                });
            }
        }
        Ok(operations)
    }

    pub fn accepts_completion(&self, id: OperationId) -> bool {
        self.state.lock().is_ok_and(|state| {
            state.validate(id.generation).is_ok()
                && state.pending.get(&id.sequence).is_some_and(|p| {
                    p.operation.is_none() && p.completion.is_none() && !p.cancelled
                })
        })
    }

    pub fn complete(&self, id: OperationId, completion: R) -> Result<(), ExchangeError> {
        let mut state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        state.validate(id.generation)?;
        let pending = state
            .pending
            .get_mut(&id.sequence)
            .ok_or(ExchangeError::Stale)?;
        if pending.operation.is_some() || pending.completion.is_some() {
            return Err(ExchangeError::InvalidState);
        }
        if pending.detached {
            state.pending.remove(&id.sequence);
        } else {
            pending.completion = Some(completion);
        }
        self.changed.notify_all();
        Ok(())
    }

    /// Consume a terminal result once. Until consumed it retains its admission
    /// slot, so completions cannot grow an unbounded second response queue.
    pub fn take_completion(&self, id: OperationId) -> Result<Option<R>, ExchangeError> {
        let mut state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        state.validate(id.generation)?;
        let pending = state
            .pending
            .get_mut(&id.sequence)
            .ok_or(ExchangeError::Stale)?;
        let completion = pending.completion.take();
        if completion.is_some() {
            state.pending.remove(&id.sequence);
        }
        Ok(completion)
    }

    /// Read workers poll this between bounded batches and propagate it to the
    /// existing runtime interruption path. Accepted mutations are not cancelled.
    pub fn is_cancelled(&self, id: OperationId) -> bool {
        self.state.lock().map_or(true, |state| {
            state.validate(id.generation).is_err()
                || state.pending.get(&id.sequence).is_none_or(|p| p.cancelled)
        })
    }

    pub fn detach_client(&self, generation: u64, client: u64) -> Result<(), ExchangeError> {
        let mut state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        state.validate(generation)?;
        state.pending.retain(|_, pending| {
            if pending.client != client {
                return true;
            }
            pending.detached = true;
            if pending.completion.is_some() {
                return false;
            }
            if pending.kind == OperationKind::Read {
                pending.cancelled = true;
                // Undrained reads have no worker resources to acknowledge.
                if pending.operation.is_some() {
                    return false;
                }
            }
            true
        });
        let State {
            queued, pending, ..
        } = &mut *state;
        queued.retain(|id| pending.contains_key(id));
        self.changed.notify_all();
        Ok(())
    }

    /// Wake waiters and discard transport state only. The caller must propagate
    /// interruption/cleanup to host workers; this does not destroy the runtime.
    pub fn stop(&self) -> Result<(), ExchangeError> {
        self.close(false)
    }

    pub fn destroy(&self) -> Result<(), ExchangeError> {
        self.close(true)
    }

    fn close(&self, destroy: bool) -> Result<(), ExchangeError> {
        let mut state = self.state.lock().map_err(|_| ExchangeError::Poisoned)?;
        state.running = false;
        state.destroyed |= destroy;
        state.queued.clear();
        state.pending.clear();
        self.changed.notify_all();
        Ok(())
    }
}

impl<T, R> State<T, R> {
    fn validate(&self, generation: u64) -> Result<(), ExchangeError> {
        if self.destroyed {
            Err(ExchangeError::Destroyed)
        } else if !self.running {
            Err(ExchangeError::Stopped)
        } else if self.generation != generation {
            Err(ExchangeError::Stale)
        } else {
            Ok(())
        }
    }
}
