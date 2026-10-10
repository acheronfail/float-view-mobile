//! Safe coarse application composition for future native host cutovers.
//!
//! The runtime owns portable VESC stream, connection/application, and one
//! canonical session-writer lifetime. Hosts still own OS services and execute
//! the ordered effects returned here. No callback or host reference crosses
//! this boundary.

#![forbid(unsafe_code)]

use std::fmt::{self, Debug, Formatter};
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

pub use float_view_connection_application_coordinator as coordinator;
pub use float_view_refloat_telemetry_service as telemetry;
pub use float_view_session_history_kernel as history;
pub use float_view_session_sqlite as session;

use coordinator::{ConnectionApplicationCoordinator, Effect as CoordinatorEffect};
use float_view_vesc_frame::{FrameError, MAX_STREAM_BYTES, StreamDecoder, encode};
use session::{CommitMarker, LocationObservation, SessionMetadata, SessionWriter, WriterError};

pub const MAX_RUNTIME_EFFECTS: usize = coordinator::MAX_TRANSITION_EFFECTS * 2;
pub const MAX_SESSION_PATH_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionMode {
    Create,
    Open,
}

pub enum Command {
    Application(coordinator::Command),
    ReceiveBytes {
        generation: coordinator::Generation,
        bytes: Vec<u8>,
        now_ms: u64,
        observed_at_epoch_ms: i64,
    },
    AttachSession {
        path: PathBuf,
        mode: SessionMode,
        metadata: SessionMetadata,
    },
    AppendLocations(Vec<LocationObservation>),
    ProfileChanged {
        saved_device_id: String,
        revision: u64,
        values_json: String,
        at_ms: i64,
    },
    DetachSession,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    WriteBytes(Vec<u8>),
    Application(CoordinatorEffect),
    PublishSessionMarker(CommitMarker),
    Failure(RuntimeFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeFailure {
    Persistence(WriterError),
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeError {
    Coordinator(coordinator::CoordinatorError),
    Framing(FrameError),
    InvalidSessionPath,
    SessionAlreadyAttached,
    SessionNotAttached,
    Persistence(WriterError),
    EffectLimit,
}

impl From<coordinator::CoordinatorError> for RuntimeError {
    fn from(value: coordinator::CoordinatorError) -> Self {
        Self::Coordinator(value)
    }
}

impl From<FrameError> for RuntimeError {
    fn from(value: FrameError) -> Self {
        Self::Framing(value)
    }
}

impl From<WriterError> for RuntimeError {
    fn from(value: WriterError) -> Self {
        Self::Persistence(value)
    }
}

#[derive(Clone, PartialEq)]
pub struct Snapshot {
    pub application: coordinator::Snapshot,
    pub session_attached: bool,
    pub session_marker: Option<CommitMarker>,
}

impl Debug for Snapshot {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Snapshot")
            .field("generation", &self.application.generation)
            .field("phase", &self.application.phase)
            .field("session_attached", &self.session_attached)
            .field("has_session_marker", &self.session_marker.is_some())
            .finish()
    }
}

pub struct ApplicationRuntime {
    application: ConnectionApplicationCoordinator,
    stream: StreamDecoder,
    session: Option<SessionWriter>,
    session_marker: Option<CommitMarker>,
}

impl Default for ApplicationRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ApplicationRuntime {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApplicationRuntime")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

impl ApplicationRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self {
            application: ConnectionApplicationCoordinator::default(),
            stream: StreamDecoder::new(),
            session: None,
            session_marker: None,
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            application: self.application.snapshot(),
            session_attached: self.session.is_some(),
            session_marker: self.session_marker.clone(),
        }
    }

    #[must_use]
    pub fn configuration_schema(
        &self,
    ) -> Option<&coordinator::configuration_codec::ConfigurationSchema> {
        self.application.configuration_schema()
    }

    pub fn apply(&mut self, command: Command) -> Result<Vec<Effect>, RuntimeError> {
        match command {
            Command::Application(command) => {
                if matches!(
                    command,
                    coordinator::Command::Begin(_) | coordinator::Command::Reset { .. }
                ) {
                    self.stream = StreamDecoder::new();
                }
                let effects = self.application.apply(command)?;
                self.translate(effects)
            }
            Command::ReceiveBytes {
                generation,
                bytes,
                now_ms,
                observed_at_epoch_ms,
            } => {
                if bytes.len() > MAX_STREAM_BYTES {
                    return Err(RuntimeError::Framing(FrameError::PayloadTooLarge));
                }
                let payloads = self.stream.append(&bytes)?;
                let mut output = Vec::new();
                for payload in payloads {
                    let effects = self.application.apply(coordinator::Command::Receive {
                        generation,
                        payload,
                        now_ms,
                        observed_at_epoch_ms,
                    })?;
                    append_bounded(&mut output, self.translate(effects)?)?;
                }
                Ok(output)
            }
            Command::AttachSession {
                path,
                mode,
                metadata,
            } => self.attach_session(&path, mode, metadata),
            Command::AppendLocations(locations) => self.append_locations(&locations),
            Command::ProfileChanged {
                saved_device_id,
                revision,
                values_json,
                at_ms,
            } => {
                let writer = self
                    .session
                    .as_mut()
                    .ok_or(RuntimeError::SessionNotAttached)?;
                let marker = writer.append_profile_change(
                    &saved_device_id,
                    revision,
                    &values_json,
                    at_ms,
                )?;
                self.session_marker = Some(marker.clone());
                Ok(vec![Effect::PublishSessionMarker(marker)])
            }
            Command::DetachSession => self.detach_session(),
        }
    }

    pub fn save_ride_marker(
        &mut self,
        operation_id: &str,
        elapsed_ms: u64,
        at_ms: i64,
        capture_location: bool,
    ) -> Result<(session::RideMarker, CommitMarker), RuntimeError> {
        let (marker, boundary) = self
            .session
            .as_mut()
            .ok_or(RuntimeError::SessionNotAttached)?
            .append_ride_marker(operation_id, elapsed_ms, at_ms, capture_location)?;
        self.session_marker = Some(boundary.clone());
        Ok((marker, boundary))
    }

    pub fn open_reader(
        path: impl AsRef<Path>,
    ) -> Result<session::SessionReader, session::ReaderError> {
        session::SessionReader::open(path)
    }

    pub fn open_pruner(
        path: impl AsRef<Path>,
    ) -> Result<session::SessionPruner, session::PruneError> {
        session::SessionPruner::open(path)
    }

    fn attach_session(
        &mut self,
        path: &Path,
        mode: SessionMode,
        metadata: SessionMetadata,
    ) -> Result<Vec<Effect>, RuntimeError> {
        if self.session.is_some() {
            return Err(RuntimeError::SessionAlreadyAttached);
        }
        validate_path(path)?;
        let writer = match mode {
            SessionMode::Create => SessionWriter::create(path, metadata),
            SessionMode::Open => SessionWriter::open(path),
        }?;
        let marker = writer.marker()?;
        self.session = Some(writer);
        self.session_marker = Some(marker.clone());
        Ok(vec![Effect::PublishSessionMarker(marker)])
    }

    fn append_locations(
        &mut self,
        locations: &[LocationObservation],
    ) -> Result<Vec<Effect>, RuntimeError> {
        let Some(writer) = self.session.as_mut() else {
            return Err(RuntimeError::SessionNotAttached);
        };
        match writer.append_locations(locations) {
            Ok(marker) => {
                self.session_marker = Some(marker.clone());
                Ok(vec![Effect::PublishSessionMarker(marker)])
            }
            Err(error) => Ok(vec![Effect::Failure(RuntimeFailure::Persistence(error))]),
        }
    }

    fn detach_session(&mut self) -> Result<Vec<Effect>, RuntimeError> {
        let Some(writer) = self.session.take() else {
            return Err(RuntimeError::SessionNotAttached);
        };
        writer.checkpoint()?;
        writer.close()?;
        self.session_marker = None;
        Ok(Vec::new())
    }

    fn translate(&mut self, effects: Vec<CoordinatorEffect>) -> Result<Vec<Effect>, RuntimeError> {
        let mut output = Vec::with_capacity(effects.len());
        for effect in effects {
            match effect {
                CoordinatorEffect::Write(request) => {
                    push_bounded(&mut output, Effect::WriteBytes(encode(&request.payload)?))?;
                }
                CoordinatorEffect::PublishTelemetry(sample) => {
                    if let Some(writer) = self.session.as_mut() {
                        match writer.append_samples(std::slice::from_ref(sample.as_ref())) {
                            Ok(marker) => {
                                self.session_marker = Some(marker.clone());
                                push_bounded(&mut output, Effect::PublishSessionMarker(marker))?;
                            }
                            Err(error) => {
                                return Ok(vec![Effect::Failure(RuntimeFailure::Persistence(
                                    error,
                                ))]);
                            }
                        }
                    }
                    push_bounded(
                        &mut output,
                        Effect::Application(CoordinatorEffect::PublishTelemetry(sample)),
                    )?;
                }
                other => push_bounded(&mut output, Effect::Application(other))?,
            }
        }
        Ok(output)
    }
}

fn validate_path(path: &Path) -> Result<(), RuntimeError> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_SESSION_PATH_BYTES
        || bytes.contains(&0)
        || !path.is_absolute()
        || path.components().any(|part| part == Component::ParentDir)
    {
        Err(RuntimeError::InvalidSessionPath)
    } else {
        Ok(())
    }
}

fn push_bounded(effects: &mut Vec<Effect>, effect: Effect) -> Result<(), RuntimeError> {
    if effects.len() >= MAX_RUNTIME_EFFECTS {
        return Err(RuntimeError::EffectLimit);
    }
    effects.push(effect);
    Ok(())
}

fn append_bounded(target: &mut Vec<Effect>, mut source: Vec<Effect>) -> Result<(), RuntimeError> {
    if target.len().saturating_add(source.len()) > MAX_RUNTIME_EFFECTS {
        return Err(RuntimeError::EffectLimit);
    }
    target.append(&mut source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use coordinator::discovery::Continuity;
    use telemetry::{
        AlertReason, FootpadState, NormalizedSample, PackageMode, PackageState, RideState,
        SetpointAdjustment, Settings, StopCondition,
    };

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn begin(now_ms: u64) -> coordinator::Command {
        coordinator::Command::Begin(coordinator::BeginFacts {
            now_ms,
            settings: Settings::new(1_000, 5_000).expect("settings"),
            preferred_controller_can_id: None,
            routing_hint: None,
            continuity: Continuity::default(),
            preferred_bms_can_id: None,
        })
    }

    fn sample(sequence: u64) -> NormalizedSample {
        NormalizedSample {
            sequence,
            observed_at_epoch_ms: 1_700_000_000_000 + sequence as i64,
            slow_observed_at_epoch_ms: None,
            elapsed_ms: sequence * 100,
            ride_state: RideState::Idle,
            package_state: Some(PackageState::Ready),
            package_mode: Some(PackageMode::Normal),
            footpad_state: Some(FootpadState::None),
            stop_condition: Some(StopCondition::None),
            setpoint_adjustment: Some(SetpointAdjustment::None),
            wheelslip: Some(false),
            darkride: Some(false),
            charging: Some(false),
            alert_reason: Some(AlertReason::None),
            firmware_fault: None,
            fatal_error_latched: Some(false),
            pack_voltage_v: Some(80.0),
            duty_cycle: Some(0.0),
            controller_temp_c: Some(20.0),
            motor_temp_c: Some(20.0),
            motor_current_a: Some(0.0),
            battery_current_a: Some(0.0),
            erpm: Some(0.0),
            speed_kph: Some(0.0),
            pitch_deg: Some(0.0),
            balance_pitch_deg: Some(0.0),
            roll_deg: Some(0.0),
            directional_motor_current_a: Some(0.0),
            filtered_motor_current_a: Some(0.0),
            requested_balance_current_a: Some(0.0),
            booster_current_a: Some(0.0),
            field_weakening_current_a: Some(0.0),
            footpad_adc1_v: Some(0.0),
            footpad_adc2_v: Some(0.0),
            remote_input: Some(0.0),
            setpoint_deg: Some(0.0),
            atr_setpoint_deg: Some(0.0),
            brake_tilt_setpoint_deg: Some(0.0),
            torque_tilt_setpoint_deg: Some(0.0),
            turn_tilt_setpoint_deg: Some(0.0),
            remote_setpoint_deg: Some(0.0),
            atr_acceleration_difference_erpm_per_second: Some(0.0),
            atr_speed_boost: Some(0.0),
            distance_m: Some(0.0),
            odometer_m: Some(0.0),
            amp_hours_drawn_ah: Some(0.0),
            amp_hours_charged_ah: Some(0.0),
            watt_hours_drawn_wh: Some(0.0),
            watt_hours_charged_wh: Some(0.0),
            battery_level: Some(1.0),
            charging_current_a: Some(0.0),
            charging_voltage_v: Some(0.0),
            extensions: Vec::new(),
        }
    }

    fn session_metadata() -> SessionMetadata {
        SessionMetadata {
            expected_riding_persistence_ms: 100,
            expected_idle_persistence_ms: 1_000,
            committed_at_epoch_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn retained_ride_reconnects_preserve_samples_clock_and_marker_alignment() {
        let directory = tempfile_directory("reconnect-clock");
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::AttachSession {
                path: directory.join("ride.sqlite3"),
                mode: SessionMode::Create,
                metadata: session_metadata(),
            })
            .unwrap();
        for (index, base) in [0, 170_000, 200_000].into_iter().enumerate() {
            let coordinator::Command::Begin(mut facts) = begin(base + 10) else {
                unreachable!()
            };
            facts.continuity = Continuity::new(index > 0, None, None);
            runtime
                .apply(Command::Application(coordinator::Command::Begin(facts)))
                .unwrap();
            let generation = runtime.snapshot().application.generation;
            // Legacy fixtures exercise the same session ownership as selectable telemetry.
            for payload in [firmware_response(), vec![36, 101, 0, 10]] {
                runtime
                    .apply(Command::Application(coordinator::Command::Receive {
                        generation,
                        payload,
                        now_ms: base + 20,
                        observed_at_epoch_ms: 1_700_000_000_000 + base as i64 + 20,
                    }))
                    .unwrap();
            }
            runtime
                .apply(Command::Application(coordinator::Command::Activate {
                    generation,
                    now_ms: base + 40,
                    session_started: index == 0,
                    cached_configuration_schema: None,
                }))
                .unwrap();
            let mut limits = vec![14];
            limits.extend_from_slice(&0x2EFD_0142_u32.to_be_bytes());
            limits.extend_from_slice(&[0; 4]);
            for limit in [100_f32, -100., 50., -20.] {
                limits.extend_from_slice(&limit.to_be_bytes());
            }
            runtime
                .apply(Command::Application(coordinator::Command::Receive {
                    generation,
                    payload: limits,
                    now_ms: base + 45,
                    observed_at_epoch_ms: 1_700_000_000_000 + base as i64 + 45,
                }))
                .unwrap();
            let fixture = include_str!("../../../fixtures/refloat/protocol-1.0.0.json");
            let hex = fixture
                .split_once("legacy-riding-all-data")
                .unwrap()
                .1
                .split_once("\"frameHex\": \"")
                .unwrap()
                .1
                .split('"')
                .next()
                .unwrap();
            let frame = hex
                .as_bytes()
                .chunks_exact(2)
                .map(|bytes| u8::from_str_radix(std::str::from_utf8(bytes).unwrap(), 16).unwrap())
                .collect();
            let effects = runtime
                .apply(Command::ReceiveBytes {
                    generation,
                    bytes: frame,
                    now_ms: base + 70,
                    observed_at_epoch_ms: 1_700_000_000_000 + base as i64 + 70,
                })
                .unwrap();
            let sample = effects
                .iter()
                .find_map(|effect| match effect {
                    Effect::Application(CoordinatorEffect::PublishTelemetry(sample)) => {
                        Some(sample)
                    }
                    _ => None,
                })
                .expect("published telemetry");
            assert_eq!(sample.sequence, index as u64 + 1);
            assert_eq!(sample.elapsed_ms, base + 30);
            let boundary = runtime.snapshot().session_marker.unwrap();
            assert_eq!(boundary.stored_sample_count, index as u64 + 1);
            assert_eq!(boundary.recorded_sample_count, boundary.stored_sample_count);
            let (marker, _) = runtime
                .save_ride_marker(
                    &format!("00000000-0000-4000-8000-{index:012x}"),
                    base + 35,
                    1_700_000_000_000 + base as i64 + 75,
                    false,
                )
                .unwrap();
            assert_eq!(marker.elapsed_ms, sample.elapsed_ms + 5);
        }
        drop(runtime);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn begins_with_one_framed_firmware_request() {
        let mut runtime = ApplicationRuntime::new();
        let effects = runtime
            .apply(Command::Application(begin(10)))
            .expect("begin");
        let frame = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::WriteBytes(bytes) => Some(bytes),
                _ => None,
            })
            .expect("write bytes");
        assert_eq!(float_view_vesc_frame::decode(frame), Ok(vec![0]));
        assert!(!format!("{runtime:?}").contains("payload"));
    }

    #[test]
    fn fragmented_response_is_retained_and_processed_once() {
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::Application(begin(10)))
            .expect("begin");
        let generation = runtime.snapshot().application.generation;
        let mut payload = vec![0, 6, 6];
        payload.extend_from_slice(b"THOR300\0");
        payload.extend_from_slice(&[7; 12]);
        payload.extend_from_slice(&[1, 0, 0, 0, 1, 0, 1, 0]);
        payload.extend_from_slice(b"Float\0");
        let frame = float_view_vesc_frame::encode(&payload).expect("frame");
        let split = frame.len() / 2;
        assert!(
            runtime
                .apply(Command::ReceiveBytes {
                    generation,
                    bytes: frame[..split].to_vec(),
                    now_ms: 20,
                    observed_at_epoch_ms: 1_700_000_000_020,
                })
                .expect("fragment")
                .is_empty()
        );
        let effects = runtime
            .apply(Command::ReceiveBytes {
                generation,
                bytes: frame[split..].to_vec(),
                now_ms: 21,
                observed_at_epoch_ms: 1_700_000_000_021,
            })
            .expect("complete");
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::WriteBytes(_)))
        );
    }

    #[test]
    fn noise_and_multiple_frames_are_consumed_in_order() {
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::Application(begin(10)))
            .expect("begin");
        let generation = runtime.snapshot().application.generation;
        let mut bytes = vec![0xff, 0x01];
        bytes.extend(float_view_vesc_frame::encode(&firmware_response()).expect("firmware frame"));
        bytes.extend(float_view_vesc_frame::encode(&[36, 101, 0, 10]).expect("legacy info frame"));
        let effects = runtime
            .apply(Command::ReceiveBytes {
                generation,
                bytes,
                now_ms: 20,
                observed_at_epoch_ms: 1_700_000_000_020,
            })
            .expect("two responses");
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::WriteBytes(_)))
        );
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Application(CoordinatorEffect::PrepareActivation(_))
        )));
    }

    #[test]
    fn malformed_frame_is_consumed_and_the_stream_recovers() {
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::Application(begin(10)))
            .expect("begin");
        let generation = runtime.snapshot().application.generation;
        let mut corrupt = float_view_vesc_frame::encode(&firmware_response()).expect("frame");
        corrupt[4] ^= 0xff;
        assert_eq!(
            runtime
                .apply(Command::ReceiveBytes {
                    generation,
                    bytes: corrupt,
                    now_ms: 20,
                    observed_at_epoch_ms: 1_700_000_000_020,
                })
                .expect_err("bad CRC"),
            RuntimeError::Framing(FrameError::InvalidCrc)
        );
        let effects = runtime
            .apply(Command::ReceiveBytes {
                generation,
                bytes: float_view_vesc_frame::encode(&firmware_response()).expect("frame"),
                now_ms: 21,
                observed_at_epoch_ms: 1_700_000_000_021,
            })
            .expect("recovered");
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::WriteBytes(_)))
        );
    }

    #[test]
    fn stale_generation_is_diagnostic_only() {
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::Application(begin(10)))
            .expect("begin");
        let current = runtime.snapshot().application.generation;
        let stale = coordinator::Generation(current.0.wrapping_sub(1));
        let effects = runtime
            .apply(Command::Application(
                coordinator::Command::SelectController {
                    generation: stale,
                    can_id: 1,
                    now_ms: 20,
                },
            ))
            .expect("stale event");
        assert!(matches!(
            effects.as_slice(),
            [Effect::Application(CoordinatorEffect::Diagnostic(
                coordinator::Diagnostic::StaleEvent
            ))]
        ));
        assert_eq!(runtime.snapshot().application.generation, current);
    }

    #[test]
    fn session_commit_precedes_telemetry_publication_and_reopens() {
        let directory = tempfile_directory("commit-order");
        let path = directory.join("session.sqlite");
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::AttachSession {
                path: path.clone(),
                mode: SessionMode::Create,
                metadata: session_metadata(),
            })
            .expect("attach");
        let effects = runtime
            .translate(vec![CoordinatorEffect::PublishTelemetry(Box::new(sample(
                1,
            )))])
            .expect("translate");
        assert!(
            matches!(effects.as_slice(), [Effect::PublishSessionMarker(marker), Effect::Application(CoordinatorEffect::PublishTelemetry(_))] if marker.committed_sample_sequence == 1)
        );
        runtime.apply(Command::DetachSession).expect("detach");
        let mut reader = ApplicationRuntime::open_reader(&path).expect("reader");
        assert_eq!(
            reader.marker().expect("marker").committed_sample_sequence,
            1
        );
        reader.close().expect("close reader");
    }

    #[test]
    fn rejected_persistence_never_reaches_service_estimator() {
        let directory = tempfile_directory("battery-publication-gate");
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::AttachSession {
                path: directory.join("session.sqlite"),
                mode: SessionMode::Create,
                metadata: session_metadata(),
            })
            .unwrap();
        let mut invalid = sample(1);
        invalid.pack_voltage_v = Some(f64::NAN);
        let effects = runtime
            .translate(vec![CoordinatorEffect::PublishTelemetry(Box::new(invalid))])
            .unwrap();
        assert!(effects.iter().any(|e| matches!(e, Effect::Failure(_))));
        assert!(!effects.iter().any(|e| matches!(
            e,
            Effect::Application(CoordinatorEffect::PublishTelemetry(_))
        )));
        assert_eq!(
            runtime
                .snapshot()
                .session_marker
                .unwrap()
                .committed_sample_sequence,
            0
        );
    }

    #[test]
    fn profile_events_allocate_session_sequences_without_overwriting_telemetry() {
        let directory = tempfile_directory("profile-events");
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::AttachSession {
                path: directory.join("session.sqlite"),
                mode: SessionMode::Create,
                metadata: session_metadata(),
            })
            .unwrap();
        runtime
            .translate(vec![CoordinatorEffect::PublishTelemetry(Box::new(sample(
                1,
            )))])
            .unwrap();
        let before = runtime.snapshot().session_marker.unwrap();
        for revision in [1, 2] {
            runtime
                .apply(Command::ProfileChanged {
                    saved_device_id: "board".into(),
                    revision,
                    values_json: "{}".into(),
                    at_ms: 1_700_000_000_002,
                })
                .unwrap();
        }
        let after = runtime.snapshot().session_marker.unwrap();
        assert_eq!(
            after.committed_event_count,
            before.committed_event_count + 2
        );
        assert_eq!(
            after.committed_event_sequence,
            before.committed_event_sequence + 2
        );
        assert_eq!(
            after.committed_sample_sequence,
            before.committed_sample_sequence
        );
        runtime.apply(Command::DetachSession).unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn live_telemetry_without_session_has_no_marker() {
        let mut runtime = ApplicationRuntime::new();
        let effects = runtime
            .translate(vec![CoordinatorEffect::PublishTelemetry(Box::new(sample(
                1,
            )))])
            .expect("translate");
        assert!(matches!(
            effects.as_slice(),
            [Effect::Application(CoordinatorEffect::PublishTelemetry(_))]
        ));
    }

    #[test]
    fn locations_commit_through_the_attached_writer() {
        let directory = tempfile_directory("locations");
        let path = directory.join("session.sqlite");
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::AttachSession {
                path,
                mode: SessionMode::Create,
                metadata: session_metadata(),
            })
            .expect("attach");
        let effects = runtime
            .apply(Command::AppendLocations(vec![LocationObservation {
                observed_at_epoch_ms: 1_700_000_000_100,
                elapsed_ms: 100,
                latitude: -33.86,
                longitude: 151.20,
                accuracy_m: Some(5.0),
            }]))
            .expect("location");
        assert!(
            matches!(effects.as_slice(), [Effect::PublishSessionMarker(marker)] if marker.committed_location_sequence == 1)
        );
    }

    #[test]
    fn invalid_location_fails_atomically_and_the_writer_recovers() {
        let directory = tempfile_directory("location-recovery");
        let path = directory.join("session.sqlite");
        let mut runtime = ApplicationRuntime::new();
        runtime
            .apply(Command::AttachSession {
                path: path.clone(),
                mode: SessionMode::Create,
                metadata: session_metadata(),
            })
            .expect("attach");
        let failure = runtime
            .apply(Command::AppendLocations(vec![LocationObservation {
                observed_at_epoch_ms: 1_700_000_000_100,
                elapsed_ms: 100,
                latitude: f64::NAN,
                longitude: 151.20,
                accuracy_m: Some(5.0),
            }]))
            .expect("contained failure");
        assert_eq!(
            failure,
            [Effect::Failure(RuntimeFailure::Persistence(
                WriterError::InvalidInput
            ))]
        );
        runtime
            .apply(Command::AppendLocations(vec![LocationObservation {
                observed_at_epoch_ms: 1_700_000_000_101,
                elapsed_ms: 101,
                latitude: -33.86,
                longitude: 151.20,
                accuracy_m: Some(5.0),
            }]))
            .expect("recovered location");
        runtime.apply(Command::DetachSession).expect("detach");
        let mut reader = ApplicationRuntime::open_reader(&path).expect("reader");
        assert_eq!(reader.marker().expect("marker").recorded_location_count, 1);
        reader.close().expect("close reader");
        let mut pruner = ApplicationRuntime::open_pruner(&path).expect("pruner");
        assert!(pruner.preview(1).expect("preview").preview().skipped);
        pruner.close().expect("close pruner");
    }

    #[test]
    fn session_lifecycle_rejects_duplicates_and_missing_attachments() {
        let directory = tempfile_directory("session-lifecycle");
        let path = directory.join("session.sqlite");
        let mut runtime = ApplicationRuntime::new();
        assert_eq!(
            runtime
                .apply(Command::DetachSession)
                .expect_err("no session"),
            RuntimeError::SessionNotAttached
        );
        runtime
            .apply(Command::AttachSession {
                path: path.clone(),
                mode: SessionMode::Create,
                metadata: session_metadata(),
            })
            .expect("attach");
        assert_eq!(
            runtime
                .apply(Command::AttachSession {
                    path,
                    mode: SessionMode::Open,
                    metadata: session_metadata(),
                })
                .expect_err("duplicate"),
            RuntimeError::SessionAlreadyAttached
        );
    }

    #[test]
    fn oversized_stream_fails_closed_and_begin_resets_it() {
        let mut runtime = ApplicationRuntime::new();
        let error = runtime
            .apply(Command::ReceiveBytes {
                generation: coordinator::Generation(0),
                bytes: vec![0; MAX_STREAM_BYTES + 1],
                now_ms: 1,
                observed_at_epoch_ms: 1,
            })
            .expect_err("oversized");
        assert_eq!(error, RuntimeError::Framing(FrameError::PayloadTooLarge));
        assert!(runtime.apply(Command::Application(begin(2))).is_ok());
    }

    fn firmware_response() -> Vec<u8> {
        let mut payload = vec![0, 6, 6];
        payload.extend_from_slice(b"THOR300\0");
        payload.extend_from_slice(&[7; 12]);
        payload.extend_from_slice(&[1, 0, 0, 0, 1, 0, 1, 0]);
        payload.extend_from_slice(b"Float\0");
        payload
    }

    fn tempfile_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "float-view-application-runtime-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("create temp directory");
        path
    }
}
