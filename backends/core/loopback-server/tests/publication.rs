use float_view_backend_service::os::{Envelope, Fact};
use float_view_loopback_server::protocol::Platform;
mod support;
#[test]
fn process_publication_is_monotonic_conflated_and_survives_listener_generations() {
    let server = support::server(Platform::Macos, b"html").unwrap();
    let backend = &server.backend;
    let generation = backend.start().unwrap();
    backend.sync_service().unwrap();
    let initial = backend.snapshot().unwrap()["revision"].as_u64().unwrap();
    let slow = backend.snapshots.subscribe();
    let other = backend.snapshots.subscribe();
    for sequence in 1..=100 {
        let mut service = backend.service.lock().unwrap();
        let envelope = Envelope {
            lifetime: service.lifetime(),
            sequence,
            monotonic_ms: sequence,
            epoch_ms: 1700000000000 + sequence as i64,
            fact: Fact::Foreground {
                foreground: sequence % 2 == 0,
            },
        };
        service.apply_os(envelope).unwrap();
        drop(service);
        backend.sync_service().unwrap();
    }
    assert_eq!(slow.borrow().as_ref().unwrap()["revision"], initial + 100);
    assert_eq!(other.borrow().as_ref().unwrap()["revision"], initial + 100);
    backend.sync_service().unwrap();
    assert_eq!(backend.snapshot().unwrap()["revision"], initial + 100);
    // Late private file-UI cleanup must not race public state reads or invent
    // a product revision simply because an export requester disconnected.
    {
        let mut service = backend.service.lock().unwrap();
        let lifetime = service.lifetime();
        for (sequence, fact) in [
            (
                101,
                Fact::PresentationStarted {
                    id: 99,
                    accepted: false,
                },
            ),
            (102, Fact::PresentationFinished { id: 99 }),
        ] {
            service
                .apply_os(Envelope {
                    lifetime,
                    sequence,
                    monotonic_ms: sequence,
                    epoch_ms: 1700000000000 + sequence as i64,
                    fact,
                })
                .unwrap();
        }
    }
    backend.sync_service().unwrap();
    assert_eq!(backend.snapshot().unwrap()["revision"], initial + 100);
    server.stop().unwrap();
    assert!(backend.start().unwrap() > generation);
    backend.sync_service().unwrap();
    assert_eq!(backend.snapshot().unwrap()["revision"], initial + 100);
}

#[test]
fn periodic_os_observations_do_not_invent_public_revisions() {
    use float_view_backend_service::os::{LocationAuthorization, Permission, Radio};
    let server = support::server(Platform::Android, b"html").unwrap();
    let backend = &server.backend;
    backend.start().unwrap();
    let bluetooth = || Fact::Bluetooth {
        radio: Radio::PoweredOn,
        scan: Permission::Granted,
        connect: Permission::NotRequired,
    };
    let location = || Fact::LocationCapability {
        authorization: LocationAuthorization::Authorized,
        background: true,
    };
    let facts = [
        bluetooth(),
        location(),
        bluetooth(),
        location(),
        Fact::Storage {
            available_bytes: 1_000_000_000,
        },
        Fact::Storage {
            available_bytes: 2_000_000_000,
        },
        Fact::Foreground { foreground: true },
        Fact::Timer { id: u64::MAX },
    ];
    let mut expected = None;
    for (index, fact) in facts.into_iter().enumerate() {
        let mut service = backend.service.lock().unwrap();
        let lifetime = service.lifetime();
        let sequence = index as u64 + 1;
        service
            .apply_os(Envelope {
                lifetime,
                sequence,
                monotonic_ms: sequence,
                epoch_ms: 1700000000000 + sequence as i64,
                fact,
            })
            .unwrap();
        drop(service);
        backend.sync_service().unwrap();
        let snapshot = backend.snapshot().unwrap();
        if index == 1 {
            expected = Some(snapshot.clone());
        }
        if index > 1 {
            assert_eq!(Some(snapshot), expected);
        }
    }
}
