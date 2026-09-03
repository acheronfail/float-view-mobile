//! One bounded callback-free Apple boundary for the unified application runtime.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::slice;
use std::sync::{Arc, LazyLock, Mutex, TryLockError};

use float_view_application_runtime as runtime;
use runtime::coordinator;
use serde::Deserialize;
use serde_json::{Value, json};

pub const ABI_VERSION: u32 = 1;
pub const MAX_COMMAND_BYTES: usize = 1_048_576;
pub const MAX_RESULT_BYTES: usize = 4_194_304;
pub const MAX_TOKEN_COUNT: usize = runtime::MAX_RUNTIME_EFFECTS * 2;

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

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
enum CommandEnvelope {
    Begin {
        now_ms: u64,
        idle_interval_ms: u64,
        post_ride_tail_ms: u64,
        preferred_controller_can_id: Option<u8>,
        preferred_bms_can_id: Option<u8>,
        routing_hint_can_id: Option<u8>,
        routing_hint_controller_uuid: Option<Vec<u8>>,
        has_active_session: bool,
        continuity_can_id: Option<u8>,
        continuity_controller_uuid: Option<Vec<u8>>,
    },
    SelectController {
        generation: u64,
        can_id: u8,
        now_ms: u64,
    },
    ReceiveBytes {
        generation: u64,
        bytes: Vec<u8>,
        now_ms: u64,
        observed_at_epoch_ms: i64,
    },
    DeadlineFired {
        generation: u64,
        token: u64,
        now_ms: u64,
    },
    TimerFired {
        generation: u64,
        token: u64,
        now_ms: u64,
    },
    Activate {
        generation: u64,
        now_ms: u64,
        session_started: bool,
        cached_configuration_schema: Option<Vec<u8>>,
    },
    SetControl {
        generation: u64,
        control: Control,
        enabled: bool,
        now_ms: u64,
    },
    ClearFatalError {
        generation: u64,
        now_ms: u64,
    },
    ShutdownController {
        generation: u64,
        now_ms: u64,
    },
    ReadConfiguration {
        generation: u64,
        defaults: bool,
        now_ms: u64,
    },
    WriteConfiguration {
        generation: u64,
        mutation: ConfigurationMutation,
        package_version: String,
        fingerprint: String,
        wire_signature: u32,
        base_revision: u64,
        values: BTreeMap<String, ConfigurationValueEnvelope>,
        now_ms: u64,
    },
    UpdateSettings {
        generation: u64,
        idle_interval_ms: u64,
        post_ride_tail_ms: u64,
        now_ms: u64,
    },
    Refresh {
        generation: u64,
        now_ms: u64,
    },
    Reset {
        now_ms: u64,
    },
    AttachSession {
        path: String,
        mode: SessionMode,
        expected_riding_persistence_ms: u64,
        expected_idle_persistence_ms: u64,
        committed_at_epoch_ms: i64,
    },
    AppendLocations {
        locations: Vec<LocationEnvelope>,
    },
    DetachSession,
    SessionMetadata {
        path: String,
    },
    SessionFields {
        path: String,
    },
    SessionSamples {
        path: String,
        fields: Vec<String>,
        start_elapsed_ms: u64,
        end_elapsed_ms: u64,
        limit: usize,
        after_elapsed_ms: Option<u64>,
        after_sequence: Option<u64>,
        boundary: BoundaryEnvelope,
    },
    SessionReduce {
        path: String,
        fields: Vec<String>,
        start_elapsed_ms: u64,
        end_elapsed_ms: u64,
        buckets: usize,
        gap_seconds: Option<f64>,
        boundary: BoundaryEnvelope,
    },
    SessionEvents {
        path: String,
        start_elapsed_ms: u64,
        end_elapsed_ms: u64,
        limit: usize,
        after_elapsed_ms: Option<u64>,
        after_sequence: Option<u64>,
        boundary: BoundaryEnvelope,
    },
    SessionLocations {
        path: String,
        start_elapsed_ms: u64,
        end_elapsed_ms: u64,
        limit: usize,
        after_location_sequence: Option<u64>,
        boundary: BoundaryEnvelope,
    },
    SessionPrune {
        path: String,
        target_resolution_seconds: u64,
        commit: bool,
        committed_at_epoch_ms: i64,
    },
}

impl CommandEnvelope {
    fn consumed_token(&self) -> Option<InputToken> {
        match self {
            Self::DeadlineFired { token, .. } => Some(InputToken::Deadline(*token)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
enum InputToken {
    Deadline(u64),
}

#[derive(Clone, Copy)]
enum RecentToken {
    Deadline(u64, coordinator::scheduler::DeadlineToken),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Control {
    Package,
    Flywheel,
    Leds,
    Headlights,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConfigurationMutation {
    Write,
    TuneApply,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum ConfigurationValueEnvelope {
    Double(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionMode {
    Create,
    Open,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationEnvelope {
    observed_at_epoch_ms: i64,
    elapsed_ms: u64,
    latitude: f64,
    longitude: f64,
    accuracy_m: Option<f64>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryEnvelope {
    history_revision: u64,
    sample_sequence: u64,
    event_sequence: u64,
    location_sequence: u64,
}

impl From<BoundaryEnvelope> for runtime::session::PublishedBoundary {
    fn from(value: BoundaryEnvelope) -> Self {
        Self {
            history_revision: value.history_revision,
            sample_sequence: value.sample_sequence,
            event_sequence: value.event_sequence,
            location_sequence: value.location_sequence,
        }
    }
}

struct Entry {
    runtime: runtime::ApplicationRuntime,
    pending: Option<Vec<u8>>,
    next_token: u64,
    deadlines: Vec<(u64, coordinator::scheduler::DeadlineToken)>,
    timers: Vec<(u64, coordinator::TimerToken)>,
    recent_token: Option<RecentToken>,
}

impl Entry {
    fn new() -> Self {
        Self {
            runtime: runtime::ApplicationRuntime::new(),
            pending: None,
            next_token: 0,
            deadlines: Vec::new(),
            timers: Vec::new(),
            recent_token: None,
        }
    }

    fn allocate_deadline(
        &mut self,
        token: coordinator::scheduler::DeadlineToken,
    ) -> Result<u64, Status> {
        if let Some((id, _)) = self.deadlines.iter().find(|(_, value)| *value == token) {
            return Ok(*id);
        }
        let id = self.allocate_token()?;
        self.deadlines.push((id, token));
        Ok(id)
    }

    fn allocate_timer(&mut self, token: coordinator::TimerToken) -> Result<u64, Status> {
        if let Some((id, _)) = self.timers.iter().find(|(_, value)| *value == token) {
            return Ok(*id);
        }
        let id = self.allocate_token()?;
        self.timers.push((id, token));
        Ok(id)
    }

    fn allocate_token(&mut self) -> Result<u64, Status> {
        if self.deadlines.len().saturating_add(self.timers.len()) >= MAX_TOKEN_COUNT {
            return Err(Status::RuntimeFailure);
        }
        self.next_token = self
            .next_token
            .checked_add(1)
            .ok_or(Status::RuntimeFailure)?;
        Ok(self.next_token)
    }

    fn deadline(&self, id: u64) -> Result<coordinator::scheduler::DeadlineToken, Status> {
        self.deadlines
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, token)| *token)
            .ok_or(Status::InvalidEnvelope)
    }

    fn timer(&self, id: u64) -> Result<coordinator::TimerToken, Status> {
        self.timers
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, token)| *token)
            .ok_or(Status::InvalidEnvelope)
    }

    fn remove_deadline(
        &mut self,
        token: coordinator::scheduler::DeadlineToken,
    ) -> Result<u64, Status> {
        if let Some(index) = self
            .deadlines
            .iter()
            .position(|(_, candidate)| *candidate == token)
        {
            return Ok(self.deadlines.remove(index).0);
        }
        match self.recent_token {
            Some(RecentToken::Deadline(id, candidate)) if candidate == token => Ok(id),
            _ => Ok(0),
        }
    }

    fn remove_timer(&mut self, token: coordinator::TimerToken) -> Result<u64, Status> {
        if let Some(index) = self
            .timers
            .iter()
            .position(|(_, candidate)| *candidate == token)
        {
            return Ok(self.timers.remove(index).0);
        }
        Ok(0)
    }

    fn consume_token(&mut self, consumed: Option<InputToken>) -> Result<(), Status> {
        self.recent_token = match consumed {
            None => None,
            Some(InputToken::Deadline(id)) => {
                let index = self
                    .deadlines
                    .iter()
                    .position(|(candidate, _)| *candidate == id)
                    .ok_or(Status::Internal)?;
                let (_, token) = self.deadlines.remove(index);
                Some(RecentToken::Deadline(id, token))
            }
        };
        Ok(())
    }
}

#[derive(Default)]
struct Registry {
    next_handle: u64,
    entries: BTreeMap<u64, Arc<Mutex<Entry>>>,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| Mutex::new(Registry::default()));
static INTERRUPTERS: LazyLock<Mutex<BTreeMap<u64, ActiveInterrupter>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

enum ActiveInterrupter {
    Reader(runtime::session::ReadInterrupter),
    Pruner(runtime::session::PruneInterrupter),
}

impl ActiveInterrupter {
    fn interrupt(&self) {
        match self {
            Self::Reader(value) => value.interrupt(),
            Self::Pruner(value) => value.interrupt(),
        }
    }
}

struct InterruptRegistration(u64);

impl Drop for InterruptRegistration {
    fn drop(&mut self) {
        if let Ok(mut values) = INTERRUPTERS.lock() {
            values.remove(&self.0);
        }
    }
}

fn register_interrupt(
    handle: u64,
    value: ActiveInterrupter,
) -> Result<InterruptRegistration, Status> {
    let mut values = INTERRUPTERS.lock().map_err(|_| Status::Internal)?;
    if values.insert(handle, value).is_some() {
        return Err(Status::Busy);
    }
    Ok(InterruptRegistration(handle))
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_application_runtime_abi_version() -> u32 {
    ABI_VERSION
}

#[unsafe(no_mangle)]
/// Creates a runtime and writes its nonzero registry handle.
///
/// # Safety
///
/// `out_handle` must be non-null, aligned, and writable for one `u64`.
pub unsafe extern "C" fn float_view_application_runtime_create(out_handle: *mut u64) -> i32 {
    contain(|| {
        if !valid_scalar(out_handle) {
            return Status::InvalidArgument;
        }
        with_registry(|registry| {
            registry.next_handle = registry
                .next_handle
                .checked_add(1)
                .ok_or(Status::Internal)?;
            let handle = registry.next_handle;
            registry
                .entries
                .insert(handle, Arc::new(Mutex::new(Entry::new())));
            unsafe { out_handle.write(handle) };
            Ok(())
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_application_runtime_destroy(handle: u64) -> i32 {
    contain(|| {
        with_registry(|registry| {
            registry.entries.remove(&handle);
            Ok(())
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_application_runtime_interrupt(handle: u64) -> i32 {
    contain(|| {
        let values = match INTERRUPTERS.lock() {
            Ok(values) => values,
            Err(_) => return Status::Internal,
        };
        if let Some(value) = values.get(&handle) {
            value.interrupt();
        }
        Status::Ok
    })
}

#[unsafe(no_mangle)]
/// Applies one JSON command and records the required result-buffer length.
///
/// # Safety
///
/// `command` must identify `command_length` readable bytes. `out_required` must
/// be non-null, aligned, and writable for one `usize`; the two regions must not
/// overlap.
pub unsafe extern "C" fn float_view_application_runtime_apply(
    handle: u64,
    command: *const u8,
    command_length: usize,
    out_required: *mut usize,
) -> i32 {
    contain(|| {
        if !valid_scalar(out_required) {
            return Status::InvalidArgument;
        }
        if command_length == 0
            || command_length > MAX_COMMAND_BYTES
            || !valid_region(command, command_length)
            || regions_overlap(
                command.cast(),
                command_length,
                out_required.cast(),
                std::mem::size_of::<usize>(),
            )
        {
            return Status::InvalidArgument;
        }
        unsafe { out_required.write(0) };
        let bytes = unsafe { slice::from_raw_parts(command, command_length) };
        let Ok(envelope) = serde_json::from_slice::<CommandEnvelope>(bytes) else {
            return Status::InvalidEnvelope;
        };
        with_entry(handle, |entry| {
            if entry.pending.is_some() {
                return Err(Status::ResultPending);
            }
            let consumed_token = envelope.consumed_token();
            let mut value = if let Some(value) = entry.direct(handle, &envelope) {
                value?
            } else {
                let command = entry.command(envelope)?;
                let effects = entry.runtime.apply(command).map_err(runtime_status)?;
                entry.consume_token(consumed_token)?;
                entry.effects(effects)?
            };
            let generation = entry.runtime.snapshot().application.generation.0;
            value
                .as_object_mut()
                .ok_or(Status::Internal)?
                .insert("generation".to_owned(), generation.into());
            entry.recent_token = None;
            let result = serde_json::to_vec(&value).map_err(|_| Status::Internal)?;
            if result.len() > MAX_RESULT_BYTES {
                return Err(Status::RuntimeFailure);
            }
            unsafe { out_required.write(result.len()) };
            entry.pending = Some(result);
            Ok(())
        })
    })
}

#[unsafe(no_mangle)]
/// Copies the pending JSON result into caller-owned storage.
///
/// # Safety
///
/// `output` must identify `output_capacity` writable bytes when the capacity is
/// nonzero. `out_written` must be non-null, aligned, and writable for one
/// `usize`; the two regions must not overlap.
pub unsafe extern "C" fn float_view_application_runtime_copy_result(
    handle: u64,
    output: *mut u8,
    output_capacity: usize,
    out_written: *mut usize,
) -> i32 {
    contain(|| {
        if !valid_scalar(out_written) {
            return Status::InvalidArgument;
        }
        if output_capacity > 0
            && (!valid_region(output, output_capacity)
                || regions_overlap(
                    output.cast(),
                    output_capacity,
                    out_written.cast(),
                    std::mem::size_of::<usize>(),
                ))
        {
            return Status::InvalidArgument;
        }
        unsafe { out_written.write(0) };
        with_entry(handle, |entry| {
            let result = entry.pending.as_ref().ok_or(Status::InvalidEnvelope)?;
            unsafe { out_written.write(result.len()) };
            if output_capacity < result.len() || !valid_region(output, result.len()) {
                return Err(Status::BufferTooSmall);
            }
            let destination = unsafe { slice::from_raw_parts_mut(output, result.len()) };
            destination.copy_from_slice(result);
            entry.pending = None;
            Ok(())
        })
    })
}

impl Entry {
    fn direct(&mut self, handle: u64, envelope: &CommandEnvelope) -> Option<Result<Value, Status>> {
        use CommandEnvelope as E;
        Some(match envelope {
            E::SessionMetadata { path } => session_metadata(path),
            E::SessionFields { path } => session_fields(path),
            E::SessionSamples {
                path,
                fields,
                start_elapsed_ms,
                end_elapsed_ms,
                limit,
                after_elapsed_ms,
                after_sequence,
                boundary,
            } => match optional_sample_key(*after_elapsed_ms, *after_sequence) {
                Ok(after) => session_samples(SessionSampleQuery {
                    handle,
                    path,
                    fields,
                    start_elapsed_ms: *start_elapsed_ms,
                    end_elapsed_ms: *end_elapsed_ms,
                    limit: *limit,
                    after,
                    boundary: (*boundary).into(),
                }),
                Err(error) => Err(error),
            },
            E::SessionReduce {
                path,
                fields,
                start_elapsed_ms,
                end_elapsed_ms,
                buckets,
                gap_seconds,
                boundary,
            } => session_reduce(SessionReduceQuery {
                handle,
                path,
                fields,
                start_elapsed_ms: *start_elapsed_ms,
                end_elapsed_ms: *end_elapsed_ms,
                buckets: *buckets,
                gap_seconds: *gap_seconds,
                boundary: (*boundary).into(),
            }),
            E::SessionEvents {
                path,
                start_elapsed_ms,
                end_elapsed_ms,
                limit,
                after_elapsed_ms,
                after_sequence,
                boundary,
            } => match optional_sample_key(*after_elapsed_ms, *after_sequence) {
                Ok(after) => session_events(
                    handle,
                    path,
                    *start_elapsed_ms,
                    *end_elapsed_ms,
                    *limit,
                    after,
                    (*boundary).into(),
                ),
                Err(error) => Err(error),
            },
            E::SessionLocations {
                path,
                start_elapsed_ms,
                end_elapsed_ms,
                limit,
                after_location_sequence,
                boundary,
            } => session_locations(
                handle,
                path,
                *start_elapsed_ms,
                *end_elapsed_ms,
                *limit,
                *after_location_sequence,
                (*boundary).into(),
            ),
            E::SessionPrune {
                path,
                target_resolution_seconds,
                commit,
                committed_at_epoch_ms,
            } => session_prune(
                handle,
                path,
                *target_resolution_seconds,
                *commit,
                *committed_at_epoch_ms,
            ),
            _ => return None,
        })
    }

    fn command(&mut self, envelope: CommandEnvelope) -> Result<runtime::Command, Status> {
        use CommandEnvelope as E;
        Ok(match envelope {
            E::Begin {
                now_ms,
                idle_interval_ms,
                post_ride_tail_ms,
                preferred_controller_can_id,
                preferred_bms_can_id,
                routing_hint_can_id,
                routing_hint_controller_uuid,
                has_active_session,
                continuity_can_id,
                continuity_controller_uuid,
            } => {
                let routing_hint =
                    optional_routing_hint(routing_hint_can_id, routing_hint_controller_uuid)?;
                let continuity = coordinator::discovery::Continuity::new(
                    has_active_session,
                    optional_route(continuity_can_id)?,
                    optional_uuid(continuity_controller_uuid)?,
                );
                runtime::Command::Application(coordinator::Command::Begin(
                    coordinator::BeginFacts {
                        now_ms,
                        settings: coordinator::telemetry::Settings::new(
                            idle_interval_ms,
                            post_ride_tail_ms,
                        )
                        .map_err(|_| Status::InvalidEnvelope)?,
                        preferred_controller_can_id,
                        routing_hint,
                        continuity,
                        preferred_bms_can_id,
                    },
                ))
            }
            E::SelectController {
                generation,
                can_id,
                now_ms,
            } => runtime::Command::Application(coordinator::Command::SelectController {
                generation: coordinator::Generation(generation),
                can_id,
                now_ms,
            }),
            E::ReceiveBytes {
                generation,
                bytes,
                now_ms,
                observed_at_epoch_ms,
            } => runtime::Command::ReceiveBytes {
                generation: coordinator::Generation(generation),
                bytes,
                now_ms,
                observed_at_epoch_ms,
            },
            E::DeadlineFired {
                generation,
                token,
                now_ms,
            } => runtime::Command::Application(coordinator::Command::DeadlineFired {
                generation: coordinator::Generation(generation),
                token: self.deadline(token)?,
                now_ms,
            }),
            E::TimerFired {
                generation,
                token,
                now_ms,
            } => runtime::Command::Application(coordinator::Command::TimerFired {
                generation: coordinator::Generation(generation),
                token: self.timer(token)?,
                now_ms,
            }),
            E::Activate {
                generation,
                now_ms,
                session_started,
                cached_configuration_schema,
            } => runtime::Command::Application(coordinator::Command::Activate {
                generation: coordinator::Generation(generation),
                now_ms,
                session_started,
                cached_configuration_schema,
            }),
            E::SetControl {
                generation,
                control,
                enabled,
                now_ms,
            } => runtime::Command::Application(coordinator::Command::SetControl {
                generation: coordinator::Generation(generation),
                control: match control {
                    Control::Package => coordinator::mutation::BooleanControl::Package,
                    Control::Flywheel => coordinator::mutation::BooleanControl::Flywheel,
                    Control::Leds => coordinator::mutation::BooleanControl::Leds,
                    Control::Headlights => coordinator::mutation::BooleanControl::Headlights,
                },
                enabled,
                now_ms,
            }),
            E::ClearFatalError { generation, now_ms } => {
                runtime::Command::Application(coordinator::Command::ClearFatalError {
                    generation: coordinator::Generation(generation),
                    now_ms,
                })
            }
            E::ShutdownController { generation, now_ms } => {
                runtime::Command::Application(coordinator::Command::ShutdownController {
                    generation: coordinator::Generation(generation),
                    now_ms,
                })
            }
            E::ReadConfiguration {
                generation,
                defaults,
                now_ms,
            } => runtime::Command::Application(coordinator::Command::ReadConfiguration {
                generation: coordinator::Generation(generation),
                defaults,
                now_ms,
            }),
            E::WriteConfiguration {
                generation,
                mutation,
                package_version,
                fingerprint,
                wire_signature,
                base_revision,
                values,
                now_ms,
            } => runtime::Command::Application(coordinator::Command::WriteConfiguration {
                generation: coordinator::Generation(generation),
                mutation: match mutation {
                    ConfigurationMutation::Write => {
                        coordinator::mutation::ConfigurationMutation::Write
                    }
                    ConfigurationMutation::TuneApply => {
                        coordinator::mutation::ConfigurationMutation::TuneApply
                    }
                },
                package_version,
                fingerprint,
                wire_signature,
                base_revision,
                values: values
                    .into_iter()
                    .map(|(name, value)| {
                        let value = match value {
                            ConfigurationValueEnvelope::Double(value) if value.is_finite() => {
                                coordinator::configuration_codec::ConfigurationValue::Decimal(value)
                            }
                            ConfigurationValueEnvelope::Double(_) => {
                                return Err(Status::InvalidEnvelope);
                            }
                            ConfigurationValueEnvelope::Integer(value) => {
                                coordinator::configuration_codec::ConfigurationValue::Integer(value)
                            }
                            ConfigurationValueEnvelope::Boolean(value) => {
                                coordinator::configuration_codec::ConfigurationValue::Boolean(value)
                            }
                            ConfigurationValueEnvelope::String(value) => {
                                coordinator::configuration_codec::ConfigurationValue::String(value)
                            }
                        };
                        Ok((name, value))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                now_ms,
            }),
            E::UpdateSettings {
                generation,
                idle_interval_ms,
                post_ride_tail_ms,
                now_ms,
            } => runtime::Command::Application(coordinator::Command::UpdateSettings {
                generation: coordinator::Generation(generation),
                settings: coordinator::telemetry::Settings::new(
                    idle_interval_ms,
                    post_ride_tail_ms,
                )
                .map_err(|_| Status::InvalidEnvelope)?,
                now_ms,
            }),
            E::Refresh { generation, now_ms } => {
                runtime::Command::Application(coordinator::Command::Refresh {
                    generation: coordinator::Generation(generation),
                    now_ms,
                })
            }
            E::Reset { now_ms } => {
                runtime::Command::Application(coordinator::Command::Reset { now_ms })
            }
            E::AttachSession {
                path,
                mode,
                expected_riding_persistence_ms,
                expected_idle_persistence_ms,
                committed_at_epoch_ms,
            } => runtime::Command::AttachSession {
                path: PathBuf::from(path),
                mode: match mode {
                    SessionMode::Create => runtime::SessionMode::Create,
                    SessionMode::Open => runtime::SessionMode::Open,
                },
                metadata: runtime::session::SessionMetadata {
                    expected_riding_persistence_ms,
                    expected_idle_persistence_ms,
                    committed_at_epoch_ms,
                },
            },
            E::AppendLocations { locations } => runtime::Command::AppendLocations(
                locations
                    .into_iter()
                    .map(|value| runtime::session::LocationObservation {
                        observed_at_epoch_ms: value.observed_at_epoch_ms,
                        elapsed_ms: value.elapsed_ms,
                        latitude: value.latitude,
                        longitude: value.longitude,
                        accuracy_m: value.accuracy_m,
                    })
                    .collect(),
            ),
            E::DetachSession => runtime::Command::DetachSession,
            E::SessionMetadata { .. }
            | E::SessionFields { .. }
            | E::SessionSamples { .. }
            | E::SessionReduce { .. }
            | E::SessionEvents { .. }
            | E::SessionLocations { .. }
            | E::SessionPrune { .. } => return Err(Status::Internal),
        })
    }

    fn effects(&mut self, effects: Vec<runtime::Effect>) -> Result<Value, Status> {
        let values = effects
            .into_iter()
            .map(|effect| self.effect(effect))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({ "version": ABI_VERSION, "effects": values }))
    }

    fn effect(&mut self, effect: runtime::Effect) -> Result<Value, Status> {
        Ok(match effect {
            runtime::Effect::WriteBytes(bytes) => {
                json!({ "effect": "write_bytes", "bytes": bytes })
            }
            runtime::Effect::PublishSessionMarker(marker) => session_marker(&marker),
            runtime::Effect::Failure(failure) => {
                json!({ "effect": "failure", "kind": format!("{failure:?}") })
            }
            runtime::Effect::Application(effect) => self.application_effect(effect)?,
        })
    }

    fn application_effect(&mut self, effect: coordinator::Effect) -> Result<Value, Status> {
        Ok(match effect {
            coordinator::Effect::CancelDeadline(token) => json!({
                "effect": "cancel_deadline",
                "token": self.remove_deadline(token)?,
            }),
            coordinator::Effect::ArmDeadline { token, timeout_ms } => json!({
                "effect": "arm_deadline",
                "token": self.allocate_deadline(token)?,
                "timeout_ms": timeout_ms,
            }),
            coordinator::Effect::Write(_) => return Err(Status::Internal),
            coordinator::Effect::ReplaceTimer { token, interval_ms } => json!({
                "effect": "replace_timer",
                "token": self.allocate_timer(token)?,
                "interval_ms": interval_ms,
            }),
            coordinator::Effect::CancelTimer(token) => json!({
                "effect": "cancel_timer",
                "token": self.remove_timer(token)?,
            }),
            coordinator::Effect::ClearRoutingHint => json!({ "effect": "clear_routing_hint" }),
            coordinator::Effect::StoreRoutingHint(value) => json!({
                "effect": "store_routing_hint",
                "route": route(value.route),
                "identity": firmware_identity(&value.identity),
            }),
            coordinator::Effect::PrepareActivation(value) => json!({
                "effect": "prepare_activation",
                "generation": value.generation.0,
                "selected_route": route(value.selected.route),
                "selected_identity": firmware_identity(&value.selected.identity),
                "transport_identity": firmware_identity(&value.transport),
                "refloat_version": value.refloat_version.to_string(),
                "inventory_complete": value.inventory_complete,
                "inventory": value.inventory.iter().map(|item| json!({
                    "route": route(item.route),
                    "identity": firmware_identity(&item.identity),
                })).collect::<Vec<_>>(),
                "configuration_cache_identity": value.configuration_cache_identity.as_ref().map(cache_identity),
            }),
            coordinator::Effect::PublishState(value) => {
                state_snapshot(&value, self.runtime.configuration_schema())
            }
            coordinator::Effect::PublishTelemetry(value) => telemetry_sample(&value),
            coordinator::Effect::PublishBmsState(value) => bms_state(&value),
            coordinator::Effect::PublishBmsObservation(value) => bms_observation(&value),
            coordinator::Effect::ApplyConfigurationCache(value) => match value.into_inner() {
                coordinator::configuration::CacheEffect::Store { identity, bytes } => json!({
                    "effect": "store_configuration_cache",
                    "identity": cache_identity(&identity),
                    "bytes": bytes,
                }),
                coordinator::configuration::CacheEffect::Remove { identity } => json!({
                    "effect": "remove_configuration_cache",
                    "identity": cache_identity(&identity),
                }),
            },
            coordinator::Effect::Mutation(value) => {
                json!({ "effect": "mutation", "value": format!("{value:?}") })
            }
            coordinator::Effect::Failure(value) => {
                json!({ "effect": "failure", "kind": format!("{value:?}") })
            }
            coordinator::Effect::Diagnostic(value) => {
                json!({ "effect": "diagnostic", "kind": format!("{value:?}") })
            }
        })
    }
}

fn optional_routing_hint(
    can_id: Option<u8>,
    uuid: Option<Vec<u8>>,
) -> Result<Option<coordinator::discovery::RoutingHint>, Status> {
    match (can_id, optional_uuid(uuid)?) {
        (None, None) => Ok(None),
        (Some(can_id), Some(uuid)) => coordinator::discovery::RoutingHint::new(can_id, uuid)
            .map(Some)
            .map_err(|_| Status::InvalidEnvelope),
        _ => Err(Status::InvalidEnvelope),
    }
}

fn runtime_status(error: runtime::RuntimeError) -> Status {
    use coordinator::CoordinatorError;
    use coordinator::configuration::ServiceError as ConfigurationError;
    use coordinator::mutation::CoordinatorError as MutationError;
    match error {
        runtime::RuntimeError::Coordinator(CoordinatorError::Mutation(
            MutationError::InvalidState,
        ))
        | runtime::RuntimeError::Coordinator(CoordinatorError::Configuration(
            ConfigurationError::InvalidState
            | ConfigurationError::StaleResponse
            | ConfigurationError::UnauditedSchema
            | ConfigurationError::MalformedResponse
            | ConfigurationError::Codec(_),
        )) => Status::InvalidState,
        runtime::RuntimeError::Coordinator(CoordinatorError::Configuration(
            ConfigurationError::Conflict,
        )) => Status::Conflict,
        runtime::RuntimeError::Coordinator(CoordinatorError::Configuration(
            ConfigurationError::Unavailable,
        )) => Status::Unavailable,
        _ => Status::RuntimeFailure,
    }
}

fn optional_uuid(value: Option<Vec<u8>>) -> Result<Option<[u8; 12]>, Status> {
    value
        .map(|bytes| bytes.try_into().map_err(|_| Status::InvalidEnvelope))
        .transpose()
}

fn optional_route(can_id: Option<u8>) -> Result<Option<coordinator::discovery::Route>, Status> {
    can_id
        .map(|value| coordinator::discovery::Route::can(value).map_err(|_| Status::InvalidEnvelope))
        .transpose()
}

fn route(value: coordinator::discovery::Route) -> Value {
    match value {
        coordinator::discovery::Route::Direct => json!({ "kind": "direct" }),
        coordinator::discovery::Route::Can(can_id) => json!({ "kind": "can", "can_id": can_id }),
    }
}

fn firmware_identity(value: &coordinator::firmware::FirmwareIdentity) -> Value {
    json!({
        "major": value.major,
        "minor": value.minor,
        "hardware_name": value.hardware_name,
        "firmware_name": value.firmware_name,
        "controller_uuid": value.controller_uuid,
        "test_version": value.test_version,
        "hardware_type": value.hardware_type,
        "custom_configuration_count": value.custom_configuration_count,
        "hardware_configuration_crc": value.hardware_configuration_crc,
    })
}

fn state_snapshot(
    value: &coordinator::Snapshot,
    schema: Option<&coordinator::configuration_codec::ConfigurationSchema>,
) -> Value {
    json!({
        "effect": "publish_state",
        "generation": value.generation.0,
        "phase": format!("{:?}", value.phase).to_lowercase(),
        "standalone_bms": value.standalone_bms,
        "controller_route": value.discovery.route.map(route),
        "selected_firmware": value.discovery.selected.as_ref().map(|selected| firmware_identity(&selected.identity)),
        "transport_firmware": value.discovery.transport.as_ref().map(firmware_identity),
        "controller_candidates": value.discovery.candidates.iter().map(|candidate| json!({
            "can_id": candidate.can_id,
            "hardware_name": candidate.hardware_name,
            "firmware_name": candidate.firmware_name,
            "vesc_version": candidate.vesc_version,
            "vesc_supported": candidate.vesc_supported,
        })).collect::<Vec<_>>(),
        "connection_issue": value.discovery.issue.map(|issue| json!({
            "code": issue.code.as_str(),
            "message": issue.message,
        })),
        "controls": value.mutations.controls.as_ref().map(controls),
        "controller_actions": {
            "shutdown_available": value.mutations.controller_actions.shutdown_available,
            "shutdown_pending": value.mutations.controller_actions.shutdown_pending,
        },
        "configuration": configuration_state(&value.configuration, schema),
        "current_limits": value.current_limits.as_ref().map(|limits| json!({
            "motor_current_max_a": limits.motor_current_max_a,
            "motor_current_min_a": limits.motor_current_min_a,
            "battery_current_max_a": limits.battery_current_max_a,
            "battery_current_min_a": limits.battery_current_min_a,
            "battery_series_cells": limits.battery_series_cells,
        })),
    })
}

fn controls(value: &coordinator::mutation::Controls) -> Value {
    json!({
        "status": format!("{:?}", value.status).to_lowercase(),
        "writable": value.writable,
        "package_enabled": value.package_enabled,
        "flywheel_enabled": value.flywheel_enabled,
        "lighting_available": value.lighting_available,
        "leds_enabled": value.leds_enabled,
        "headlights_enabled": value.headlights_enabled,
        "fatal_error_clear_available": value.fatal_error_clear_available,
        "fatal_error_latched": value.fatal_error_latched,
        "fatal_error_active": value.fatal_error_active,
        "fatal_error_clearable": value.fatal_error_clearable,
        "pending_action": value.pending_action.map(|pending| format!("{pending:?}").to_lowercase()),
    })
}

fn configuration_state(
    value: &coordinator::configuration::Snapshot,
    schema: Option<&coordinator::configuration_codec::ConfigurationSchema>,
) -> Value {
    json!({
        "availability": format!("{:?}", value.availability).to_lowercase(),
        "writable": value.writable,
        "pending": value.pending.map(|pending| format!("{pending:?}").to_lowercase()),
        "identity": value.identity.as_ref().map(|identity| json!({
            "package_version": identity.package_version,
            "fingerprint": identity.fingerprint,
            "wire_signature": identity.wire_signature,
        })),
        "current_revision": value.current.as_ref().map(|loaded| loaded.revision),
        "default_revision": value.defaults.as_ref().map(|loaded| loaded.revision),
        "current_values": value.current.as_ref().map(|loaded| configuration_values(&loaded.values)),
        "default_values": value.defaults.as_ref().map(|loaded| configuration_values(&loaded.values)),
        "schema": schema.map(configuration_schema),
    })
}

fn configuration_values(values: &coordinator::configuration_codec::ConfigurationValues) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(name, value)| (name.clone(), configuration_value(value)))
            .collect(),
    )
}

fn configuration_value(value: &coordinator::configuration_codec::ConfigurationValue) -> Value {
    use coordinator::configuration_codec::ConfigurationValue as C;
    match value {
        C::Decimal(value) => json!({ "type": "double", "value": value }),
        C::Integer(value) => json!({ "type": "integer", "value": value }),
        C::Boolean(value) => json!({ "type": "boolean", "value": value }),
        C::String(value) => json!({ "type": "string", "value": value }),
    }
}

fn configuration_schema(value: &coordinator::configuration_codec::ConfigurationSchema) -> Value {
    json!({
        "fingerprint": value.fingerprint,
        "wire_signature": value.wire_signature,
        "fields": value.fields.iter().map(|field| json!({
            "name": field.name,
            "long_name": field.long_name,
            "field_type": debug_lower(field.field_type),
            "wire_type": field.wire_type.map(debug_lower),
            "default_value": configuration_value(&field.default_value),
            "minimum": field.minimum,
            "maximum": field.maximum,
            "step": field.step,
            "precision": field.precision,
            "scale": field.scale,
            "suffix": field.suffix,
            "choices": field.choices,
            "description": field.description,
        })).collect::<Vec<_>>(),
        "groups": value.groups.iter().map(|group| json!({
            "name": group.name,
            "subgroups": group.subgroups.iter().map(|subgroup| json!({
                "name": subgroup.name,
                "fields": subgroup.fields,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn telemetry_sample(value: &coordinator::telemetry::NormalizedSample) -> Value {
    let mut object = serde_json::to_value(TelemetryProjection::from(value))
        .expect("serializable telemetry projection");
    object["effect"] = Value::String("publish_telemetry".to_owned());
    object
}

#[derive(serde::Serialize)]
struct TelemetryProjection<'a> {
    sequence: u64,
    observed_at_epoch_ms: i64,
    slow_observed_at_epoch_ms: Option<i64>,
    elapsed_ms: u64,
    ride_state: String,
    package_state: Option<String>,
    package_mode: Option<String>,
    footpad_state: Option<String>,
    stop_condition: Option<String>,
    setpoint_adjustment: Option<String>,
    wheelslip: Option<bool>,
    darkride: Option<bool>,
    charging: Option<bool>,
    alert_reason: Option<String>,
    firmware_fault: Option<String>,
    fatal_error_latched: Option<bool>,
    #[serde(flatten)]
    numbers: BTreeMap<&'static str, Option<f64>>,
    extensions: Vec<ExtensionProjection<'a>>,
}

#[derive(serde::Serialize)]
struct ExtensionProjection<'a> {
    name: &'a str,
    value: f64,
}

impl<'a> From<&'a coordinator::telemetry::NormalizedSample> for TelemetryProjection<'a> {
    fn from(value: &'a coordinator::telemetry::NormalizedSample) -> Self {
        let numbers = BTreeMap::from([
            ("pack_voltage_v", value.pack_voltage_v),
            ("duty_cycle", value.duty_cycle),
            ("controller_temp_c", value.controller_temp_c),
            ("motor_temp_c", value.motor_temp_c),
            ("motor_current_a", value.motor_current_a),
            ("battery_current_a", value.battery_current_a),
            ("erpm", value.erpm),
            ("speed_kph", value.speed_kph),
            ("pitch_deg", value.pitch_deg),
            ("balance_pitch_deg", value.balance_pitch_deg),
            ("roll_deg", value.roll_deg),
            (
                "directional_motor_current_a",
                value.directional_motor_current_a,
            ),
            ("filtered_motor_current_a", value.filtered_motor_current_a),
            (
                "requested_balance_current_a",
                value.requested_balance_current_a,
            ),
            ("booster_current_a", value.booster_current_a),
            ("field_weakening_current_a", value.field_weakening_current_a),
            ("footpad_adc1_v", value.footpad_adc1_v),
            ("footpad_adc2_v", value.footpad_adc2_v),
            ("remote_input", value.remote_input),
            ("setpoint_deg", value.setpoint_deg),
            ("atr_setpoint_deg", value.atr_setpoint_deg),
            ("brake_tilt_setpoint_deg", value.brake_tilt_setpoint_deg),
            ("torque_tilt_setpoint_deg", value.torque_tilt_setpoint_deg),
            ("turn_tilt_setpoint_deg", value.turn_tilt_setpoint_deg),
            ("remote_setpoint_deg", value.remote_setpoint_deg),
            (
                "atr_acceleration_difference_erpm_per_second",
                value.atr_acceleration_difference_erpm_per_second,
            ),
            ("atr_speed_boost", value.atr_speed_boost),
            ("distance_m", value.distance_m),
            ("odometer_m", value.odometer_m),
            ("amp_hours_drawn_ah", value.amp_hours_drawn_ah),
            ("amp_hours_charged_ah", value.amp_hours_charged_ah),
            ("watt_hours_drawn_wh", value.watt_hours_drawn_wh),
            ("watt_hours_charged_wh", value.watt_hours_charged_wh),
            ("battery_level", value.battery_level),
            ("charging_current_a", value.charging_current_a),
            ("charging_voltage_v", value.charging_voltage_v),
        ]);
        Self {
            sequence: value.sequence,
            observed_at_epoch_ms: value.observed_at_epoch_ms,
            slow_observed_at_epoch_ms: value.slow_observed_at_epoch_ms,
            elapsed_ms: value.elapsed_ms,
            ride_state: debug_lower(value.ride_state),
            package_state: value.package_state.map(debug_lower),
            package_mode: value.package_mode.map(debug_lower),
            footpad_state: value.footpad_state.map(debug_lower),
            stop_condition: value.stop_condition.map(debug_lower),
            setpoint_adjustment: value.setpoint_adjustment.map(debug_lower),
            wheelslip: value.wheelslip,
            darkride: value.darkride,
            charging: value.charging,
            alert_reason: value.alert_reason.map(debug_lower),
            firmware_fault: value.firmware_fault.map(debug_lower),
            fatal_error_latched: value.fatal_error_latched,
            numbers,
            extensions: value
                .extensions
                .iter()
                .map(|extension| ExtensionProjection {
                    name: &extension.name,
                    value: extension.value,
                })
                .collect(),
        }
    }
}

fn bms_state(value: &coordinator::bms::Snapshot) -> Value {
    json!({
        "effect": "publish_bms_state",
        "status": value.status.as_str(),
        "route": value.route.map(route_bms),
        "candidates": value.candidates.iter().map(|candidate| json!({
            "can_id": candidate.route.can_id(),
            "hardware_name": candidate.hardware_name,
            "version": candidate.vesc_version,
        })).collect::<Vec<_>>(),
        "consecutive_failures": value.consecutive_failures,
        "has_observation": value.has_observation,
        "issue": value.issue.map(|issue| format!("{issue:?}").to_lowercase()),
    })
}

fn bms_observation(value: &coordinator::bms::BmsObservation) -> Value {
    json!({
        "effect": "publish_bms_observation",
        "route": route_bms(value.route),
        "observed_at_epoch_ms": value.observed_at_epoch_ms,
        "total_voltage_v": value.total_voltage_v,
        "charge_voltage_v": value.charge_voltage_v,
        "input_current_a": value.input_current_a,
        "input_ic_current_a": value.input_ic_current_a,
        "session_amp_hours_ah": value.session_amp_hours_ah,
        "session_watt_hours_wh": value.session_watt_hours_wh,
        "cell_voltages_v": value.cell_voltages_v,
        "balancing": value.balancing,
        "temperatures_c": value.temperatures_c,
        "ic_temperature_c": value.ic_temperature_c,
        "humidity_sensor_temperature_c": value.humidity_sensor_temperature_c,
        "humidity_percent": value.humidity_percent,
        "highest_cell_temperature_c": value.highest_cell_temperature_c,
        "state_of_charge": value.state_of_charge,
        "state_of_health": value.state_of_health,
        "reported_can_id": value.reported_can_id,
        "lifetime_charge_amp_hours_ah": value.lifetime_charge_amp_hours_ah,
        "lifetime_charge_watt_hours_wh": value.lifetime_charge_watt_hours_wh,
        "lifetime_discharge_amp_hours_ah": value.lifetime_discharge_amp_hours_ah,
        "lifetime_discharge_watt_hours_wh": value.lifetime_discharge_watt_hours_wh,
        "pressure_pa": value.pressure_pa,
        "data_version": value.data_version,
        "status": value.status,
    })
}

fn route_bms(value: coordinator::bms::Route) -> Value {
    match value {
        coordinator::bms::Route::Direct => json!({ "kind": "direct" }),
        coordinator::bms::Route::Can(can_id) => json!({ "kind": "can", "can_id": can_id }),
    }
}

fn cache_identity(value: &coordinator::configuration::CacheIdentity) -> Value {
    json!({
        "firmware_version": value.firmware_version,
        "hardware_name": value.hardware_name,
        "package_version": value.package_version,
        "custom_index": value.custom_index,
        "hardware_configuration_crc": value.hardware_configuration_crc,
    })
}

fn session_marker(value: &runtime::session::CommitMarker) -> Value {
    json!({
        "effect": "publish_session_marker",
        "committed_sample_sequence": value.committed_sample_sequence,
        "committed_event_sequence": value.committed_event_sequence,
        "recorded_sample_count": value.recorded_sample_count,
        "stored_sample_count": value.stored_sample_count,
        "committed_event_count": value.committed_event_count,
        "committed_elapsed_ms": value.committed_elapsed_ms,
        "history_revision": value.history_revision,
        "committed_at_epoch_ms": value.committed_at_epoch_ms,
        "riding_elapsed_ms": value.riding_elapsed_ms,
        "distance_m": value.distance_m,
        "maximum_speed_kph": value.maximum_speed_kph,
        "maximum_speed_source_sequence": value.maximum_speed_source_sequence,
        "maximum_duty_cycle": value.maximum_duty_cycle,
        "maximum_duty_source_sequence": value.maximum_duty_source_sequence,
        "last_speed_kph": value.last_speed_kph,
        "last_ride_state": value.last_ride_state,
        "committed_location_sequence": value.committed_location_sequence,
        "committed_location_elapsed_ms": value.committed_location_elapsed_ms,
        "recorded_location_count": value.recorded_location_count,
        "stored_location_count": value.stored_location_count,
        "last_location_status": value.last_location_status,
    })
}

fn debug_lower(value: impl std::fmt::Debug) -> String {
    format!("{value:?}").to_lowercase()
}

fn session_result(operation: &str, value: Value) -> Value {
    json!({
        "version": ABI_VERSION,
        "effects": [{ "effect": "session_result", "operation": operation, "value": value }],
    })
}

fn session_path(path: &str) -> Result<PathBuf, Status> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty()
        || path.as_os_str().as_encoded_bytes().len() > runtime::MAX_SESSION_PATH_BYTES
        || !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(Status::InvalidEnvelope);
    }
    Ok(path)
}

fn open_reader(path: &str) -> Result<runtime::session::SessionReader, Status> {
    runtime::ApplicationRuntime::open_reader(session_path(path)?)
        .map_err(|_| Status::RuntimeFailure)
}

fn optional_sample_key(
    elapsed_ms: Option<u64>,
    sequence: Option<u64>,
) -> Result<Option<runtime::session::SampleKey>, Status> {
    match (elapsed_ms, sequence) {
        (None, None) => Ok(None),
        (Some(elapsed_ms), Some(sequence)) => Ok(Some(runtime::session::SampleKey {
            elapsed_ms,
            sequence,
        })),
        _ => Err(Status::InvalidEnvelope),
    }
}

fn sample_key(key: runtime::session::SampleKey) -> Value {
    json!({ "elapsed_ms": key.elapsed_ms, "sequence": key.sequence })
}

fn session_metadata(path: &str) -> Result<Value, Status> {
    let mut reader = open_reader(path)?;
    let metadata = reader.metadata().map_err(|_| Status::RuntimeFailure)?;
    let marker = reader.marker().map_err(|_| Status::RuntimeFailure)?;
    Ok(session_result(
        "metadata",
        json!({
            "expected_riding_persistence_ms": metadata.expected_riding_persistence_ms,
            "expected_idle_persistence_ms": metadata.expected_idle_persistence_ms,
            "pruned_at_epoch_ms": metadata.pruned_at_epoch_ms,
            "retained_resolution_seconds": metadata.retained_resolution_seconds,
            "prune_policy": metadata.prune_policy,
            "history_revision": marker.history_revision,
            "recorded_sample_count": marker.recorded_sample_count,
            "stored_sample_count": marker.stored_sample_count,
            "sample_sequence": marker.committed_sample_sequence,
            "event_sequence": marker.committed_event_sequence,
            "location_sequence": marker.committed_location_sequence,
            "stored_location_count": marker.stored_location_count,
        }),
    ))
}

fn session_fields(path: &str) -> Result<Value, Status> {
    let mut reader = open_reader(path)?;
    let fields = reader.fields().map_err(|_| Status::RuntimeFailure)?;
    Ok(session_result(
        "fields",
        json!(
            fields
                .into_iter()
                .map(|field| json!({
                    "name": field.name,
                    "source": match field.source {
                        runtime::session::FieldSource::Stable => "stable",
                        runtime::session::FieldSource::Extension => "extension",
                    },
                    "first_sequence": field.first_sequence,
                }))
                .collect::<Vec<_>>()
        ),
    ))
}

struct SessionSampleQuery<'a> {
    handle: u64,
    path: &'a str,
    fields: &'a [String],
    start_elapsed_ms: u64,
    end_elapsed_ms: u64,
    limit: usize,
    after: Option<runtime::session::SampleKey>,
    boundary: runtime::session::PublishedBoundary,
}

fn session_samples(query: SessionSampleQuery<'_>) -> Result<Value, Status> {
    let mut reader = open_reader(query.path)?;
    let _interrupt = register_interrupt(
        query.handle,
        ActiveInterrupter::Reader(reader.interrupter()),
    )?;
    let page = reader
        .sample_page(
            query.fields,
            query.start_elapsed_ms,
            query.end_elapsed_ms,
            query.limit,
            query.after,
            query.boundary,
        )
        .map_err(|_| Status::RuntimeFailure)?;
    Ok(session_result(
        "samples",
        json!({
            "rows": page.rows.into_iter().map(|row| json!({
                "elapsed_ms": row.key.elapsed_ms,
                "sequence": row.key.sequence,
                "ride_state": row.ride_state,
                "values": row.values,
            })).collect::<Vec<_>>(),
            "next": page.next.map(sample_key),
        }),
    ))
}

struct SessionReduceQuery<'a> {
    handle: u64,
    path: &'a str,
    fields: &'a [String],
    start_elapsed_ms: u64,
    end_elapsed_ms: u64,
    buckets: usize,
    gap_seconds: Option<f64>,
    boundary: runtime::session::PublishedBoundary,
}

fn session_reduce(query: SessionReduceQuery<'_>) -> Result<Value, Status> {
    let gap_policy = match query.gap_seconds {
        None => runtime::history::GapPolicy::Automatic,
        Some(value) => runtime::history::GapPolicy::explicit_seconds(value)
            .map_err(|_| Status::InvalidEnvelope)?,
    };
    let mut reader = open_reader(query.path)?;
    let _interrupt = register_interrupt(
        query.handle,
        ActiveInterrupter::Reader(reader.interrupter()),
    )?;
    let value = reader
        .reduce_series(
            query.fields,
            query.start_elapsed_ms,
            query.end_elapsed_ms,
            query.buckets,
            gap_policy,
            query.boundary,
        )
        .map_err(|_| Status::RuntimeFailure)?;
    Ok(session_result(
        "reduce",
        json!({
            "columns": value.columns.into_iter().map(|column| json!({
                "field": column.field,
                "elapsed_ms": column.elapsed_ms,
                "sequences": column.sequences,
                "values": column.values,
                "generations": column.generations,
            })).collect::<Vec<_>>(),
            "source_rows": value.source_rows,
        }),
    ))
}

fn session_events(
    handle: u64,
    path: &str,
    start_elapsed_ms: u64,
    end_elapsed_ms: u64,
    limit: usize,
    after: Option<runtime::session::SampleKey>,
    boundary: runtime::session::PublishedBoundary,
) -> Result<Value, Status> {
    let mut reader = open_reader(path)?;
    let _interrupt = register_interrupt(handle, ActiveInterrupter::Reader(reader.interrupter()))?;
    let page = reader
        .event_page(start_elapsed_ms, end_elapsed_ms, limit, after, boundary)
        .map_err(|_| Status::RuntimeFailure)?;
    Ok(session_result(
        "events",
        json!({
            "rows": page.rows.into_iter().map(|row| json!({
                "elapsed_ms": row.key.elapsed_ms,
                "sequence": row.key.sequence,
                "kind": row.kind,
                "details_json": row.details_json,
                "source_sample_sequence": row.source_sample_sequence,
            })).collect::<Vec<_>>(),
            "next": page.next.map(sample_key),
        }),
    ))
}

fn session_locations(
    handle: u64,
    path: &str,
    start_elapsed_ms: u64,
    end_elapsed_ms: u64,
    limit: usize,
    after_location_sequence: Option<u64>,
    boundary: runtime::session::PublishedBoundary,
) -> Result<Value, Status> {
    let mut reader = open_reader(path)?;
    let _interrupt = register_interrupt(handle, ActiveInterrupter::Reader(reader.interrupter()))?;
    let page = reader
        .location_page(
            start_elapsed_ms,
            end_elapsed_ms,
            limit,
            after_location_sequence,
            boundary,
        )
        .map_err(|_| Status::RuntimeFailure)?;
    Ok(session_result(
        "locations",
        json!({
            "rows": page.rows.into_iter().map(|row| json!({
                "location_sequence": row.location_sequence,
                "elapsed_ms": row.elapsed_ms,
                "observed_at_epoch_ms": row.observed_at_epoch_ms,
                "latitude": row.latitude,
                "longitude": row.longitude,
                "accuracy_m": row.accuracy_m,
            })).collect::<Vec<_>>(),
            "gaps": page.gaps.into_iter().map(|gap| json!({
                "start_elapsed_ms": gap.start_elapsed_ms,
                "end_elapsed_ms": gap.end_elapsed_ms,
            })).collect::<Vec<_>>(),
            "published_location_sequence": page.published_location_sequence,
            "next_location_sequence": page.next_location_sequence,
        }),
    ))
}

fn session_prune(
    handle: u64,
    path: &str,
    target_resolution_seconds: u64,
    commit: bool,
    committed_at_epoch_ms: i64,
) -> Result<Value, Status> {
    let mut pruner = runtime::ApplicationRuntime::open_pruner(session_path(path)?)
        .map_err(|_| Status::RuntimeFailure)?;
    let _interrupt = register_interrupt(handle, ActiveInterrupter::Pruner(pruner.interrupter()))?;
    let plan = pruner
        .preview(target_resolution_seconds)
        .map_err(|_| Status::RuntimeFailure)?;
    let preview = plan.preview();
    let (history_revision, stored_location_count, skipped) = if commit {
        let value = pruner
            .commit(plan, committed_at_epoch_ms)
            .map_err(|_| Status::RuntimeFailure)?;
        (
            value.history_revision,
            value.stored_location_count,
            value.skipped,
        )
    } else {
        (preview.history_revision, 0, preview.skipped)
    };
    Ok(session_result(
        "prune",
        json!({
            "recorded_sample_count": preview.recorded_sample_count,
            "stored_sample_count": preview.stored_sample_count,
            "retained_sample_count": preview.retained_sample_count,
            "estimated_reclaimable_bytes": preview.estimated_reclaimable_bytes,
            "target_resolution_seconds": preview.target_resolution_seconds,
            "history_revision": history_revision,
            "stored_location_count": stored_location_count,
            "skipped": skipped,
            "committed": commit,
        }),
    ))
}

fn valid_scalar<T>(pointer: *const T) -> bool {
    !pointer.is_null()
        && (pointer as usize).is_multiple_of(std::mem::align_of::<T>())
        && (pointer as usize)
            .checked_add(std::mem::size_of::<T>().saturating_sub(1))
            .is_some()
}

fn valid_region<T>(pointer: *const T, length: usize) -> bool {
    if length == 0 {
        return true;
    }
    !pointer.is_null()
        && length <= isize::MAX as usize
        && (pointer as usize).checked_add(length - 1).is_some()
}

fn regions_overlap(
    left: *const u8,
    left_length: usize,
    right: *const u8,
    right_length: usize,
) -> bool {
    if left_length == 0 || right_length == 0 {
        return false;
    }
    let left_start = left as usize;
    let right_start = right as usize;
    let Some(left_end) = left_start.checked_add(left_length) else {
        return true;
    };
    let Some(right_end) = right_start.checked_add(right_length) else {
        return true;
    };
    left_start < right_end && right_start < left_end
}

fn with_registry(operation: impl FnOnce(&mut Registry) -> Result<(), Status>) -> Status {
    let mut registry = match REGISTRY.lock() {
        Ok(value) => value,
        Err(_) => return Status::Internal,
    };
    match operation(&mut registry) {
        Ok(()) => Status::Ok,
        Err(status) => status,
    }
}

fn with_entry(handle: u64, operation: impl FnOnce(&mut Entry) -> Result<(), Status>) -> Status {
    let entry = {
        let registry = match REGISTRY.lock() {
            Ok(value) => value,
            Err(_) => return Status::Internal,
        };
        let Some(entry) = registry.entries.get(&handle).cloned() else {
            return Status::InvalidHandle;
        };
        entry
    };
    let mut entry = match entry.try_lock() {
        Ok(value) => value,
        Err(TryLockError::WouldBlock) => return Status::Busy,
        Err(TryLockError::Poisoned(_)) => return Status::Internal,
    };
    match operation(&mut entry) {
        Ok(()) => Status::Ok,
        Err(status) => status,
    }
}

fn contain(operation: impl FnOnce() -> Status) -> i32 {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(Status::Internal) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_and_registry_contention_fail_closed() {
        assert_eq!(
            contain(|| panic!("private payload")),
            Status::Internal as i32
        );
        let entry = Arc::new(Mutex::new(Entry::new()));
        REGISTRY
            .lock()
            .expect("registry lock")
            .entries
            .insert(u64::MAX, Arc::clone(&entry));
        let _guard = entry.lock().expect("entry lock");
        assert_eq!(with_entry(u64::MAX, |_| Ok(())), Status::Busy);
        REGISTRY
            .lock()
            .expect("registry lock")
            .entries
            .remove(&u64::MAX);
    }

    #[test]
    fn overlap_detection_is_half_open_and_overflow_safe() {
        let bytes = [0_u8; 16];
        let start = bytes.as_ptr();
        assert!(regions_overlap(start, 8, unsafe { start.add(7) }, 2));
        assert!(!regions_overlap(start, 8, unsafe { start.add(8) }, 8));
        assert!(regions_overlap(usize::MAX as *const u8, 2, start, 1));
    }

    #[test]
    fn cancellation_of_a_never_armed_internal_timer_is_an_idempotent_no_op() {
        let mut entry = Entry::new();
        let token = coordinator::TimerToken::Mutation(
            coordinator::MutationTimerToken::LightingRefresh(coordinator::scheduler::Generation(1)),
        );
        assert_eq!(entry.remove_timer(token), Ok(0));
    }

    #[test]
    fn logical_timer_handle_survives_firing_and_is_reused_by_replacement() {
        let mut entry = Entry::new();
        let token = coordinator::TimerToken::Mutation(
            coordinator::MutationTimerToken::LightingRefresh(coordinator::scheduler::Generation(1)),
        );
        let id = entry.allocate_timer(token).expect("timer handle");
        let fired = CommandEnvelope::TimerFired {
            generation: 1,
            token: id,
            now_ms: 1_000,
        };

        entry
            .consume_token(fired.consumed_token())
            .expect("logical timer remains registered");
        assert_eq!(entry.timer(id), Ok(token));
        let replacement = entry.allocate_timer(token).expect("replacement handle");
        assert_eq!(replacement, id);
        assert_eq!(entry.timer(replacement), Ok(token));
        assert_eq!(entry.remove_timer(token), Ok(id));
    }
}
