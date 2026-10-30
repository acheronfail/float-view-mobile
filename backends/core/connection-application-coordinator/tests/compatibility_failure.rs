mod support;
use float_view_connection_application_coordinator::{
    Command, ConnectionApplicationCoordinator, Effect, Phase, discovery::IssueCode,
};
use support::*;

#[test]
fn stable_suffix_artifact_uses_canonical_activation_but_prereleases_stay_blocked() {
    for suffix in ["", "0", "beta2", "0-beta2", "00", "1"] {
        let mut c = ConnectionApplicationCoordinator::default();
        c.apply(Command::Begin(begin_facts(10))).unwrap();
        receive(&mut c, motor_payload(7), 20);
        let mut info = selectable_info_payload();
        info[28..48].fill(0);
        info[28..28 + suffix.len()].copy_from_slice(suffix.as_bytes());
        let effects = receive(&mut c, info, 30);
        let snapshot = c.snapshot();
        if suffix.is_empty() || suffix == "0" {
            assert_eq!(snapshot.phase, Phase::AwaitingActivation);
            assert_eq!(
                snapshot
                    .awaiting_activation
                    .unwrap()
                    .refloat_version
                    .to_string(),
                "1.3.0"
            );
        } else {
            assert_eq!(snapshot.phase, Phase::Failed);
            assert_eq!(
                snapshot.discovery.issue.unwrap().code,
                IssueCode::UnsupportedRefloat
            );
            assert!(
                effects
                    .iter()
                    .all(|e| !matches!(e, Effect::Write(_) | Effect::PrepareActivation(_)))
            );
        }
    }
}

#[test]
fn rejected_identity_is_display_only_and_reset_discards_it() {
    let mut c = ConnectionApplicationCoordinator::default();
    c.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut c, motor_payload(7), 20);
    let mut info = selectable_info_payload();
    info[27] = 1;
    info[28..32].copy_from_slice(b"beta");
    let effects = receive(&mut c, info.clone(), 30);
    assert_eq!(c.snapshot().phase, Phase::Failed);
    assert_eq!(
        c.snapshot().reported_refloat_version.as_deref(),
        Some("1.3.1-beta")
    );
    assert_eq!(
        c.snapshot().discovery.issue.unwrap().code,
        IssueCode::UnsupportedRefloat
    );
    assert!(c.snapshot().awaiting_activation.is_none());
    assert!(
        effects
            .iter()
            .all(|e| !matches!(e, Effect::Write(_) | Effect::PrepareActivation(_)))
    );
    let previous_generation = c.snapshot().generation;
    c.apply(Command::Begin(begin_facts(40))).unwrap();
    assert!(c.snapshot().reported_refloat_version.is_none());
    c.apply(Command::Receive {
        generation: previous_generation,
        payload: info,
        now_ms: 41,
        observed_at_epoch_ms: 0,
    })
    .unwrap();
    assert!(c.snapshot().reported_refloat_version.is_none());
    receive(&mut c, motor_payload(7), 50);
    receive(&mut c, selectable_info_payload(), 60);
    assert_eq!(c.snapshot().phase, Phase::AwaitingActivation);
}

#[test]
fn absent_and_unreadable_info_do_not_claim_an_unsupported_version() {
    for malformed in [false, true] {
        let mut c = ConnectionApplicationCoordinator::default();
        c.apply(Command::Begin(begin_facts(10))).unwrap();
        let pending = receive(&mut c, motor_payload(7), 20);
        if malformed {
            receive(&mut c, vec![36, 101, 0, 2], 30);
        } else {
            c.apply(Command::DeadlineFired {
                generation: c.snapshot().generation,
                token: deadline(&pending),
                now_ms: 10_000,
            })
            .unwrap();
        }
        assert_eq!(c.snapshot().phase, Phase::Failed);
        assert!(c.snapshot().reported_refloat_version.is_none());
        assert_eq!(
            c.snapshot().discovery.issue.unwrap().code,
            if malformed {
                IssueCode::RefloatInfoInvalid
            } else {
                IssueCode::IdentificationTimeout
            }
        );
    }
}
