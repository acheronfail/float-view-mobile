mod support;

use float_view_connection_application_coordinator::{
    Command, ConnectionApplicationCoordinator, Effect, Failure, MutationEvent, MutationTimerToken,
    Phase, TimerToken,
};
use float_view_device_mutation_coordinator::{
    BooleanControl, ConfigurationMutation, ControlStatus, PendingKind,
};
use float_view_refloat_configuration_codec::decode_values;
use float_view_refloat_configuration_service::CacheEffect;
use float_view_refloat_configuration_service::{Availability, PendingOperation, ValueSource};

use support::*;

const VECTORS: &str =
    include_str!("../../../fixtures/connection/device-configuration-composition.tsv");

#[test]
fn refloat_1_3_0_skips_layout_and_starts_selectable_float32_telemetry() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_configuration_payload(42), 20);
    let ready = receive(&mut coordinator, selectable_info_payload(), 30);
    assert!(ready.iter().all(|effect| match effect {
        Effect::Write(request) => request.payload != [36, 101, 32],
        _ => true,
    }));
    let activation = coordinator
        .snapshot()
        .awaiting_activation
        .expect("1.3 activation");
    assert_eq!(activation.refloat_version.to_string(), "1.3.0");

    let effects = activate_with_cache(&mut coordinator, 40, Some(configuration_schema_1_3_0()));
    assert_eq!(write(&effects).payload, [93, 0]);
    let effects = receive(&mut coordinator, configuration_current_response_1_3_0(), 50);
    assert_eq!(write(&effects).payload, [14]);
    let effects = receive(
        &mut coordinator,
        current_limits_payload(0x2EFD_0142, Some(15)),
        60,
    );
    assert_eq!(
        write(&effects).payload,
        [36, 101, 33, 1, 0x7f, 0xff, 0xbf, 0xc3, 0, 0, 0, 0]
    );
    let effects = receive(&mut coordinator, selectable_fast_payload(), 70);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::PublishTelemetry(_)))
    );
}

#[test]
fn late_fast_reply_does_not_consume_pending_distance_request() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_configuration_payload(42), 20);
    receive(&mut coordinator, selectable_info_payload(), 30);
    activate_with_cache(&mut coordinator, 40, Some(configuration_schema_1_3_0()));
    receive(&mut coordinator, configuration_current_response_1_3_0(), 50);
    receive(
        &mut coordinator,
        current_limits_payload(0x2EFD_0142, Some(15)),
        60,
    );
    receive(&mut coordinator, selectable_fast_payload(), 70);
    let generation = coordinator.snapshot().generation;
    coordinator
        .apply(Command::Refresh {
            generation,
            now_ms: 1_100,
        })
        .unwrap();
    let effects = receive(&mut coordinator, selectable_fast_payload(), 1_110);
    let slow = write(&effects).clone();
    assert_eq!(&slow.payload[4..8], &16384_u32.to_be_bytes());
    let effects = receive(&mut coordinator, selectable_fast_payload(), 1_111);
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, Effect::Diagnostic(_)))
    );

    let mut payload = selectable_fixture("1.3.0-command-33-slow-gnss-float32");
    assert_eq!(slow.payload, payload[..12]);
    payload[24..28].copy_from_slice(&42.5_f32.to_be_bytes());
    let late_slow = payload.clone();
    receive(&mut coordinator, payload, 1_112);
    receive(
        &mut coordinator,
        selectable_fixture("1.3.0-command-10-mode-0-normal"),
        1_113,
    );
    receive(&mut coordinator, vec![62], 1_114);
    coordinator
        .apply(Command::Refresh {
            generation,
            now_ms: 1_200,
        })
        .unwrap();
    let effects = receive(&mut coordinator, late_slow, 1_201);
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, Effect::Diagnostic(_)))
    );
    let effects = receive(&mut coordinator, selectable_fast_payload(), 1_210);
    let sample = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::PublishTelemetry(sample) => Some(sample),
            _ => None,
        })
        .expect("fast sample with accepted slow telemetry");
    assert_eq!(sample.distance_m, Some(42.5));
    assert_eq!(sample.odometer_m, Some(12345.0));
    assert!(sample.slow_observed_at_epoch_ms.is_some());
}

fn ready_configuration() -> (ConnectionApplicationCoordinator, TimerToken) {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_configuration_payload(42), 20);
    let layout = receive(&mut coordinator, dynamic_info_payload(), 30);
    assert_eq!(write(&layout).payload, [36, 101, 32]);
    receive(&mut coordinator, dynamic_layout_payload(), 40);
    let awaiting = coordinator.snapshot();
    assert_eq!(awaiting.phase, Phase::AwaitingActivation);
    let activation = awaiting.awaiting_activation.expect("activation facts");
    assert_eq!(activation.refloat_version.to_string(), "1.2.3");
    assert!(activation.configuration_cache_identity.is_some());

    let effects = activate_with_cache(&mut coordinator, 50, Some(configuration_schema_1_2_3()));
    assert_eq!(write(&effects).payload, [93, 0]);
    let lighting_timer = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::ReplaceTimer {
                token: token @ TimerToken::Mutation(MutationTimerToken::LightingRefresh(_)),
                ..
            } => Some(*token),
            _ => None,
        })
        .expect("lighting refresh timer");
    assert_eq!(
        coordinator.snapshot().configuration.availability,
        Availability::Ready
    );
    assert_eq!(
        coordinator.snapshot().configuration.pending,
        Some(PendingOperation::ReadCurrent)
    );

    let telemetry_request = receive(&mut coordinator, configuration_current_response_1_2_3(), 60);
    assert_eq!(write(&telemetry_request).payload, [14]);
    let telemetry_request = receive(
        &mut coordinator,
        current_limits_payload(0x2EFD_0142, Some(15)),
        65,
    );
    assert_eq!(write(&telemetry_request).payload, [36, 101, 31]);
    let bms_request = receive(&mut coordinator, dynamic_idle_payload(), 70);
    assert_eq!(write(&bms_request).payload, [62]);
    receive(&mut coordinator, vec![62], 80);

    let lighting = coordinator
        .apply(Command::TimerFired {
            generation: coordinator.snapshot().generation,
            token: lighting_timer,
            now_ms: 1_050,
        })
        .unwrap();
    assert_eq!(write(&lighting).payload, [36, 101, 20, 0, 0, 0, 0, 0]);
    receive(&mut coordinator, vec![36, 101, 20, 3], 1_060);
    assert_eq!(
        coordinator
            .snapshot()
            .mutations
            .controls
            .expect("controls")
            .status,
        ControlStatus::Ready
    );
    (coordinator, lighting_timer)
}

#[test]
fn source_derived_composition_vectors_are_all_covered() {
    let rows = VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 12);
    for row in rows {
        let columns = row.split('|').collect::<Vec<_>>();
        assert_eq!(columns.len(), 3, "invalid vector: {row}");
        assert!(!columns[0].is_empty());
        assert!(!columns[1].is_empty());
        assert!(matches!(
            columns[2],
            "apple-android-application-service"
                | "refloat-configuration-native-services"
                | "refloat-mutation-native-coordinators"
                | "application-service-shutdown-intent"
                | "application-service-tune-apply"
                | "application-service-scheduler-completion"
        ));
    }
}

#[test]
fn cached_activation_default_read_and_control_confirmation_share_one_scheduler() {
    let (mut coordinator, _) = ready_configuration();
    let snapshot = coordinator.snapshot();
    assert_eq!(
        snapshot
            .configuration
            .current
            .as_ref()
            .expect("current")
            .source,
        ValueSource::Current
    );
    assert!(snapshot.configuration.writable);

    let defaults = coordinator
        .apply(Command::ReadConfiguration {
            generation: snapshot.generation,
            defaults: true,
            now_ms: 1_100,
        })
        .unwrap();
    assert_eq!(write(&defaults).payload, [94, 0]);
    receive(
        &mut coordinator,
        configuration_default_response_1_2_3(),
        1_110,
    );
    assert_eq!(
        coordinator
            .snapshot()
            .configuration
            .defaults
            .expect("defaults")
            .source,
        ValueSource::Default
    );

    let control = coordinator
        .apply(Command::SetControl {
            generation: coordinator.snapshot().generation,
            control: BooleanControl::Leds,
            enabled: false,
            now_ms: 1_200,
        })
        .unwrap();
    assert_eq!(write(&control).payload, [36, 101, 20, 0, 0, 0, 1, 2]);
    let token = coordinator
        .snapshot()
        .mutations
        .pending
        .expect("control token");
    assert_eq!(token.kind, PendingKind::Leds);
    assert!(control.iter().any(|effect| matches!(
        effect,
        Effect::ReplaceTimer {
            token: TimerToken::Mutation(MutationTimerToken::Confirmation(value)),
            ..
        } if *value == token
    )));
    let confirmed = receive(&mut coordinator, vec![36, 101, 20, 2], 1_210);
    assert!(confirmed.iter().any(|effect| matches!(
        effect,
        Effect::Mutation(MutationEvent::Confirmed(value)) if *value == token
    )));
    assert_eq!(coordinator.snapshot().mutations.pending, None);
}

#[test]
fn configuration_write_holds_one_lease_through_ack_and_verified_readback() {
    let (mut coordinator, _) = ready_configuration();
    let schema = coordinator.configuration_schema().expect("schema");
    let values = decode_values(configuration_write_values_1_2_3(), schema).expect("draft");
    let snapshot = coordinator.snapshot();
    let identity = snapshot.configuration.identity.expect("identity");
    let base_revision = snapshot.configuration.current.expect("current").revision;
    let write_effects = coordinator
        .apply(Command::WriteConfiguration {
            generation: snapshot.generation,
            mutation: ConfigurationMutation::Write,
            package_version: identity.package_version.to_owned(),
            fingerprint: identity.fingerprint.to_owned(),
            wire_signature: identity.wire_signature,
            base_revision,
            values,
            now_ms: 1_300,
        })
        .unwrap();
    let token = coordinator.snapshot().mutations.pending.expect("lease");
    assert_eq!(token.kind, PendingKind::ConfigurationWrite);
    assert_eq!(write(&write_effects).payload[0..2], [95, 0]);

    let verification = receive(&mut coordinator, vec![95], 1_310);
    assert_eq!(coordinator.snapshot().mutations.pending, Some(token));
    assert_eq!(write(&verification).payload, [93, 0]);
    let completed = receive(
        &mut coordinator,
        configuration_readback_response_1_2_3(),
        1_320,
    );
    assert!(
        completed.iter().any(|effect| matches!(
            effect,
            Effect::Mutation(MutationEvent::Confirmed(value)) if *value == token
        )),
        "{completed:?}"
    );
    assert_eq!(coordinator.snapshot().mutations.pending, None);
    assert_eq!(coordinator.snapshot().configuration.pending, None);
}

#[test]
fn malformed_write_response_fails_the_exact_lease_and_allows_retry() {
    let (mut coordinator, _) = ready_configuration();
    let schema = coordinator.configuration_schema().expect("schema");
    let values = decode_values(configuration_write_values_1_2_3(), schema).expect("draft");
    let snapshot = coordinator.snapshot();
    let identity = snapshot.configuration.identity.expect("identity");
    let base_revision = snapshot.configuration.current.expect("current").revision;
    coordinator
        .apply(Command::WriteConfiguration {
            generation: snapshot.generation,
            mutation: ConfigurationMutation::TuneApply,
            package_version: identity.package_version.to_owned(),
            fingerprint: identity.fingerprint.to_owned(),
            wire_signature: identity.wire_signature,
            base_revision,
            values,
            now_ms: 1_400,
        })
        .unwrap();
    let token = coordinator
        .snapshot()
        .mutations
        .pending
        .expect("tune lease");
    assert_eq!(token.kind, PendingKind::TuneApply);
    let failed = receive(&mut coordinator, vec![95, 0], 1_410);
    assert!(failed.iter().any(|effect| matches!(
        effect,
        Effect::Failure(Failure::Mutation { token: value, .. }) if *value == token
    )));
    assert_eq!(coordinator.snapshot().mutations.pending, None);
    assert_eq!(coordinator.snapshot().configuration.pending, None);
}

#[test]
fn can_configuration_is_forwarded_once_and_corrupt_cache_is_removed() {
    let mut can = ConnectionApplicationCoordinator::default();
    let mut facts = begin_facts(10);
    facts.preferred_controller_can_id = Some(42);
    can.apply(Command::Begin(facts)).unwrap();
    receive(&mut can, gateway_payload(1), 20);
    receive_can_motor(&mut can, motor_configuration_payload(42), 42, 30);
    receive(&mut can, dynamic_info_payload(), 40);
    receive(&mut can, dynamic_layout_payload(), 50);
    let activation = activate_with_cache(&mut can, 60, Some(configuration_schema_1_2_3()));
    assert_eq!(write(&activation).payload, [34, 42, 93, 0]);

    let mut corrupt = ConnectionApplicationCoordinator::default();
    corrupt.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut corrupt, motor_configuration_payload(42), 20);
    receive(&mut corrupt, dynamic_info_payload(), 30);
    receive(&mut corrupt, dynamic_layout_payload(), 40);
    let activation = activate_with_cache(&mut corrupt, 50, Some(vec![1, 2, 3]));
    assert!(activation.iter().any(|effect| matches!(
        effect,
        Effect::ApplyConfigurationCache(cache)
            if matches!(cache.as_inner(), CacheEffect::Remove { .. })
    )));
    assert_eq!(write(&activation).payload[0..2], [92, 0]);
}

#[test]
fn control_and_shutdown_timers_release_only_their_exact_tokens() {
    let (mut coordinator, _) = ready_configuration();
    let control = coordinator
        .apply(Command::SetControl {
            generation: coordinator.snapshot().generation,
            control: BooleanControl::Headlights,
            enabled: false,
            now_ms: 1_500,
        })
        .unwrap();
    let token = coordinator
        .snapshot()
        .mutations
        .pending
        .expect("control token");
    let timer = control
        .iter()
        .find_map(|effect| match effect {
            Effect::ReplaceTimer {
                token: timer @ TimerToken::Mutation(MutationTimerToken::Confirmation(value)),
                ..
            } if *value == token => Some(*timer),
            _ => None,
        })
        .expect("confirmation timer");
    let timed_out = coordinator
        .apply(Command::TimerFired {
            generation: coordinator.snapshot().generation,
            token: timer,
            now_ms: 4_500,
        })
        .unwrap();
    assert!(timed_out.iter().any(|effect| matches!(
        effect,
        Effect::Mutation(MutationEvent::Failed { token: value, .. }) if *value == token
    )));
    assert_eq!(coordinator.snapshot().mutations.pending, None);

    let shutdown = coordinator
        .apply(Command::ShutdownController {
            generation: coordinator.snapshot().generation,
            now_ms: 4_600,
        })
        .unwrap();
    let shutdown_token = coordinator
        .snapshot()
        .mutations
        .pending
        .expect("shutdown token");
    assert_eq!(shutdown_token.kind, PendingKind::ControllerShutdown);
    assert_eq!(write(&shutdown).payload, [156, 0, 0]);
    let shutdown_timer = shutdown
        .iter()
        .find_map(|effect| match effect {
            Effect::ReplaceTimer {
                token: timer @ TimerToken::Mutation(MutationTimerToken::Shutdown(value)),
                ..
            } if *value == shutdown_token => Some(*timer),
            _ => None,
        })
        .expect("shutdown timer");
    let released = coordinator
        .apply(Command::TimerFired {
            generation: coordinator.snapshot().generation,
            token: shutdown_timer,
            now_ms: 34_600,
        })
        .unwrap();
    assert!(released.iter().any(|effect| matches!(
        effect,
        Effect::Mutation(MutationEvent::Released(value)) if *value == shutdown_token
    )));
    assert_eq!(coordinator.snapshot().mutations.pending, None);
}

#[test]
fn scheduler_timeout_and_reconnect_leave_no_configuration_lease() {
    let (mut coordinator, _) = ready_configuration();
    let schema = coordinator.configuration_schema().expect("schema");
    let values = decode_values(configuration_write_values_1_2_3(), schema).expect("draft");
    let snapshot = coordinator.snapshot();
    let identity = snapshot.configuration.identity.expect("identity");
    let base_revision = snapshot.configuration.current.expect("current").revision;
    let write_effects = coordinator
        .apply(Command::WriteConfiguration {
            generation: snapshot.generation,
            mutation: ConfigurationMutation::Write,
            package_version: identity.package_version.to_owned(),
            fingerprint: identity.fingerprint.to_owned(),
            wire_signature: identity.wire_signature,
            base_revision,
            values,
            now_ms: 1_600,
        })
        .unwrap();
    let token = coordinator.snapshot().mutations.pending.expect("lease");
    let timed_out = coordinator
        .apply(Command::DeadlineFired {
            generation: coordinator.snapshot().generation,
            token: deadline(&write_effects),
            now_ms: 4_600,
        })
        .unwrap();
    assert!(timed_out.iter().any(|effect| matches!(
        effect,
        Effect::Mutation(MutationEvent::Failed { token: value, .. }) if *value == token
    )));
    assert_eq!(coordinator.snapshot().configuration.pending, None);
    assert_eq!(coordinator.snapshot().mutations.pending, None);

    coordinator
        .apply(Command::Begin(begin_facts(5_000)))
        .unwrap();
    let reset = coordinator.snapshot();
    assert_eq!(reset.phase, Phase::Identifying);
    assert!(!reset.mutations.configured);
    assert_eq!(reset.configuration.availability, Availability::Unavailable);
    assert_eq!(reset.configuration.pending, None);
}
