use std::fs;
use std::path::{Path, PathBuf};

use float_view_refloat_configuration_codec::{
    ConfigurationValue, decode_qt_compressed, decode_values,
};
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

// Expected shared test evidence, independent of runtime release admission.
fn fixture_version(version: &str) -> &str {
    match version {
        "1.3.1-beta3" | "1.3.1" | "1.3.2" => "1.3.1-beta2",
        other => other,
    }
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
    let fixture = fixture_version(version);
    let schema_bytes = bytes(&format!("schemas/{fixture}.qcompress"));
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
                bytes: bytes(&format!("values/{fixture}-current.bin")),
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
    assert_eq!(CATALOG.len(), 13);
    for entry in CATALOG {
        let compressed = bytes(&format!(
            "schemas/{}.qcompress",
            fixture_version(entry.version)
        ));
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
fn rounded_write_is_verified_against_wire_values_instead_of_submitted_precision() {
    for version in ["1.3.1", "1.3.2"] {
        let route = Route(42);
        let generation = Generation(9);
        let mut service = load_current(version, route, generation);
        let identity = service.snapshot().identity.unwrap();
        let current = service.snapshot().current.unwrap();
        let mut draft = current.values.clone();
        for (name, value) in &mut draft {
            if name == "kp2" {
                *value = ConfigurationValue::Decimal(1.091);
            } else if name == "meta.is_default" {
                *value = ConfigurationValue::Boolean(false);
            }
        }
        let write = service
            .request_write(
                &identity.package_version,
                identity.fingerprint,
                identity.wire_signature,
                current.revision,
                &draft,
            )
            .unwrap();
        let Some(RequestEffect::Write {
            payload, operation, ..
        }) = write.request
        else {
            panic!("expected write");
        };
        let canonical = decode_values(&payload, service.loaded_schema().unwrap()).unwrap();
        assert_eq!(
            canonical.iter().find(|(name, _)| name == "kp2").unwrap().1,
            ConfigurationValue::Decimal(1.09)
        );
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
            .unwrap();
        service
            .accept(
                Response::Values {
                    command: ValuesCommand::Current,
                    index: 0,
                    bytes: payload,
                },
                RequestContext::Verification {
                    route,
                    index: 0,
                    operation,
                },
                generation,
            )
            .unwrap();
        assert_eq!(service.snapshot().current.unwrap().values, canonical);
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
    let mut map_ordered_draft = draft.clone();
    map_ordered_draft.sort_by(|left, right| left.0.cmp(&right.0));
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
            &identity.package_version,
            identity.fingerprint,
            identity.wire_signature,
            base,
            &map_ordered_draft,
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
            &identity.package_version,
            identity.fingerprint,
            identity.wire_signature,
            current.revision,
            &map_ordered_draft,
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

#[test]
fn unaudited_schema_retains_only_its_observed_identity_until_reset() {
    let route = Route(42);
    let generation = Generation(9);
    let source = bytes("schemas/1.2.3.qcompress");
    let expected = decode_qt_compressed(&source).unwrap();
    let mut service = ConfigurationService::new();
    let mut transition = service.begin(facts("1.3.0", route, generation, None));
    loop {
        let RequestEffect::SchemaChunk {
            length,
            offset,
            operation,
            ..
        } = transition.request.unwrap()
        else {
            panic!("schema request")
        };
        let end = (offset + length).min(source.len());
        match service.accept(
            Response::SchemaChunk {
                index: 0,
                total: source.len(),
                offset,
                bytes: source[offset..end].to_vec(),
            },
            RequestContext::Schema {
                route,
                index: 0,
                offset,
                operation,
            },
            generation,
        ) {
            Ok(next) => transition = next,
            Err(ServiceError::UnauditedSchema) => break,
            other => panic!("unexpected {other:?}"),
        }
    }
    let snapshot = service.snapshot();
    let observed = snapshot.observed_schema.unwrap();
    assert_eq!(observed.fingerprint, expected.fingerprint);
    assert_eq!(observed.wire_signature, expected.wire_signature);
    assert!(snapshot.identity.is_none());
    assert!(!snapshot.writable);
    assert!(snapshot.current.is_none());
    service.reset();
    assert!(service.snapshot().observed_schema.is_none());
}

#[test]
fn stable_minor_schema_admission_retains_observed_patch_and_rejects_other_families() {
    for (version, fixture) in [
        ("1.0.255", "1.0.1"),
        ("1.1.255", "1.1.2"),
        ("1.2.255", "1.2.3"),
        ("1.3.2", "1.3.1-beta2"),
    ] {
        let compressed = bytes(&format!("schemas/{fixture}.qcompress"));
        let schema = decode_qt_compressed(&compressed).unwrap();
        assert!(resolve_catalog(version, &schema).is_some());
        for invalid in ["1.4.0", "2.0.0", "1.3.2-beta4", "1.03.2"] {
            assert!(resolve_catalog(invalid, &schema).is_none());
        }
        let mut altered = schema.clone();
        altered.fingerprint = "a".repeat(64);
        assert!(resolve_catalog(version, &altered).is_none());
        altered = schema.clone();
        altered.wire_signature ^= 1;
        assert!(resolve_catalog(version, &altered).is_none());
        for cached in [false, true] {
            let mut service = ConfigurationService::new();
            let mut transition = service.begin(facts(
                version,
                Route(42),
                Generation(2),
                cached.then(|| compressed.clone()),
            ));
            if !cached {
                loop {
                    let Some(RequestEffect::SchemaChunk {
                        route,
                        length,
                        offset,
                        operation,
                    }) = transition.request
                    else {
                        panic!("schema page")
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
                        .unwrap();
                    if end == compressed.len() {
                        break;
                    }
                }
            }
            let snapshot = service.snapshot();
            assert_eq!(snapshot.availability, Availability::Ready);
            assert_eq!(snapshot.identity.unwrap().package_version, version);
            if let Some(CacheEffect::Store { identity, .. }) = transition.cache {
                assert_eq!(identity.package_version, version);
            }
        }
    }
    let schema = decode_qt_compressed(&bytes("schemas/1.2.3.qcompress")).unwrap();
    assert!(resolve_catalog("1.3.2", &schema).is_none());
}
