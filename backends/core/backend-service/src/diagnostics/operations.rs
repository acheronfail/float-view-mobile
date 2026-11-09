//! Payload-free evidence at service and runtime operation boundaries.
use super::*;
use float_view_application_runtime::{RuntimeError, coordinator as c};

pub(crate) fn configuration_error(error: &c::configuration::ServiceError) -> &'static str {
    use c::configuration::ServiceError as E;
    match error {
        E::Unavailable => "Unavailable",
        E::InvalidState => "InvalidState",
        E::StaleResponse => "StaleResponse",
        E::UnauditedSchema => "UnauditedSchema",
        E::MalformedResponse => "MalformedResponse",
        E::Conflict => "Conflict",
        E::Codec(error) => codec_error(error),
    }
}

fn codec_error(error: &c::configuration_codec::ConfigurationError) -> &'static str {
    use c::configuration_codec::ConfigurationError as E;
    match error {
        E::TruncatedEnvelope => "truncated_envelope",
        E::CompressedTooLarge => "compressed_too_large",
        E::ExpandedTooLarge => "expanded_too_large",
        E::ExpandedLengthMismatch => "expanded_length_mismatch",
        E::DecompressionFailed => "decompression_failed",
        E::ProhibitedXml => "prohibited_xml",
        E::MalformedXml => "malformed_xml",
        E::StructuralLimit => "StructuralLimit",
        E::DuplicateField => "duplicate_field",
        E::DuplicateOrder => "duplicate_order",
        E::MissingReference => "missing_reference",
        E::UnsupportedType => "unsupported_type",
        E::UnsupportedEncoding => "unsupported_encoding",
        E::InvalidMetadata => "invalid_metadata",
        E::InvalidNumber => "invalid_number",
        E::SignatureMismatch => "signature_mismatch",
        E::TruncatedValues => "truncated_values",
        E::Remainder => "remainder",
        E::MissingValue => "missing_value",
        E::ExtraValue => "extra_value",
        E::InvalidValue => "invalid_value",
    }
}

fn runtime_error(error: &RuntimeError) -> (&'static str, &'static str) {
    use c::CoordinatorError as E;
    match error {
        RuntimeError::Coordinator(error) => match error {
            E::Configuration(error) => ("configuration", configuration_error(error)),
            E::Mutation(error) => (
                "mutation",
                match error {
                    c::mutation::CoordinatorError::InvalidState => "InvalidState",
                    c::mutation::CoordinatorError::InvalidRoute => "invalid_route",
                    c::mutation::CoordinatorError::UnsupportedController => {
                        "unsupported_controller"
                    }
                    c::mutation::CoordinatorError::InvalidLightingResponse => {
                        "invalid_lighting_response"
                    }
                },
            ),
            E::Scheduler(error) => (
                "scheduler",
                match error {
                    c::scheduler::SchedulerError::InvalidCanId => "invalid_can_id",
                    c::scheduler::SchedulerError::PayloadTooLarge => "payload_too_large",
                    c::scheduler::SchedulerError::InvalidTimeout => "invalid_timeout",
                    c::scheduler::SchedulerError::MonotonicOverflow => "monotonic_overflow",
                },
            ),
            E::Discovery(error) => (
                "discovery",
                match error {
                    c::discovery::DiscoveryInputError::InvalidCanId => "invalid_can_id",
                    c::discovery::DiscoveryInputError::InvalidControllerUuid => {
                        "invalid_controller_identity"
                    }
                    c::discovery::DiscoveryInputError::InvalidSelection => "invalid_selection",
                },
            ),
            E::Telemetry(error) => (
                "telemetry",
                match error {
                    c::telemetry::ServiceError::InvalidIdleInterval => "invalid_idle_interval",
                    c::telemetry::ServiceError::InvalidPostRideTail => "invalid_post_ride_tail",
                    c::telemetry::ServiceError::InvalidRoute => "invalid_route",
                    c::telemetry::ServiceError::MonotonicOverflow => "monotonic_overflow",
                    c::telemetry::ServiceError::InvalidTelemetry => "invalid_telemetry",
                },
            ),
            E::Bms(error) => (
                "bms",
                match error {
                    c::bms::ServiceError::DiscoveryInput(_) => "invalid_discovery_input",
                    c::bms::ServiceError::InvalidRoute => "invalid_route",
                    c::bms::ServiceError::InvalidObservationTime => "invalid_observation_time",
                },
            ),
            E::CustomConfiguration(_) => ("configuration", "MalformedResponse"),
            E::InvalidCanId => ("runtime", "invalid_can_id"),
            E::InvalidObservationTime => ("runtime", "invalid_observation_time"),
            E::InvalidMonotonicTime => ("runtime", "invalid_monotonic_time"),
            E::InvalidPayload => ("runtime", "invalid_payload"),
            E::EffectLimit => ("runtime", "effect_limit"),
        },
        RuntimeError::Framing(_) => ("framing", "MalformedResponse"),
        RuntimeError::InvalidSessionPath => ("storage", "invalid_session_path"),
        RuntimeError::SessionAlreadyAttached => ("storage", "session_already_attached"),
        RuntimeError::SessionNotAttached => ("storage", "session_not_attached"),
        RuntimeError::Persistence(error) => (
            "storage",
            match error {
                float_view_application_runtime::session::WriterError::InvalidInput => {
                    "invalid_input"
                }
                float_view_application_runtime::session::WriterError::TargetExists => {
                    "target_exists"
                }
                float_view_application_runtime::session::WriterError::Incompatible => {
                    "incompatible_storage"
                }
                float_view_application_runtime::session::WriterError::Corrupt => "corrupt_storage",
                float_view_application_runtime::session::WriterError::Busy => "busy",
                float_view_application_runtime::session::WriterError::Full => "storage_full",
                float_view_application_runtime::session::WriterError::Io => "io_failure",
                float_view_application_runtime::session::WriterError::Database => {
                    "database_failure"
                }
            },
        ),
        RuntimeError::EffectLimit => ("runtime", "effect_limit"),
    }
}

pub(crate) fn operation_kind(command: &c::Command) -> Option<&'static str> {
    Some(match command {
        c::Command::WriteConfiguration {
            mutation: c::mutation::ConfigurationMutation::TuneApply,
            ..
        } => "tune_apply",
        c::Command::WriteConfiguration { .. } => "configuration_write",
        c::Command::ReadConfiguration { defaults: true, .. } => "read_default",
        c::Command::ReadConfiguration { .. } => "read_current",
        c::Command::SetControl { control, .. } => match control {
            c::mutation::BooleanControl::Package => "package_control",
            c::mutation::BooleanControl::Flywheel => "flywheel_control",
            c::mutation::BooleanControl::Leds => "leds_control",
            c::mutation::BooleanControl::Headlights => "headlights_control",
        },
        c::Command::ClearFatalError { .. } => "clear_fatal_error",
        c::Command::ShutdownController { .. } => "controller_shutdown",
        _ => return None,
    })
}

impl BackendService {
    fn operation_state_context(
        &self,
        state: &c::Snapshot,
        kind: &'static str,
        mutation: c::mutation::ConfigurationMutation,
    ) -> Value {
        let config = &state.configuration;
        let phase = match state.phase {
            c::Phase::Disconnected => "disconnected",
            c::Phase::Identifying => "identifying",
            c::Phase::SelectingController => "selecting_controller",
            c::Phase::AwaitingActivation => "awaiting_activation",
            c::Phase::Ready => "ready",
            c::Phase::Failed => "failed",
        };
        let block = self
            .runtime
            .configuration_mutation_block_reason(mutation, self.policy.exchange.now)
            .map(|reason| match reason {
                c::ConfigurationWriteBlockReason::Reconnecting => "reconnecting",
                c::ConfigurationWriteBlockReason::Loading => "loading",
                c::ConfigurationWriteBlockReason::Unavailable => "unavailable",
                c::ConfigurationWriteBlockReason::ReadOnly => "read_only",
                c::ConfigurationWriteBlockReason::Busy => "busy",
                c::ConfigurationWriteBlockReason::Flywheel => "flywheel",
                c::ConfigurationWriteBlockReason::ImuTimeoutActive => "imu_timeout_active",
                c::ConfigurationWriteBlockReason::ImuTimeoutUnconfirmed => {
                    "imu_timeout_unconfirmed"
                }
                c::ConfigurationWriteBlockReason::Riding => "riding",
                c::ConfigurationWriteBlockReason::StateUnknown => "state_unknown",
                c::ConfigurationWriteBlockReason::PackageNotReady => "package_not_ready",
                c::ConfigurationWriteBlockReason::ModeNotNormal => "mode_not_normal",
                c::ConfigurationWriteBlockReason::CurrentRequired => "current_required",
            });
        json!({
            "kind":kind,"application_generation":state.generation.0,"state":phase,
            "standalone_bms":state.standalone_bms,"writable":config.writable,
            "current_loaded":config.current.is_some(),"identity_loaded":config.identity.is_some(),
            "configuration_pending":config.pending.map(|p| match p {
                c::configuration::PendingOperation::Schema => "schema",
                c::configuration::PendingOperation::ReadCurrent => "read_current",
                c::configuration::PendingOperation::ReadDefault => "read_default",
                c::configuration::PendingOperation::Write => "configuration_write",
            }),
            "mutation_pending":state.mutations.pending.map(|p| mutation_kind(p.kind)),
            "write_block":block,"current_revision":config.current.as_ref().map(|v| v.revision),
            "availability": match config.availability {
                c::configuration::Availability::Ready => "ready",
                c::configuration::Availability::Loading => "loading",
                c::configuration::Availability::Unavailable => "unavailable",
            },
        })
    }

    /// Capture before execution: a failed transition may already have changed state.
    pub(crate) fn operation_context(&self, command: &c::Command) -> Option<Value> {
        let kind = operation_kind(command)?;
        let state = self.runtime.snapshot().application;
        let config = &state.configuration;
        let mutation = match command {
            c::Command::WriteConfiguration { mutation, .. } => *mutation,
            _ => c::mutation::ConfigurationMutation::Write,
        };
        let mut facts = self.operation_state_context(&state, kind, mutation);
        if let c::Command::SetControl { enabled, .. } = command {
            facts["enabled"] = json!(enabled);
        }
        if matches!(
            command,
            c::Command::SetControl { .. }
                | c::Command::ClearFatalError { .. }
                | c::Command::ShutdownController { .. }
        ) {
            facts["controls_ready"] = json!(
                state
                    .mutations
                    .controls
                    .as_ref()
                    .map(|c| c.status == c::mutation::ControlStatus::Ready)
            );
            facts["controls_writable"] =
                json!(state.mutations.controls.as_ref().map(|c| c.writable));
            facts["package_enabled"] = json!(
                state
                    .mutations
                    .controls
                    .as_ref()
                    .and_then(|c| c.package_enabled)
            );
            facts["flywheel_enabled"] = json!(
                state
                    .mutations
                    .controls
                    .as_ref()
                    .and_then(|c| c.flywheel_enabled)
            );
        }
        if let c::Command::WriteConfiguration {
            generation,
            package_version,
            fingerprint,
            wire_signature,
            base_revision,
            ..
        } = command
        {
            facts["generation_matches"] = json!(*generation == state.generation);
            facts["base_revision"] = json!(base_revision);
            facts["revision_matches"] = json!(
                config
                    .current
                    .as_ref()
                    .map(|v| v.revision == *base_revision)
            );
            facts["package_matches"] = json!(
                config
                    .identity
                    .as_ref()
                    .map(|i| i.package_version == *package_version)
            );
            facts["schema_matches"] = json!(
                config
                    .identity
                    .as_ref()
                    .map(|i| i.fingerprint == fingerprint)
            );
            facts["signature_matches"] = json!(
                config
                    .identity
                    .as_ref()
                    .map(|i| i.wire_signature == *wire_signature)
            );
        }
        Some(facts)
    }

    pub(crate) fn operation_event(
        &mut self,
        mut facts: Value,
        outcome: &'static str,
        code: Option<&'static str>,
    ) {
        facts["outcome"] = json!(outcome);
        if let Some(code) = code {
            facts["code"] = json!(code);
        }
        self.diagnostics.event(
            self.policy.exchange.now,
            self.policy.exchange.epoch,
            "configuration_operation",
            facts,
            true,
        );
        if matches!(outcome, "rejected" | "failed" | "timeout" | "cancelled") {
            self.diagnostics.capture(
                self.policy.exchange.now,
                self.policy.exchange.epoch,
                "configuration_failure",
                false,
            );
        }
    }

    pub(crate) fn timeout_operation(&mut self, id: u64) {
        if let Some(p) = self.pending.get(&id)
            && let Some(kind) = p.diagnostic_kind
        {
            self.operation_event(json!({"kind":kind,"operation":id,"application_generation":p.application_generation}), "timeout", Some("request_timeout"));
        }
        self.finish_pending(id, Err(Failure::Unavailable));
    }

    pub(crate) fn cancel_operation(&mut self, id: u64, reason: &'static str) {
        if let Some(kind) = self.pending.get(&id).and_then(|p| p.diagnostic_kind) {
            self.operation_event(json!({"kind":kind,"operation":id,"application_generation":self.pending[&id].application_generation}), "cancelled", Some(reason));
        }
    }

    pub(crate) fn record_mutation_failure(
        &mut self,
        token: c::mutation::MutationToken,
        reason: c::mutation::FailureReason,
    ) {
        let kind = mutation_kind(token.kind);
        let mutation = if token.kind == c::mutation::PendingKind::TuneApply {
            c::mutation::ConfigurationMutation::TuneApply
        } else {
            c::mutation::ConfigurationMutation::Write
        };
        let mut facts =
            self.operation_state_context(&self.runtime.snapshot().application, kind, mutation);
        if let Some((&id, pending)) = self
            .pending
            .iter()
            .find(|(_, p)| p.diagnostic_kind == Some(kind))
        {
            facts["operation"] = json!(id);
            facts["application_generation"] = json!(pending.application_generation);
        }
        facts["code"] = json!(mutation_failure(reason));
        self.diagnostics.event(
            self.policy.exchange.now,
            self.policy.exchange.epoch,
            "mutation_failure",
            facts,
            true,
        );
        self.diagnostics.capture(
            self.policy.exchange.now,
            self.policy.exchange.epoch,
            "configuration_failure",
            false,
        );
    }

    pub(crate) fn runtime_failure(&mut self, error: &RuntimeError, context: Option<Value>) {
        self.record_configuration_contents(None);
        let (layer, code) = runtime_error(error);
        let mut facts = context.unwrap_or_else(
            || json!({"application_generation":self.runtime.snapshot().application.generation.0}),
        );
        if let Some((&id, _)) = self
            .pending
            .iter()
            .find(|(_, p)| p.diagnostic_kind.is_some())
        {
            facts["operation"] = json!(id);
        }
        facts["layer"] = json!(layer);
        facts["code"] = json!(code);
        facts["outcome"] = json!("rejected");
        self.diagnostics.event(
            self.policy.exchange.now,
            self.policy.exchange.epoch,
            "runtime_failure",
            facts,
            true,
        );
        self.diagnostics.capture(
            self.policy.exchange.now,
            self.policy.exchange.epoch,
            "protocol_failure",
            false,
        );
    }
}

pub(crate) fn mutation_kind(kind: c::mutation::PendingKind) -> &'static str {
    use c::mutation::PendingKind as K;
    match kind {
        K::Package => "package_control",
        K::Flywheel => "flywheel_control",
        K::Leds => "leds_control",
        K::Headlights => "headlights_control",
        K::ClearFatalError => "clear_fatal_error",
        K::ControllerShutdown => "controller_shutdown",
        K::ConfigurationWrite => "configuration_write",
        K::TuneApply => "tune_apply",
    }
}

pub(crate) fn mutation_failure(reason: c::mutation::FailureReason) -> &'static str {
    use c::mutation::FailureReason as R;
    match reason {
        R::RequestFailed => "request_failed",
        R::ConfirmationTimedOut => "confirmation_timed_out",
        R::ProhibitedByFlywheel => "prohibited_by_flywheel",
        R::Cancelled => "cancelled",
        R::Replaced => "replaced",
    }
}
