use float_view_loopback_server::{backend::Backend, protocol::Publication};
use serde_json::json;
mod support;
use support::facts;
#[test]
fn process_publication_is_monotonic_conflated_and_survives_listener_generations() {
    let backend = Backend::default();
    let generation = backend.start().unwrap();
    let publish = |sequence, catalog_changed, facts| Publication {
        generation,
        sequence,
        catalog_changed,
        facts,
    };
    backend.publish(publish(1, false, facts())).unwrap();
    let slow = backend.snapshots.subscribe();
    let other = backend.snapshots.subscribe();
    backend.publish(publish(2, false, facts())).unwrap();
    assert_eq!(backend.snapshot().unwrap()["revision"], 1);
    assert!(backend.publish(publish(2, false, facts())).is_err());
    for sequence in 3..=100 {
        let mut f = facts();
        f.settings.revision = sequence;
        backend.publish(publish(sequence, false, f)).unwrap();
    }
    assert_eq!(slow.borrow().as_ref().unwrap()["revision"], 99);
    assert_eq!(other.borrow().as_ref().unwrap()["revision"], 99);
    assert_eq!(backend.catalog_revision().unwrap(), 0);
    backend.publish(publish(101, true, facts())).unwrap();
    assert_eq!(backend.catalog_revision().unwrap(), 1);
    backend.stop();
    let next = backend.start().unwrap();
    assert!(next > generation);
    assert_eq!(backend.snapshot().unwrap()["revision"], 100);
    assert!(backend.publish(publish(102, false, facts())).is_err());
    backend
        .publish(Publication {
            generation: next,
            sequence: 102,
            catalog_changed: false,
            facts: facts(),
        })
        .unwrap();
    assert_eq!(backend.snapshot().unwrap()["revision"], 100);
}
#[test]
fn malformed_guest_facts_cannot_publish_a_saved_session_even_without_a_sample() {
    let mut f = serde_json::to_value(facts()).unwrap();
    f["active"] = json!({"deviceId":"guest","name":null,"state":"ready","deviceKind":"board","trackingMode":"guest","savedDevice":null,"activeSession":{"sessionId":"saved","deviceHistoryId":"history","displayName":"Board","startedAt":"2026-01-01T00:00:00Z","endedAt":null,"state":"active","integrity":"complete","sampleCount":0,"eventCount":0,"ridingElapsedMs":0,"totalElapsedMs":0,"distanceM":null,"maxSpeedMps":null,"maxDutyCycle":null},"latestSample":null,"latestLocation":null,"currentLimits":null,"controllerRoute":null,"controllerCandidates":[],"connectionIssue":null,"bms":{"status":"not_found","connection":null,"candidates":[],"latestSnapshot":null},"refloatControls":null,"controllerActions":{"shutdownAvailable":false,"pendingAction":null}});
    let backend = Backend::default();
    assert!(
        backend
            .publish(Publication {
                generation: 0,
                sequence: 1,
                catalog_changed: false,
                facts: serde_json::from_value(f).unwrap()
            })
            .is_err()
    );
    assert!(backend.snapshot().is_err());
}
