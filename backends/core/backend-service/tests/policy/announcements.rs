use super::*;
use crate::announcements::{Outcome, Readiness, Speech, TestKind};
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
