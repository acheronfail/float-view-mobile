use std::fs;
use std::path::{Path, PathBuf};

use float_view_refloat_configuration_codec::{decode_qt_compressed, decode_values};
use float_view_refloat_configuration_service::{
    Availability, BeginFacts, CATALOG, CacheEffect, ConfigurationService, FirmwareFacts,
    Generation, Operation, PendingOperation, RequestContext, RequestEffect, Response, Route,
    ServiceError, Transition, ValueSource, ValuesCommand, resolve_catalog,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/refloat-configuration")
}

fn bytes(relative: &str) -> Vec<u8> {
    fs::read(root().join(relative)).expect("committed fixture")
}

fn facts(
    version: &str,
    route: Route,
    generation: Generation,
    cached_schema: Option<Vec<u8>>,
) -> BeginFacts {
    BeginFacts {
        route,
        generation,
        firmware: FirmwareFacts {
            major: 6,
            minor: 6,
            hardware_name: "THOR300".into(),
            custom_configuration_count: 1,
            hardware_configuration_crc: Some(123),
        },
        reported_version: version.into(),
        writable: true,
        flywheel: false,
        cached_schema,
    }
}

fn load_current(version: &str, route: Route, generation: Generation) -> ConfigurationService {
    let mut service = ConfigurationService::new();
    let schema_bytes = bytes(&format!("schemas/{version}.qcompress"));
    let first = service.begin(facts(version, route, generation, Some(schema_bytes)));
    let operation = match first.request {
        Some(RequestEffect::Values {
            route: effect_route,
            defaults: false,
            operation,
        }) => {
            assert_eq!(effect_route, route);
            operation
        }
        other => panic!("expected cache-backed current read, got {other:?}"),
    };
    service
        .accept(
            Response::Values {
                command: ValuesCommand::Current,
                index: 0,
                bytes: bytes(&format!("values/{version}-current.bin")),
            },
            RequestContext::Values {
                route,
                index: 0,
                defaults: false,
                operation,
            },
            generation,
        )
        .expect("current values");
    service
}

#[test]
fn every_catalogue_release_and_legacy_patch_is_admitted_from_exact_cache() {
    assert_eq!(CATALOG.len(), 9);
    for entry in CATALOG {
        let compressed = bytes(&format!("schemas/{}.qcompress", entry.version));
        let schema = decode_qt_compressed(&compressed).expect("schema");
        assert_eq!(resolve_catalog(entry.version, &schema), Some(*entry));
        let reported = if entry.version.starts_with("1.0.") {
            "1.0"
        } else {
            entry.version
        };
        assert_eq!(resolve_catalog(reported, &schema), Some(*entry));

        let mut service = ConfigurationService::new();
        let transition = service.begin(facts(reported, Route(41), Generation(9), Some(compressed)));
        assert!(matches!(
            transition.request,
            Some(RequestEffect::Values {
                defaults: false,
                ..
            })
        ));
        assert_eq!(
            service
                .snapshot()
                .identity
                .expect("identity")
                .package_version,
            entry.version
        );
        assert_eq!(service.snapshot().availability, Availability::Ready);
    }
}

#[test]
fn cache_miss_corruption_and_full_paging_emit_atomic_cache_decisions() {
    let compressed = bytes("schemas/1.2.3.qcompress");
    let mut corrupt = ConfigurationService::new();
    let transition = corrupt.begin(facts(
        "1.2.3",
        Route(7),
        Generation(1),
        Some(b"corrupt".to_vec()),
    ));
    assert!(matches!(transition.cache, Some(CacheEffect::Remove { .. })));
    assert_eq!(
        transition.request,
        Some(RequestEffect::SchemaChunk {
            route: Route(7),
            length: 400,
            offset: 0,
            operation: Operation(1)
        })
    );

    let mut service = ConfigurationService::new();
    let mut transition = service.begin(facts("1.2.3", Route(42), Generation(2), None));
    loop {
        let RequestEffect::SchemaChunk {
            route,
            length,
            offset,
            operation,
        } = transition.request.expect("schema request")
        else {
            panic!("expected schema page");
        };
        let end = (offset + length).min(compressed.len());
        transition = service
            .accept(
                Response::SchemaChunk {
                    index: 0,
                    total: compressed.len(),
                    offset,
                    bytes: compressed[offset..end].to_vec(),
                },
                RequestContext::Schema {
                    route,
                    index: 0,
                    offset,
                    operation,
                },
                Generation(2),
            )
            .expect("schema page");
        if end == compressed.len() {
            assert!(
                matches!(transition.cache, Some(CacheEffect::Store { ref bytes, .. }) if bytes == &compressed)
            );
            assert!(matches!(
                transition.request,
                Some(RequestEffect::Values {
                    route: Route(42),
                    defaults: false,
                    ..
                })
            ));
            break;
        }
        assert!(transition.cache.is_none());
    }
}

#[test]
fn selected_native_read_default_write_verify_and_conflict_transitions_match() {
    let route = Route(42);
    let generation = Generation(9);
    let mut service = load_current("1.2.3", route, generation);
    let first = service.snapshot();
    assert_eq!(first.current.as_ref().expect("current").revision, 1);
    assert_eq!(
        first.current.as_ref().expect("current").source,
        ValueSource::Current
    );
    assert!(first.writable);

    let read_default = service.request_values(true).expect("default request");
    let operation = match read_default.request.expect("request") {
        RequestEffect::Values {
            defaults: true,
            operation,
            ..
        } => operation,
        other => panic!("unexpected {other:?}"),
    };
    service
        .accept(
            Response::Values {
                command: ValuesCommand::Default,
                index: 0,
                bytes: bytes("values/1.2.3-default.bin"),
            },
            RequestContext::Values {
                route,
                index: 0,
                defaults: true,
                operation,
            },
            generation,
        )
        .expect("defaults");
    assert_eq!(service.snapshot().defaults.expect("defaults").revision, 2);

    let schema = service.loaded_schema().expect("schema");
    let draft = decode_values(&bytes("values/1.2.3-write.bin"), schema).expect("draft");
    let identity = service.snapshot().identity.expect("identity");
    let base = service.snapshot().current.expect("current").revision;
    assert_eq!(
        service.request_write(
            "wrong",
            identity.fingerprint,
            identity.wire_signature,
            base,
            &draft
        ),
        Err(ServiceError::InvalidState)
    );
    let write = service
        .request_write(
            identity.package_version,
            identity.fingerprint,
            identity.wire_signature,
            base,
            &draft,
        )
        .expect("write");
    let operation = match write.request.expect("request") {
        RequestEffect::Write {
            payload, operation, ..
        } => {
            assert_eq!(payload, bytes("values/1.2.3-write.bin"));
            operation
        }
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(
        service
            .accept(
                Response::WriteAcknowledgement,
                RequestContext::Write {
                    route,
                    index: 0,
                    operation
                },
                generation,
            )
            .expect("ack"),
        Transition {
            request: Some(RequestEffect::Verification { route, operation }),
            cache: None
        }
    );
    service
        .accept(
            Response::Values {
                command: ValuesCommand::Current,
                index: 0,
                bytes: bytes("values/1.2.3-readback.bin"),
            },
            RequestContext::Verification {
                route,
                index: 0,
                operation,
            },
            generation,
        )
        .expect("normalized verification");
    assert_eq!(service.snapshot().current.expect("verified").revision, 3);

    let current = service.snapshot().current.expect("current");
    let write = service
        .request_write(
            identity.package_version,
            identity.fingerprint,
            identity.wire_signature,
            current.revision,
            &draft,
        )
        .expect("second write");
    let RequestEffect::Write { operation, .. } = write.request.expect("request") else {
        panic!("write")
    };
    service
        .accept(
            Response::WriteAcknowledgement,
            RequestContext::Write {
                route,
                index: 0,
                operation,
            },
            generation,
        )
        .expect("ack");
    assert_eq!(
        service.accept(
            Response::Values {
                command: ValuesCommand::Current,
                index: 0,
                bytes: bytes("values/1.2.3-current.bin")
            },
            RequestContext::Verification {
                route,
                index: 0,
                operation
            },
            generation,
        ),
        Err(ServiceError::Conflict)
    );
    assert_eq!(
        service
            .snapshot()
            .current
            .expect("authoritative mismatch")
            .revision,
        4
    );
}

#[test]
fn flywheel_fail_abort_and_reset_match_native_recovery() {
    let route = Route(0);
    let generation = Generation(3);
    let mut service = load_current("1.0.0", route, generation);
    assert_eq!(
        service.set_flywheel(true),
        Transition {
            request: None,
            cache: None
        }
    );
    assert!(!service.snapshot().writable);
    assert!(service.snapshot().current.is_none());
    assert_eq!(
        service.request_values(false),
        Err(ServiceError::InvalidState)
    );
    let transition = service.set_flywheel(false);
    let operation = match transition.request.expect("refresh") {
        RequestEffect::Values {
            defaults: false,
            operation,
            ..
        } => operation,
        other => panic!("unexpected {other:?}"),
    };
    service.request_failed(
        RequestContext::Values {
            route,
            index: 0,
            defaults: false,
            operation,
        },
        Generation(99),
    );
    assert_eq!(
        service.snapshot().pending,
        Some(PendingOperation::ReadCurrent)
    );
    service.abort_active_operation(generation);
    assert_eq!(service.snapshot().availability, Availability::Ready);
    assert!(service.snapshot().pending.is_none());

    let pending = service.request_values(true).expect("default");
    assert!(pending.request.is_some());
    service.reset();
    assert_eq!(service.snapshot().availability, Availability::Unavailable);
    assert!(service.snapshot().identity.is_none());
    assert!(service.snapshot().defaults.is_none());
}
