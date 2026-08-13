use float_view_refloat_configuration_codec::MAX_COMPRESSED_BYTES;
use float_view_refloat_configuration_service::{
    Availability, BeginFacts, ConfigurationService, FirmwareFacts, Generation, Operation,
    RequestContext, RequestEffect, Response, Route, ServiceError, ValuesCommand,
};

fn begin(service: &mut ConfigurationService) -> (Route, Generation, Operation) {
    let route = Route(8);
    let generation = Generation(4);
    let transition = service.begin(BeginFacts {
        route,
        generation,
        firmware: FirmwareFacts {
            major: 6,
            minor: 6,
            hardware_name: "fixture".into(),
            custom_configuration_count: 1,
            hardware_configuration_crc: None,
        },
        reported_version: "1.2.3".into(),
        writable: true,
        flywheel: false,
        cached_schema: None,
    });
    let Some(RequestEffect::SchemaChunk { operation, .. }) = transition.request else {
        panic!("schema")
    };
    (route, generation, operation)
}

#[test]
fn stale_generations_routes_operations_indexes_offsets_and_contexts_fail_closed() {
    let mutations = [
        (Route(8), Generation(99), Operation(1), 0, 0),
        (Route(9), Generation(4), Operation(1), 0, 0),
        (Route(8), Generation(4), Operation(2), 0, 0),
        (Route(8), Generation(4), Operation(1), 1, 0),
        (Route(8), Generation(4), Operation(1), 0, 1),
    ];
    for (route, generation, operation, index, offset) in mutations {
        let mut service = ConfigurationService::new();
        begin(&mut service);
        assert_eq!(
            service.accept(
                Response::SchemaChunk {
                    index,
                    total: 2,
                    offset,
                    bytes: vec![1]
                },
                RequestContext::Schema {
                    route,
                    index,
                    offset,
                    operation
                },
                generation,
            ),
            Err(ServiceError::StaleResponse)
        );
        assert_eq!(service.snapshot().availability, Availability::Loading);
    }

    let mut service = ConfigurationService::new();
    let (route, generation, operation) = begin(&mut service);
    assert_eq!(
        service.accept(
            Response::WriteAcknowledgement,
            RequestContext::Write {
                route,
                index: 0,
                operation
            },
            generation,
        ),
        Err(ServiceError::StaleResponse)
    );
    assert_eq!(
        service.accept(
            Response::Values {
                command: ValuesCommand::Current,
                index: 0,
                bytes: vec![]
            },
            RequestContext::Values {
                route,
                index: 0,
                defaults: false,
                operation
            },
            generation,
        ),
        Err(ServiceError::StaleResponse)
    );
}

#[test]
fn chunk_bounds_total_changes_empty_pages_and_overrun_are_deterministic() {
    let mut service = ConfigurationService::new();
    let (route, generation, operation) = begin(&mut service);
    let context = RequestContext::Schema {
        route,
        index: 0,
        offset: 0,
        operation,
    };
    assert_eq!(
        service.accept(
            Response::SchemaChunk {
                index: 0,
                total: MAX_COMPRESSED_BYTES + 1,
                offset: 0,
                bytes: vec![1]
            },
            context,
            generation,
        ),
        Err(ServiceError::StaleResponse)
    );

    let mut empty = ConfigurationService::new();
    let (route, generation, operation) = begin(&mut empty);
    assert_eq!(
        empty.accept(
            Response::SchemaChunk {
                index: 0,
                total: 1,
                offset: 0,
                bytes: vec![]
            },
            RequestContext::Schema {
                route,
                index: 0,
                offset: 0,
                operation
            },
            generation,
        ),
        Err(ServiceError::StaleResponse)
    );

    let mut overrun = ConfigurationService::new();
    let (route, generation, operation) = begin(&mut overrun);
    assert_eq!(
        overrun.accept(
            Response::SchemaChunk {
                index: 0,
                total: 1,
                offset: 0,
                bytes: vec![1, 2]
            },
            RequestContext::Schema {
                route,
                index: 0,
                offset: 0,
                operation
            },
            generation,
        ),
        Err(ServiceError::MalformedResponse)
    );

    let mut changed = ConfigurationService::new();
    let (route, generation, operation) = begin(&mut changed);
    changed
        .accept(
            Response::SchemaChunk {
                index: 0,
                total: 3,
                offset: 0,
                bytes: vec![1],
            },
            RequestContext::Schema {
                route,
                index: 0,
                offset: 0,
                operation,
            },
            generation,
        )
        .expect("first page");
    assert_eq!(
        changed.accept(
            Response::SchemaChunk {
                index: 0,
                total: 4,
                offset: 1,
                bytes: vec![2]
            },
            RequestContext::Schema {
                route,
                index: 0,
                offset: 1,
                operation
            },
            generation,
        ),
        Err(ServiceError::StaleResponse)
    );
}

#[test]
fn diagnostics_are_closed_and_never_include_adversarial_payloads() {
    let secret = b"private-telemetry-and-token".to_vec();
    let mut service = ConfigurationService::new();
    let (route, generation, operation) = begin(&mut service);
    let error = service
        .accept(
            Response::SchemaChunk {
                index: 0,
                total: 1,
                offset: 0,
                bytes: secret,
            },
            RequestContext::Schema {
                route,
                index: 0,
                offset: 0,
                operation,
            },
            generation,
        )
        .expect_err("overrun");
    let diagnostic = format!("{error:?}");
    assert_eq!(diagnostic, "MalformedResponse");
    assert!(!diagnostic.contains("private"));
}
