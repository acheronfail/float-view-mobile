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
fn refloat_beta1_skips_layout_and_starts_selectable_float32_telemetry() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_configuration_payload(42), 20);
    let mut beta_info = selectable_info_payload();
    beta_info[27] = 1;
    beta_info[28..33].copy_from_slice(b"beta1");
    let ready = receive(&mut coordinator, beta_info, 30);
    assert!(ready.iter().all(|effect| match effect {
        Effect::Write(request) => request.payload != [36, 101, 32],
        _ => true,
    }));
    let activation = coordinator
        .snapshot()
        .awaiting_activation
        .expect("1.3 activation");
    assert_eq!(activation.refloat_version.to_string(), "1.3.1-beta1");

    let effects = activate_with_cache(
        &mut coordinator,
        40,
        Some(
            include_bytes!("../../../fixtures/refloat-configuration/schemas/1.3.1-beta1.qcompress")
                .to_vec(),
        ),
    );
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
    receive(&mut coordinator, selectable_fast_payload(), 71);
    receive(&mut coordinator, empty_alert_history(), 72);
    receive(&mut coordinator, selectable_fast_payload(), 73);
    receive(&mut coordinator, vec![62], 74);
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
    receive(&mut coordinator, empty_alert_history(), 1_113);
    receive(&mut coordinator, selectable_fast_payload(), 1_114);
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

fn empty_alert_history() -> Vec<u8> {
    vec![36, 101, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
}

fn ready_configuration() -> (ConnectionApplicationCoordinator, TimerToken) {
    ready_configuration_with_test_version(0)
}

fn ready_configuration_with_test_version(
    test_version: u8,
) -> (ConnectionApplicationCoordinator, TimerToken) {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    let mut identity = motor_configuration_payload(42);
    let name_end = 3 + identity[3..].iter().position(|byte| *byte == 0).unwrap();
    identity[name_end + 14] = test_version;
    receive(&mut coordinator, identity, 20);
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
    let history_request = receive(&mut coordinator, dynamic_idle_payload(), 70);
    assert_eq!(write(&history_request).payload, [36, 101, 35, 0, 0, 0, 0]);
    receive(&mut coordinator, empty_alert_history(), 71);
    let bms_request = receive(&mut coordinator, dynamic_idle_payload(), 72);
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
    for test_version in [0, 1, u8::MAX] {
        let (mut coordinator, _) = ready_configuration_with_test_version(test_version);
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
fn cold_refloat_1_3_0_schema_download_completes_through_express_can_1() {
    cold_schema_download(false);
}

#[test]
fn duplicate_schema_pages_warn_without_interrupting_configuration_or_lighting() {
    cold_schema_download(true);
}

fn cold_schema_download(duplicate_each_page: bool) {
    let mut previous_schema = None;
    let mut coordinator = ConnectionApplicationCoordinator::default();
    let mut facts = begin_facts(10);
    facts.preferred_controller_can_id = Some(1);
    coordinator.apply(Command::Begin(facts)).unwrap();
    receive(&mut coordinator, gateway_payload(1), 20);
    receive_can_motor(&mut coordinator, motor_configuration_payload(42), 1, 30);
    receive(&mut coordinator, selectable_info_payload(), 40);
    let mut effects = activate_with_cache(&mut coordinator, 50, None);
    let lighting_timer = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::ReplaceTimer {
                token: token @ TimerToken::Mutation(MutationTimerToken::LightingRefresh(_)),
                ..
            } => Some(*token),
            _ => None,
        })
        .unwrap();
    coordinator
        .apply(Command::TimerFired {
            generation: coordinator.snapshot().generation,
            token: lighting_timer,
            now_ms: 51,
        })
        .unwrap();
    let mut controls_ready_during_download = false;
    let mut telemetry_confirmed_during_download = false;
    let compressed = configuration_schema_1_3_0();
    let mut schema_requests = 0;
    let mut stale_chunks = 0;
    for step in 0..256 {
        if coordinator.snapshot().configuration.current.is_some() {
            break;
        }
        let request = write(&effects).payload.clone();
        let payload = if request == [62] {
            request.as_slice()
        } else {
            assert_eq!(&request[..2], &[34, 1]);
            &request[2..]
        };
        let response = match payload[0] {
            92 => {
                assert_eq!(payload[1], 0);
                let length = u32::from_be_bytes(payload[2..6].try_into().unwrap()) as usize;
                let offset = u32::from_be_bytes(payload[6..10].try_into().unwrap()) as usize;
                if offset > 0 {
                    let before = coordinator.snapshot();
                    let mut duplicate = vec![92, 0];
                    duplicate.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
                    duplicate.extend_from_slice(&0_u32.to_be_bytes());
                    duplicate.extend_from_slice(&compressed[..400]);
                    if duplicate_each_page {
                        duplicate = previous_schema
                            .clone()
                            .expect("previous accepted schema page");
                    }
                    let ignored = receive(&mut coordinator, duplicate, 60 + step * 10);
                    if duplicate_each_page {
                        assert!(ignored.iter().any(|e| matches!(e, Effect::Diagnostic(
                            float_view_connection_application_coordinator::Diagnostic::ConfigurationStale))));
                        assert_eq!(coordinator.snapshot().scheduler, before.scheduler);
                        assert_eq!(coordinator.snapshot().configuration, before.configuration);
                    } else {
                        assert_eq!(ignored, vec![Effect::Diagnostic(
                            float_view_connection_application_coordinator::Diagnostic::ConfigurationStale)]);
                        assert_eq!(
                            coordinator.snapshot(),
                            before,
                            "late schema chunk must retain the active request and deadline"
                        );
                    }
                    stale_chunks += 1;
                }
                let mut response = vec![92, 0];
                response.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
                response.extend_from_slice(&(offset as u32).to_be_bytes());
                response.extend_from_slice(
                    &compressed[offset..(offset + length).min(compressed.len())],
                );
                schema_requests += 1;
                response
            }
            93 => configuration_current_response_1_3_0(),
            62 => vec![62],
            36 if payload.get(2) == Some(&20) => {
                assert!(schema_requests < compressed.len().div_ceil(400));
                vec![36, 101, 20, 3]
            }
            14 => current_limits_payload(0x2EFD_0142, Some(15)),
            36 if payload.get(2) == Some(&35) => empty_alert_history(),
            36 if payload.get(2) == Some(&33) => {
                if payload.get(3) == Some(&1) {
                    selectable_fast_payload()
                } else {
                    selectable_fixture("1.3.0-command-33-slow-gnss-float32")
                }
            }
            _ => panic!("Unexpected bootstrap request: {payload:?}"),
        };
        if response.first() == Some(&92) {
            previous_schema = Some(response.clone());
        }
        effects = receive(&mut coordinator, response, 60 + step * 10);
        let snapshot = coordinator.snapshot();
        if snapshot.configuration.availability == Availability::Loading {
            let controls = snapshot.mutations.controls.as_ref().unwrap();
            telemetry_confirmed_during_download |= controls.package_enabled.is_some()
                && controls.flywheel_enabled.is_some()
                && controls.leds_enabled.is_none();
            controls_ready_during_download |= controls.status == ControlStatus::Ready;
        }
    }
    assert_eq!(
        coordinator.snapshot().possible_can_id_conflict,
        duplicate_each_page
    );
    let config = coordinator.snapshot().configuration;
    assert!(stale_chunks > 0);
    assert!(telemetry_confirmed_during_download);
    assert!(controls_ready_during_download);
    assert!(
        schema_requests > 1,
        "cold bootstrap must download the entire schema"
    );
    assert_eq!(config.availability, Availability::Ready);
    assert_eq!(config.identity.unwrap().package_version, "1.3.0");
    assert!(config.current.is_some());
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

fn selectable_idle_payload() -> Vec<u8> {
    let mut payload = selectable_fast_payload();
    // Extra flags precede state flags; report ready, normal, no footpads,
    // zero speed and zero eRPM through the production decoder.
    payload[17..21].copy_from_slice(&0x0200_0000_u32.to_be_bytes());
    payload[21..29].fill(0);
    payload
}

fn ready_beta() -> ConnectionApplicationCoordinator {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_configuration_payload(42), 20);
    let mut info = selectable_info_payload();
    info[27] = 1;
    info[28..33].copy_from_slice(b"beta1");
    receive(&mut coordinator, info, 30);
    let activated = activate_with_cache(
        &mut coordinator,
        40,
        Some(
            include_bytes!("../../../fixtures/refloat-configuration/schemas/1.3.1-beta1.qcompress")
                .to_vec(),
        ),
    );
    let lighting_timer = activated
        .iter()
        .find_map(|effect| match effect {
            Effect::ReplaceTimer {
                token: token @ TimerToken::Mutation(MutationTimerToken::LightingRefresh(_)),
                ..
            } => Some(*token),
            _ => None,
        })
        .unwrap();
    receive(&mut coordinator, configuration_current_response_1_3_0(), 50);
    receive(
        &mut coordinator,
        current_limits_payload(0x2EFD_0142, Some(15)),
        60,
    );
    receive(&mut coordinator, selectable_idle_payload(), 70);
    receive(&mut coordinator, selectable_idle_payload(), 71);
    receive(&mut coordinator, empty_alert_history(), 72);
    receive(&mut coordinator, selectable_idle_payload(), 73);
    receive(&mut coordinator, vec![62], 74);
    let effects = coordinator
        .apply(Command::TimerFired {
            generation: coordinator.snapshot().generation,
            token: lighting_timer,
            now_ms: 1050,
        })
        .unwrap();
    assert_eq!(write(&effects).payload, [36, 101, 20, 0, 0, 0, 0, 0]);
    receive(&mut coordinator, vec![36, 101, 20, 3], 1060);
    coordinator
}

#[test]
fn beta_mutations_recheck_alert_authority_before_another_sample() {
    use float_view_connection_application_coordinator::CoordinatorError;
    use float_view_device_mutation_coordinator::CoordinatorError::InvalidState;
    for failure in ["expired", "timeout", "malformed"] {
        let mut coordinator = ready_beta();
        let mut now_ms = 1070;
        if failure != "expired" {
            let effects = coordinator
                .apply(Command::Refresh {
                    generation: coordinator.snapshot().generation,
                    now_ms,
                })
                .unwrap();
            assert_eq!(write(&effects).payload[2], 33);
            receive(&mut coordinator, selectable_idle_payload(), 1071);
            let alerts = receive(
                &mut coordinator,
                selectable_fixture("1.3.0-command-33-slow-gnss-float32"),
                1072,
            );
            assert_eq!(write(&alerts).payload[2], 35);
            if failure == "timeout" {
                coordinator
                    .apply(Command::DeadlineFired {
                        generation: coordinator.snapshot().generation,
                        token: deadline(&alerts),
                        now_ms: 1972,
                    })
                    .unwrap();
                now_ms = 1973;
            } else {
                receive(&mut coordinator, vec![36, 101, 35], 1073);
                now_ms = 1074;
            }
        } else {
            now_ms = 60_000;
        }
        // Submission must query independent history authority, even while
        // the mutation owner still has the previous healthy sample.
        for control in [BooleanControl::Package, BooleanControl::Flywheel] {
            assert_eq!(
                coordinator.apply(Command::SetControl {
                    generation: coordinator.snapshot().generation,
                    control,
                    enabled: control == BooleanControl::Flywheel,
                    now_ms,
                }),
                Err(CoordinatorError::Mutation(InvalidState)),
                "{failure}"
            );
        }
        let snapshot = coordinator.snapshot();
        let identity = snapshot.configuration.identity.unwrap();
        let current = snapshot.configuration.current.unwrap();
        for mutation in [
            ConfigurationMutation::Write,
            ConfigurationMutation::TuneApply,
        ] {
            assert_eq!(
                coordinator.apply(Command::WriteConfiguration {
                    generation: snapshot.generation,
                    mutation,
                    package_version: identity.package_version.to_owned(),
                    fingerprint: identity.fingerprint.to_owned(),
                    wire_signature: identity.wire_signature,
                    base_revision: current.revision,
                    values: current.values.clone(),
                    now_ms,
                }),
                Err(CoordinatorError::Mutation(InvalidState)),
                "{failure}"
            );
            assert_eq!(coordinator.snapshot().configuration.pending, None);
            assert_eq!(coordinator.snapshot().mutations.pending, None);
        }
    }
}

#[test]
fn beta_fresh_inactive_alert_authority_allows_guarded_submissions() {
    for control in [BooleanControl::Package, BooleanControl::Flywheel] {
        let mut coordinator = ready_beta();
        assert!(
            coordinator
                .apply(Command::SetControl {
                    generation: coordinator.snapshot().generation,
                    control,
                    enabled: control == BooleanControl::Flywheel,
                    now_ms: 1070,
                })
                .is_ok()
        );
    }
    for mutation in [
        ConfigurationMutation::Write,
        ConfigurationMutation::TuneApply,
    ] {
        let mut coordinator = ready_beta();
        let snapshot = coordinator.snapshot();
        let identity = snapshot.configuration.identity.unwrap();
        let current = snapshot.configuration.current.unwrap();
        assert!(
            coordinator
                .apply(Command::WriteConfiguration {
                    generation: snapshot.generation,
                    mutation,
                    package_version: identity.package_version.to_owned(),
                    fingerprint: identity.fingerprint.to_owned(),
                    wire_signature: identity.wire_signature,
                    base_revision: current.revision,
                    values: current.values,
                    now_ms: 1070,
                })
                .is_ok()
        );
    }
}
