use super::*;
use crate::announcements::{Battery, Outcome, Readiness, Speech, TestKind};
use float_view_application_runtime::telemetry::NormalizedSample;

#[allow(dead_code)]
mod samples {
    use float_view_application_runtime::telemetry as float_view_refloat_telemetry_service;
    include!("../../../session-sqlite/tests/support/mod.rs");
}

fn observe(t: &mut Test, sample: &NormalizedSample, now: u64) -> Vec<Effect> {
    t.service.policy.exchange.now = now;
    t.service.observe_announcement(sample);
    t.service.step_announcements().unwrap();
    t.service.drain_os().effects
}

fn playing(speed: bool, load: bool) -> (Test, NormalizedSample, Speech) {
    let mut t = Test::new();
    t.activate(false);
    t.drain();
    t.service.policy.announcements.audio.status = Readiness::Ready;
    let settings = &mut t.service.policy.settings.announcements;
    settings.speed_enabled = speed;
    settings.load_enabled = load;
    let mut sample = samples::sample(1, 100);
    sample.speed_kph = Some(32.);
    sample.duty_cycle = Some(0.8);
    let speech = observe(&mut t, &sample, 100)
        .into_iter()
        .find_map(|e| match e {
            Effect::Speak { speech } => Some(speech),
            _ => None,
        })
        .unwrap();
    assert_eq!(speech.speed.is_some(), speed);
    assert_eq!(speech.load.is_some(), load);
    (t, sample, speech)
}

fn assert_no_playback_change(effects: &[Effect]) {
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::CancelSpeech { .. } | Effect::Speak { .. })),
        "unexpected playback change: {effects:?}"
    );
}

#[test]
fn announcements_finish_below_threshold_without_repeats_or_cooldown_reset() {
    for (speed, load) in [(true, false), (false, true), (true, true)] {
        let (mut t, mut sample, speech) = playing(speed, load);
        // Equality stops eligibility; reverse speed still uses magnitude.
        sample.speed_kph = Some(-25.);
        sample.duty_cycle = Some(0.65);
        assert_no_playback_change(&observe(&mut t, &sample, 200));
        assert!(matches!(
            t.service.test_announcement(7, TestKind::Combined),
            Err(Failure::InvalidState)
        ));
        t.fact(
            Fact::SpeechFinished {
                id: speech.id,
                outcome: Outcome::Finished,
            },
            300,
        )
        .unwrap();
        assert_no_playback_change(&t.service.drain_os().effects);
        sample.speed_kph = Some(34.);
        sample.duty_cycle = Some(0.85);
        assert_no_playback_change(&observe(&mut t, &sample, 400));
        sample.speed_kph = Some(0.);
        sample.duty_cycle = Some(0.);
        for now in [2000, 3500, 5000, 5100, 6500] {
            assert_no_playback_change(&observe(&mut t, &sample, now));
        }
        sample.speed_kph = Some(37.);
        sample.duty_cycle = Some(0.9);
        let effects = observe(&mut t, &sample, 6600);
        let next = effects
            .iter()
            .find_map(|e| match e {
                Effect::Speak { speech } => Some(speech),
                _ => None,
            })
            .unwrap();
        assert_ne!(next.id, speech.id);
        assert_eq!(next.speed, speed.then_some(37));
        assert_eq!(next.load.as_ref().map(|v| v.percent), load.then_some(90));
        t.service.complete_speech(speech.id, Outcome::Finished);
        assert!(matches!(
            t.service.test_announcement(7, TestKind::Speed),
            Err(Failure::InvalidState)
        ));
    }
}

#[test]
fn announcements_below_threshold_still_cancel_for_invalid_context_settings_and_deadline() {
    for reason in ["stale", "disconnect", "controller", "settings", "timeout"] {
        let (mut t, mut sample, speech) = playing(true, true);
        sample.speed_kph = Some(0.);
        sample.duty_cycle = Some(0.);
        assert_no_playback_change(&observe(&mut t, &sample, 200));
        match reason {
            "stale" => t.fact(Fact::Clock {}, 2200).unwrap(),
            "disconnect" => {
                t.service
                    .dispatch(1, crate::operations::Operation::Disconnect)
                    .unwrap();
                t.fact(Fact::Clock {}, 201).unwrap();
            }
            "controller" => {
                t.service.policy.announcements.invalidate();
                t.service.step_announcements().unwrap();
            }
            "settings" => {
                let mut settings = t.service.policy.settings.announcements.clone();
                settings.speed_enabled = false;
                settings.load_enabled = false;
                t.service
                    .dispatch(
                        1,
                        crate::operations::Operation::UpdateSettings {
                            revision: t.service.policy.settings.revision,
                            patch_json: serde_json::json!({"announcements": settings}).to_string(),
                        },
                    )
                    .unwrap();
            }
            "timeout" => {
                for now in [1500, 3000, 4500, 6000, 7500, 9000, 10099] {
                    assert_no_playback_change(&observe(&mut t, &sample, now));
                }
                t.fact(Fact::Clock {}, 10100).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            t.service
                .drain_os()
                .effects
                .iter()
                .any(|e| matches!(e, Effect::CancelSpeech { id } if *id == speech.id)),
            "missing cancellation for {reason}"
        );
    }
}

fn footpads(delay: u64) -> (Test, NormalizedSample) {
    let mut t = Test::new();
    t.activate(false);
    t.drain();
    t.service.policy.announcements.audio.status = Readiness::Ready;
    let settings = &mut t.service.policy.settings.announcements;
    settings.footpad_enabled = true;
    settings.footpad_delay_seconds = delay;
    let mut sample = samples::sample(1, 100);
    sample.speed_kph = Some(10.);
    (t, sample)
}
fn spoken(effects: &[Effect]) -> Option<&Speech> {
    effects.iter().find_map(|e| match e {
        Effect::Speak { speech } => Some(speech),
        _ => None,
    })
}

#[test]
fn footpad_mapping_exact_boundary_independent_sides_and_timer() {
    use crate::announcements::Footpad;
    use float_view_application_runtime::telemetry::FootpadState;
    for (state, expected) in [
        (FootpadState::Right, Footpad::Left),
        (FootpadState::Left, Footpad::Right),
        (FootpadState::None, Footpad::Both),
    ] {
        for delay in 1..=5 {
            let (mut t, mut sample) = footpads(delay);
            sample.footpad_state = Some(state);
            assert!(spoken(&observe(&mut t, &sample, 100)).is_none());
            assert!(
                t.service
                    .policy
                    .timers
                    .values()
                    .any(|v| v.kind == Timer::Speech)
            );
            for now in (1100..100 + delay * 1000).step_by(1000) {
                assert!(spoken(&observe(&mut t, &sample, now)).is_none());
            }
            assert!(spoken(&observe(&mut t, &sample, 99 + delay * 1000)).is_none());
            t.service.policy.exchange.now = 100 + delay * 1000;
            t.service.step_announcements().unwrap();
            let effects = t.service.drain_os().effects;
            let speech = spoken(&effects).unwrap();
            assert_eq!(speech.footpad, Some(expected));
            assert!(speech.load.is_none() && speech.speed.is_none());
        }
    }
    let (mut t, mut sample) = footpads(2);
    sample.footpad_state = Some(FootpadState::Right);
    observe(&mut t, &sample, 100);
    sample.footpad_state = Some(FootpadState::None);
    observe(&mut t, &sample, 1100);
    let effects = observe(&mut t, &sample, 2100);
    let speech = spoken(&effects).unwrap();
    assert_eq!(speech.footpad, Some(Footpad::Left));
    t.service.complete_speech(speech.id, Outcome::Finished);
    for now in [3100, 4100, 5100, 6100] {
        observe(&mut t, &sample, now);
    }
    assert_eq!(
        spoken(&observe(&mut t, &sample, 7100)).unwrap().footpad,
        Some(Footpad::Both)
    );
}

#[test]
fn footpad_gate_and_invalid_evidence_reset_continuity() {
    use crate::announcements::Footpad;
    use float_view_application_runtime::telemetry::FootpadState;
    for speed in [
        None,
        Some(5.),
        Some(-5.),
        Some(0.),
        Some(201.),
        Some(f64::NAN),
        Some(f64::INFINITY),
    ] {
        let (mut t, mut sample) = footpads(1);
        sample.footpad_state = Some(FootpadState::None);
        observe(&mut t, &sample, 100);
        sample.speed_kph = speed;
        assert!(spoken(&observe(&mut t, &sample, 1100)).is_none());
        sample.speed_kph = Some(-5.01);
        t.service.policy.settings.unit_preference = "imperial".into();
        assert!(spoken(&observe(&mut t, &sample, 1200)).is_none());
        assert!(spoken(&observe(&mut t, &sample, 2199)).is_none());
        assert_eq!(
            spoken(&observe(&mut t, &sample, 2200)).unwrap().footpad,
            Some(Footpad::Both)
        );
    }
    for state in [None, Some(FootpadState::Unknown), Some(FootpadState::Both)] {
        let (mut t, mut sample) = footpads(1);
        sample.footpad_state = Some(FootpadState::None);
        observe(&mut t, &sample, 100);
        sample.footpad_state = state;
        assert!(spoken(&observe(&mut t, &sample, 1100)).is_none());
        sample.footpad_state = Some(FootpadState::None);
        assert!(spoken(&observe(&mut t, &sample, 1200)).is_none());
        assert_eq!(
            spoken(&observe(&mut t, &sample, 2200)).unwrap().footpad,
            Some(Footpad::Both)
        );
    }
}

#[test]
fn footpad_gap_worker_context_and_settings_resets() {
    use float_view_application_runtime::telemetry::FootpadState;
    for reset in ["gap", "timer", "controller", "delay", "disable", "restart"] {
        let (mut t, mut sample) = footpads(1);
        sample.footpad_state = Some(FootpadState::Right);
        observe(&mut t, &sample, 100);
        let now = match reset {
            "gap" => 2100,
            "timer" => {
                t.service.policy.exchange.now = 2100;
                t.service.step_announcements().unwrap();
                2200
            }
            "controller" => {
                t.service.policy.announcements.invalidate();
                1100
            }
            "delay" | "disable" => {
                let mut settings = t.service.policy.settings.announcements.clone();
                if reset == "delay" {
                    settings.footpad_delay_seconds = 2;
                } else {
                    settings.footpad_enabled = false;
                }
                t.service
                    .dispatch(
                        1,
                        crate::operations::Operation::UpdateSettings {
                            revision: t.service.policy.settings.revision,
                            patch_json: serde_json::json!({"announcements":settings}).to_string(),
                        },
                    )
                    .unwrap();
                1100
            }
            "restart" => {
                t.service.policy.announcements = Default::default();
                1100
            }
            _ => unreachable!(),
        };
        assert!(
            spoken(&observe(&mut t, &sample, now)).is_none(),
            "reset {reset}"
        );
        assert!(
            spoken(&observe(&mut t, &sample, now + 999)).is_none(),
            "reset {reset}"
        );
    }
}

#[test]
fn footpads_share_slot_repeats_cooldown_and_recovery_lifecycle() {
    use crate::announcements::Footpad;
    use float_view_application_runtime::telemetry::FootpadState;
    let (mut t, mut sample) = footpads(1);
    sample.footpad_state = Some(FootpadState::Left);
    observe(&mut t, &sample, 100);
    let effects = observe(&mut t, &sample, 1100);
    let id = spoken(&effects).unwrap().id;
    sample.footpad_state = Some(FootpadState::Both);
    sample.speed_kph = Some(0.);
    assert_no_playback_change(&observe(&mut t, &sample, 1200));
    t.service.complete_speech(id, Outcome::Finished);
    sample.footpad_state = Some(FootpadState::None);
    sample.speed_kph = Some(32.);
    sample.duty_cycle = Some(0.78);
    t.service.policy.settings.announcements.speed_enabled = true;
    t.service.policy.settings.announcements.load_enabled = true;
    for now in [1300, 2300, 3300, 4300, 5300, 6099] {
        assert_no_playback_change(&observe(&mut t, &sample, now));
    }
    t.service.speech_client_disconnected(7);
    let effects = observe(&mut t, &sample, 6100);
    let next = spoken(&effects).unwrap();
    assert_eq!(next.footpad, Some(Footpad::Both));
    assert_eq!(next.speed, Some(32));
    assert_eq!(next.load.as_ref().unwrap().percent, 78);
    t.service.complete_speech(id, Outcome::Finished);
    assert!(matches!(
        t.service.test_announcement(7, TestKind::Speed),
        Err(Failure::InvalidState)
    ));
    t.service.policy.exchange.now = 8100;
    t.service.step_announcements().unwrap();
    assert!(
        t.service
            .drain_os()
            .effects
            .iter()
            .any(|e| matches!(e, Effect::CancelSpeech { id } if *id == next.id))
    );
}

#[test]
fn footpad_saved_and_guest_share_policy_and_gain_edits_preserve_elapsed_time() {
    use crate::announcements::Footpad;
    use float_view_application_runtime::telemetry::FootpadState;
    for saved in [false, true] {
        let mut t = Test::new();
        t.activate(saved);
        t.drain();
        t.service.policy.announcements.audio.status = Readiness::Ready;
        t.service.policy.settings.announcements.footpad_enabled = true;
        let mut sample = samples::sample(1, 100);
        sample.speed_kph = Some(10.);
        sample.footpad_state = Some(FootpadState::Right);
        observe(&mut t, &sample, 100);
        t.service.policy.exchange.now = 500;
        let mut settings = t.service.policy.settings.announcements.clone();
        settings.volume_percent = 42;
        t.service
            .dispatch(
                7,
                crate::operations::Operation::UpdateSettings {
                    revision: t.service.policy.settings.revision,
                    patch_json: serde_json::json!({"announcements":settings}).to_string(),
                },
            )
            .unwrap();
        assert!(spoken(&t.service.drain_os().effects).is_none());
        t.service.speech_client_disconnected(7);
        let effects = observe(&mut t, &sample, 1100);
        let speech = spoken(&effects).unwrap();
        assert_eq!(speech.footpad, Some(Footpad::Left));
        assert_eq!(speech.volume, 42);
        // A delayed worker must cancel the old phrase before refreshing stale context.
        let id = speech.id;
        let effects = observe(&mut t, &sample, 3100);
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::CancelSpeech { id: old } if *old == id))
        );
        assert!(spoken(&effects).is_none());
    }
}

#[test]
fn footpad_busy_muted_unavailable_attempts_drop_without_catchup() {
    use float_view_application_runtime::telemetry::FootpadState;
    for blocked in ["busy", "muted", "unavailable"] {
        let (mut t, mut sample) = footpads(1);
        sample.footpad_state = Some(FootpadState::Right);
        observe(&mut t, &sample, 100);
        let mut manual = None;
        if blocked == "busy" {
            t.service.test_announcement(7, TestKind::Combined).unwrap();
            let effects = t.service.drain_os().effects;
            manual = Some(spoken(&effects).unwrap().id);
        } else if blocked == "muted" {
            t.service.policy.settings.announcements.volume_percent = 0;
        } else {
            t.service.policy.announcements.audio.status = Readiness::Unavailable;
        }
        assert!(spoken(&observe(&mut t, &sample, 1100)).is_none());
        if blocked == "busy" {
            for now in [2100, 3100, 4100, 5100] {
                assert!(spoken(&observe(&mut t, &sample, now)).is_none());
            }
        }
        if let Some(id) = manual {
            t.service.complete_speech(id, Outcome::Finished);
        }
        t.service.policy.settings.announcements.volume_percent = 100;
        t.service.policy.announcements.audio.status = Readiness::Ready;
        // Busy previews advance the same cooldown; failed automatic attempts do too.
        let due = if blocked == "busy" { 10100 } else { 6100 };
        for now in (if blocked == "busy" { 5200 } else { 1200 }..due).step_by(500) {
            assert!(spoken(&observe(&mut t, &sample, now)).is_none());
        }
        assert!(spoken(&observe(&mut t, &sample, due)).is_some());
    }
}

#[test]
fn footpad_contact_recovery_is_per_side_and_missing_contact_does_not_suppress_other_rules() {
    use crate::announcements::Footpad;
    use float_view_application_runtime::telemetry::FootpadState;
    let (mut t, mut sample) = footpads(1);
    sample.footpad_state = Some(FootpadState::None);
    observe(&mut t, &sample, 100);
    let effects = observe(&mut t, &sample, 1100);
    let both = spoken(&effects).unwrap();
    assert_eq!(both.footpad, Some(Footpad::Both));
    t.service.complete_speech(both.id, Outcome::Finished);
    sample.footpad_state = Some(FootpadState::Right);
    for now in [1200, 2200, 3200, 4200, 5200] {
        assert!(spoken(&observe(&mut t, &sample, now)).is_none());
    }
    let effects = observe(&mut t, &sample, 6100);
    let left = spoken(&effects).unwrap();
    assert_eq!(left.footpad, Some(Footpad::Left));
    t.service.complete_speech(left.id, Outcome::Finished);
    sample.footpad_state = None;
    sample.speed_kph = Some(32.);
    sample.duty_cycle = Some(0.78);
    t.service.policy.settings.announcements.speed_enabled = true;
    t.service.policy.settings.announcements.load_enabled = true;
    for now in [6200, 7200, 8200, 9200, 10200] {
        assert!(spoken(&observe(&mut t, &sample, now)).is_none());
    }
    let effects = observe(&mut t, &sample, 11100);
    let fallback = spoken(&effects).unwrap();
    assert_eq!(fallback.footpad, None);
    assert_eq!(fallback.speed, Some(32));
    assert_eq!(fallback.load.as_ref().unwrap().percent, 78);
}

fn battery_board() -> (Test, NormalizedSample) {
    let mut t = Test::new();
    t.discover();
    t.service.connect("test-device", false).unwrap();
    let generation = t.service.policy.active.as_ref().unwrap().generation;
    t.fact(
        Fact::Link {
            generation,
            state: LinkState::Ready,
        },
        5,
    )
    .unwrap();
    let generation = t.service.runtime.snapshot().application.generation;
    let mut firmware = vec![0, 6, 6];
    firmware.extend_from_slice(b"THOR300\0");
    firmware.extend_from_slice(&[7; 12]);
    firmware.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 0]);
    firmware.extend_from_slice(b"Float\0");
    for payload in [firmware, vec![36, 101, 0, 10]] {
        t.service
            .runtime_command(Command::Application(c::Command::Receive {
                generation,
                payload,
                now_ms: 6,
                observed_at_epoch_ms: 1_700_000_000_006,
            }))
            .unwrap();
    }
    t.drain();
    let schema =
        include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.0.0.qcompress");
    let receive = |t: &mut Test, payload| {
        t.service
            .runtime_command(Command::Application(c::Command::Receive {
                generation,
                payload,
                now_ms: 10,
                observed_at_epoch_ms: 1_700_000_000_010,
            }))
            .unwrap();
        t.drain();
    };
    let mut limits = vec![14];
    limits.extend_from_slice(&0x2EFD_0142_u32.to_be_bytes());
    limits.extend_from_slice(&[0; 4]);
    for value in [100_f32, -100., 50., -20.] {
        limits.extend_from_slice(&value.to_be_bytes());
    }
    let mut current = vec![93, 0];
    current.extend_from_slice(include_bytes!(
        "../../../../fixtures/refloat-configuration/values/1.0.0-current.bin"
    ));
    for _ in 0..100 {
        let snapshot = t.service.runtime.snapshot().application;
        if snapshot.configuration.current.is_some() {
            break;
        }
        let request = snapshot.scheduler.in_flight.expect("configuration request");
        let response = match request.expected_response.command {
            14 => limits.clone(),
            92 => {
                let offset =
                    i32::from_be_bytes(request.payload[6..10].try_into().unwrap()) as usize;
                let mut payload = vec![92, 0];
                payload.extend_from_slice(&(schema.len() as i32).to_be_bytes());
                payload.extend_from_slice(&(offset as i32).to_be_bytes());
                payload.extend_from_slice(&schema[offset..(offset + 400).min(schema.len())]);
                payload
            }
            93 => current.clone(),
            62 => vec![62, 42],
            36 => {
                t.riding_sample(10);
                continue;
            }
            _ => panic!("unexpected request: {request:?}"),
        };
        receive(&mut t, response);
    }
    assert!(
        t.service
            .runtime
            .snapshot()
            .application
            .configuration
            .current
            .is_some()
    );
    t.service.policy.announcements.audio.status = Readiness::Ready;
    let mut sample = samples::sample(1, 100);
    sample.package_state = Some(float_view_application_runtime::telemetry::PackageState::Running);
    sample.speed_kph = None;
    sample.pack_voltage_v = Some(40.);
    (t, sample)
}

#[test]
fn battery_priority_busy_mute_and_context_cancellation_use_one_slot() {
    use crate::announcements::Battery;
    let (mut t, mut sample) = battery_board();
    let effects = observe(&mut t, &sample, 100);
    let first = spoken(&effects).unwrap();
    assert_eq!(first.battery, Some(Battery::Low));
    assert!(first.footpad.is_none() && first.load.is_none() && first.speed.is_none());
    let id = first.id;
    sample.pack_voltage_v = Some(100.);
    assert!(spoken(&observe(&mut t, &sample, 200)).is_none());
    t.service.complete_speech(id, Outcome::Finished);
    let effects = observe(&mut t, &sample, 300);
    let high = spoken(&effects).unwrap();
    assert_eq!(high.battery, Some(Battery::OverVoltage));
    t.service.complete_speech(high.id, Outcome::Finished);
    // Each kind has its own interval; stop/start and WebView changes cannot clear it.
    sample.package_state = Some(float_view_application_runtime::telemetry::PackageState::Ready);
    assert!(spoken(&observe(&mut t, &sample, 400)).is_none());
    sample.package_state = Some(float_view_application_runtime::telemetry::PackageState::Running);
    sample.pack_voltage_v = Some(40.);
    t.service.policy.settings.announcements.volume_percent = 0;
    assert!(spoken(&observe(&mut t, &sample, 60_100)).is_none());
    t.service.policy.settings.announcements.volume_percent = 100;
    assert!(spoken(&observe(&mut t, &sample, 60_200)).is_none());
    let effects = observe(&mut t, &sample, 120_100);
    let next = spoken(&effects).unwrap();
    sample.pack_voltage_v = None;
    assert!(
        observe(&mut t, &sample, 120_200)
            .iter()
            .any(|e| matches!(e, Effect::CancelSpeech { id } if *id == next.id))
    );
}

#[test]
fn battery_running_exit_finishes_admitted_phrase_and_disable_cancels_it() {
    let (mut t, mut sample) = battery_board();
    t.service.policy.settings.announcements.load_enabled = true;
    sample.duty_cycle = Some(0.9);
    let effects = observe(&mut t, &sample, 100);
    let first = spoken(&effects).unwrap();
    assert!(first.battery.is_some());
    assert!(
        first.load.is_none(),
        "battery gets priority over qualifying routine load"
    );
    sample.package_state = Some(float_view_application_runtime::telemetry::PackageState::Ready);
    assert_no_playback_change(&observe(&mut t, &sample, 200));
    t.service
        .policy
        .settings
        .announcements
        .battery_charge_enabled = false;
    t.service.reset_battery_announcements();
    assert!(
        t.service
            .drain_os()
            .effects
            .iter()
            .any(|e| matches!(e,Effect::CancelSpeech{id} if *id == first.id))
    );
    t.service.complete_speech(first.id, Outcome::Finished);
    sample.package_state = Some(float_view_application_runtime::telemetry::PackageState::Running);
    t.service
        .policy
        .settings
        .announcements
        .battery_charge_enabled = true;
    assert!(spoken(&observe(&mut t, &sample, 300)).is_none());
}

#[test]
fn host_playback_flag_includes_battery_and_footpad_only_settings() {
    let mut t = Test::new();
    t.drain();
    for (battery, footpad, volume, expected) in [
        (false, false, 100, false),
        (true, false, 100, true),
        (false, false, 100, false),
        (false, true, 100, true),
        (true, true, 0, false),
    ] {
        let settings = &mut t.service.policy.settings.announcements;
        settings.speed_enabled = false;
        settings.load_enabled = false;
        settings.battery_charge_enabled = battery;
        settings.footpad_enabled = footpad;
        settings.volume_percent = volume;
        t.service.changed().unwrap();
        let effects = t.service.drain_os().effects;
        assert!(effects.iter().any(|e| matches!(e, Effect::HostState { speech_enabled, .. } if *speech_enabled == expected)), "missing playback flag for battery={battery}, footpad={footpad}, volume={volume}");
    }
}

#[test]
fn battery_visual_state_survives_speech_settings_cooldown_and_stopping_but_not_stale() {
    let (mut t, mut sample) = battery_board();
    t.service
        .policy
        .settings
        .announcements
        .battery_charge_enabled = false;
    assert!(spoken(&observe(&mut t, &sample, 100)).is_none());
    assert_eq!(
        t.service
            .published_snapshot()
            .active
            .unwrap()
            .battery_warning,
        Some(Battery::Low)
    );
    t.service
        .policy
        .settings
        .announcements
        .battery_charge_enabled = true;
    let effects = observe(&mut t, &sample, 200);
    let speech = spoken(&effects).unwrap();
    t.service.complete_speech(speech.id, Outcome::Finished);
    assert!(spoken(&observe(&mut t, &sample, 300)).is_none());
    assert_eq!(t.service.battery_warning(), Some(Battery::Low));
    sample.package_state = Some(float_view_application_runtime::telemetry::PackageState::Ready);
    observe(&mut t, &sample, 400);
    assert_eq!(t.service.battery_warning(), Some(Battery::Low));
    sample.package_state = Some(float_view_application_runtime::telemetry::PackageState::Running);
    observe(&mut t, &sample, 500);
    assert_eq!(t.service.battery_warning(), Some(Battery::Low));
    t.service.changed().unwrap();
    let revision = t.service.service_revision();
    // A timer/clock freshness transition must publish even without new telemetry.
    t.fact(Fact::Clock {}, 2500).unwrap();
    assert_eq!(t.service.battery_warning(), None);
    assert!(t.service.service_revision() > revision);
    assert_eq!(
        t.service
            .published_snapshot()
            .active
            .unwrap()
            .battery_warning,
        None
    );
}

#[test]
fn profile_full_voltage_drives_stationary_visuals_without_changing_speech() {
    let mut t = Test::new();
    t.activate(true);
    let id = t.service.policy.active.as_ref().unwrap().board.as_ref().unwrap().saved_device_id.clone();
    let profile = t.service.store.update_profile(&id, 0,
        r#"{"seriesCount":20,"parallelCount":2,"batteryCellProfileId":"molicel-p45b","batteryMinimumCellVoltageV":3.0}"#, 5).unwrap();
    t.service.changed().unwrap();
    let mut sample = samples::sample(1, 100);
    sample.package_state = None; // Visuals do not require Refloat running evidence.
    sample.pack_voltage_v = Some(84.001);
    sample.battery_current_a = Some(5.);
    assert!(spoken(&observe(&mut t, &sample, 100)).is_none());
    assert_eq!(t.service.battery_warning(), Some(Battery::OverVoltage));
    assert!(t.service.policy.timers.values().any(|timer| timer.kind == Timer::Speech));
    t.service.changed().unwrap();
    assert_eq!(t.service.published_snapshot().active.unwrap().battery_warning, Some(Battery::OverVoltage));
    // A profile edit immediately clears old qualification; no old sample is reused.
    let profile = t.service.store.update_profile(&id, profile.revision, r#"{"seriesCount":21}"#, 6).unwrap();
    t.service.changed().unwrap();
    assert_eq!(t.service.battery_warning(), None);
    assert!(spoken(&observe(&mut t, &sample, 200)).is_none());
    assert_eq!(t.service.battery_warning(), None);
    sample.pack_voltage_v = Some(88.201);
    observe(&mut t, &sample, 300);
    assert_eq!(t.service.battery_warning(), Some(Battery::OverVoltage));
    t.service.store.update_profile(&id, profile.revision, r#"{"batteryCellProfileId":null}"#, 7).unwrap();
    t.service.changed().unwrap();
    observe(&mut t, &sample, 400);
    assert_eq!(t.service.battery_warning(), None);
}
