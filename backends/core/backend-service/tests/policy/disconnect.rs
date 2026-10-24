use super::*;

#[allow(dead_code)]
mod samples {
    use float_view_application_runtime::telemetry as float_view_refloat_telemetry_service;
    include!("../../../session-sqlite/tests/support/mod.rs");
}

fn recorded_ride() -> (Test, String) {
    let mut t = Test::new();
    t.activate(true);
    let id = t.service.policy.ride.as_ref().unwrap().session_id.clone();
    let mut sample = samples::sample(1, 0);
    sample.observed_at_epoch_ms = t.service.policy.exchange.epoch;
    t.service.runtime.record_sample(&sample, None).unwrap();
    let marker = t.service.runtime.snapshot().session_marker.unwrap();
    t.service.store.apply_session_marker(&id, &marker).unwrap();
    t.service.changed().unwrap();
    t.drain();
    (t, id)
}

fn assert_disconnected(t: &mut Test, generation: u64, revision: u64) {
    assert!(t.service.policy.active.is_none());
    assert!(t.service.policy.capture.is_none());
    assert!(t.service.policy.latest_location.is_none());
    assert!(t.service.published_snapshot().active.is_none());
    assert!(t.service.service_revision() > revision);
    let runtime = t.service.runtime.snapshot();
    assert!(!runtime.session_attached);
    assert!(runtime.session_marker.is_none());
    assert_eq!(runtime.application.phase, c::Phase::Disconnected);
    let effects = t.service.drain_os().effects;
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::Disconnect { generation: g } if *g == generation))
    );
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::CancelConnectionTimers))
    );
    assert!(effects.iter().any(|e| matches!(e, Effect::StopLocation)));
    assert!(effects.iter().any(|e| matches!(
        e,
        Effect::HostState {
            connected: false,
            recording: false,
            ..
        }
    )));
    assert!(!effects.iter().any(|e| matches!(
        e,
        Effect::Connect { .. }
            | Effect::Write { .. }
            | Effect::EnableNotifications { .. }
            | Effect::StartLocation { .. }
            | Effect::ArmTimer {
                connection: true,
                ..
            }
    )));
}

#[test]
fn explicit_and_native_disconnect_continue_after_each_storage_failure() {
    for native in [false, true] {
        for stage in ["reason", "marker", "pause", "draft", "catalog_read"] {
            let (mut t, id) = recorded_ride();
            let generation = t.service.policy.active.as_ref().unwrap().generation;
            let revision = t.service.service_revision();
            let recording =
                rusqlite::Connection::open(t.service.store.session_path(&id).unwrap()).unwrap();
            let catalog = rusqlite::Connection::open(t.root.join("catalog.sqlite")).unwrap();
            match stage {
                "reason" => recording.execute_batch("CREATE TRIGGER fail_cleanup BEFORE INSERT ON session_event WHEN NEW.kind='recordingInterrupted' BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap(),
                "marker" => catalog.execute_batch("CREATE TRIGGER fail_cleanup BEFORE UPDATE ON session_catalog BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap(),
                "pause" => catalog.execute_batch("CREATE TRIGGER fail_cleanup BEFORE UPDATE ON ride_checkpoint BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap(),
                "draft" => {
                    let key = "07".repeat(12);
                    t.service.policy.active.as_mut().unwrap().refloat_draft_key = Some(key.clone());
                    catalog.execute("INSERT OR REPLACE INTO refloat_editor_draft(controller_key,json,updated_at_ms) VALUES (?,'{}',0)", [key]).unwrap();
                    catalog.execute_batch("CREATE TRIGGER fail_cleanup BEFORE DELETE ON refloat_editor_draft BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
                }
                "catalog_read" => catalog.execute_batch("ALTER TABLE ride_checkpoint RENAME TO unavailable_ride_checkpoint").unwrap(),
                _ => unreachable!(),
            }
            let result = if native {
                let now = t.service.policy.exchange.now + 1;
                t.fact(Fact::StopRequested {}, now)
            } else {
                t.service
                    .dispatch(1, crate::operations::Operation::Disconnect)
                    .map(|_| ())
            };
            assert!(
                result.is_err(),
                "{stage}, native={native}: report cleanup failure"
            );
            assert_disconnected(&mut t, generation, revision);
            assert_eq!(
                recording
                    .query_row("SELECT COUNT(*) FROM sample", [], |r| r.get::<_, u64>(0))
                    .unwrap(),
                1
            );
            assert!(
                t.service.published_snapshot().paused_ride.is_some(),
                "{stage}: keep logical resume available"
            );
            // Catalog cleanup must still run when the earlier reason write failed.
            if stage == "reason" {
                assert!(
                    t.service
                        .store
                        .ride_checkpoint()
                        .unwrap()
                        .unwrap()
                        .paused_at_ms
                        .is_some()
                );
            }
            match stage {
                "reason" => recording
                    .execute_batch("DROP TRIGGER fail_cleanup")
                    .unwrap(),
                "catalog_read" => catalog
                    .execute_batch(
                        "ALTER TABLE unavailable_ride_checkpoint RENAME TO ride_checkpoint",
                    )
                    .unwrap(),
                _ => catalog.execute_batch("DROP TRIGGER fail_cleanup").unwrap(),
            }
            // Old callbacks must not restart recording after the failed cleanup.
            let now = t.service.policy.exchange.now + 1;
            t.fact(
                Fact::Bytes {
                    generation,
                    bytes: vec![2, 0, 3],
                },
                now,
            )
            .unwrap();
            assert!(!t.service.runtime.snapshot().session_attached);
            assert!(t.service.runtime.snapshot().session_marker.is_none());
            t.activate(true);
            assert_eq!(t.service.policy.ride.as_ref().unwrap().session_id, id);
            let mut resumed = samples::sample(2, 10_000);
            resumed.observed_at_epoch_ms = t.service.policy.exchange.epoch + 10_000;
            t.service.runtime.record_sample(&resumed, None).unwrap();
            let mut reader =
                session::SessionReader::open(t.service.store.session_path(&id).unwrap()).unwrap();
            let marker = reader.marker().unwrap();
            let breaks = reader.recording_breaks((&marker).into()).unwrap();
            assert_eq!(
                breaks[0].reason,
                if stage == "reason" {
                    session::InterruptionReason::Unknown
                } else {
                    session::InterruptionReason::Pause
                }
            );
        }
    }
}

#[test]
fn failed_close_catalog_and_multiple_failures_do_not_block_disconnect() {
    let (mut t, id) = recorded_ride();
    let generation = t.service.policy.active.as_ref().unwrap().generation;
    let revision = t.service.service_revision();
    let catalog = rusqlite::Connection::open(t.root.join("catalog.sqlite")).unwrap();
    catalog.execute_batch("CREATE TRIGGER fail_cleanup BEFORE UPDATE ON session_catalog BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    let recording = rusqlite::Connection::open(t.service.store.session_path(&id).unwrap()).unwrap();
    recording.execute_batch("CREATE TRIGGER fail_reason BEFORE INSERT ON session_event WHEN NEW.kind='recordingInterrupted' BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    assert!(t.service.disconnect("end_ride").is_err());
    assert_disconnected(&mut t, generation, revision);
    assert!(t.service.store.session_path(&id).unwrap().exists());
    assert_eq!(
        recording
            .query_row("SELECT COUNT(*) FROM sample", [], |r| r.get::<_, u64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn disconnect_bypasses_expiry_preflight_and_uses_reserved_cleanup_slots() {
    for native in [false, true] {
        let (mut t, _id) = recorded_ride();
        let generation = t.service.policy.active.as_ref().unwrap().generation;
        let revision = t.service.service_revision();
        // Expiry would attempt a failing catalog close before dispatching stop.
        let epoch = t.service.policy.exchange.epoch;
        let ride = t.service.policy.ride.as_mut().unwrap();
        ride.paused_at_ms = Some(epoch - 60_000);
        ride.resume_until_ms = Some(epoch - 1);
        let catalog = rusqlite::Connection::open(t.root.join("catalog.sqlite")).unwrap();
        catalog.execute_batch("CREATE TRIGGER fail_cleanup BEFORE UPDATE ON session_catalog BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
        while t
            .service
            .policy
            .exchange
            .push(Effect::RefreshSpeech)
            .is_ok()
        {}
        assert!(t.service.policy.exchange.admit().is_err());
        if native {
            let now = t.service.policy.exchange.now + 1;
            assert!(t.fact(Fact::StopRequested {}, now).is_err());
        } else {
            assert!(
                t.service
                    .dispatch(1, crate::operations::Operation::Disconnect)
                    .is_err()
            );
        }
        assert_disconnected(&mut t, generation, revision);
    }
}
