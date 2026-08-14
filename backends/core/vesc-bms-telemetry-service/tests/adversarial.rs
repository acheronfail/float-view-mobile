mod support;

use float_view_auxiliary_node_discovery::{Route, Status};
use float_view_vesc_bms_telemetry_service::{
    Command, Diagnostic, Effect, FAILURE_THRESHOLD, POLL_INTERVAL_MS, RequestKind, RequestOutcome,
    SchedulerGeneration, ServiceError, VescBmsTelemetryService, request_facts,
};
use float_view_vesc_request_scheduler::{Priority, RequestContext, Route as SchedulerRoute};

use support::{
    begin, complete, controller, direct_configuration, fire_timer, receive_valid, timer,
    values_payload,
};

#[test]
fn request_facts_are_exact_read_only_and_route_correct() {
    let generation = SchedulerGeneration(9);
    let scan = request_facts(RequestKind::CanScan, None, generation).unwrap();
    assert_eq!(scan.payload, [62]);
    assert_eq!(scan.context, RequestContext::AuxiliaryCanPing);
    assert_eq!(scan.timeout_ms, 10_000);

    let firmware = request_facts(RequestKind::Firmware, Some(Route::Can(253)), generation).unwrap();
    assert_eq!(firmware.payload, [34, 253, 0]);
    assert_eq!(
        firmware.context,
        RequestContext::AuxiliaryFirmware(SchedulerRoute::Can(253))
    );

    let direct = request_facts(RequestKind::Values, Some(Route::Direct), generation).unwrap();
    assert_eq!(direct.payload, [96]);
    assert_eq!(
        direct.context,
        RequestContext::BmsValues(SchedulerRoute::Direct)
    );

    let routed = request_facts(RequestKind::Values, Some(Route::Can(42)), generation).unwrap();
    assert_eq!(routed.payload, [34, 42, 96]);
    assert_eq!(
        routed.context,
        RequestContext::BmsValues(SchedulerRoute::Can(42))
    );
    for facts in [scan, firmware, direct, routed] {
        assert_eq!(facts.priority, Priority::Auxiliary);
        assert!(facts.expects_response);
        assert_eq!(facts.generation, generation);
        assert!(
            facts
                .payload
                .iter()
                .all(|command| ![7, 8, 29, 30, 131].contains(command))
        );
    }
    assert_eq!(
        request_facts(RequestKind::CanScan, Some(Route::Direct), generation),
        Err(ServiceError::InvalidRoute)
    );
    assert_eq!(
        request_facts(RequestKind::Values, Some(Route::Can(254)), generation),
        Err(ServiceError::InvalidRoute)
    );
}

#[test]
fn every_completion_has_native_equivalent_cadence_and_failure_semantics() {
    for outcome in [
        RequestOutcome::Response,
        RequestOutcome::Sent,
        RequestOutcome::TimedOut,
        RequestOutcome::QueueFull,
        RequestOutcome::Cancelled,
        RequestOutcome::StaleGeneration,
    ] {
        let mut service = VescBmsTelemetryService::new();
        begin(&mut service, direct_configuration(1));
        let effects = complete(
            &mut service,
            RequestKind::Values,
            Some(Route::Direct),
            outcome,
        );
        let schedules = matches!(
            outcome,
            RequestOutcome::Response
                | RequestOutcome::Sent
                | RequestOutcome::TimedOut
                | RequestOutcome::QueueFull
        );
        assert_eq!(timer(&effects).is_some(), schedules, "{outcome:?}");
        assert_eq!(service.snapshot().timer.is_some(), schedules, "{outcome:?}");
        let failed = matches!(
            outcome,
            RequestOutcome::TimedOut | RequestOutcome::QueueFull
        );
        assert_eq!(
            service.snapshot().consecutive_failures,
            u8::from(failed),
            "{outcome:?}"
        );
    }
}

#[test]
fn malformed_response_counts_once_and_valid_response_recovers_all_fields() {
    let mut service = VescBmsTelemetryService::new();
    begin(&mut service, direct_configuration(1));
    for failure in 1..=FAILURE_THRESHOLD {
        complete(
            &mut service,
            RequestKind::Values,
            Some(Route::Direct),
            RequestOutcome::Response,
        );
        let snapshot = service.snapshot();
        let effects = service
            .apply(Command::ReceiveValues {
                service_generation: 1,
                auxiliary_generation: snapshot.auxiliary_generation,
                route: Route::Direct,
                payload: vec![96],
                observed_at_epoch_ms: 77,
            })
            .unwrap();
        assert_eq!(service.snapshot().consecutive_failures, failure);
        assert!(matches!(
            effects.first(),
            Some(Effect::Diagnostic(Diagnostic::MalformedValues))
        ));
        if failure < FAILURE_THRESHOLD {
            fire_timer(&mut service);
        }
    }
    assert_eq!(service.snapshot().status, Status::Unavailable);
    fire_timer(&mut service);
    complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    let effects = receive_valid(&mut service, Route::Direct, i64::MAX);
    assert!(matches!(
        effects.as_slice(),
        [Effect::PublishObservation(_), Effect::PublishState(_)]
    ));
    assert_eq!(service.snapshot().status, Status::Attached);
    assert_eq!(service.snapshot().consecutive_failures, 0);
    let observation = service.observation().unwrap();
    assert_eq!(observation.observed_at_epoch_ms, i64::MAX);
    assert_eq!(observation.total_voltage_v, 50.4);
    assert_eq!(observation.charge_voltage_v, 54.6);
    assert_eq!(observation.input_current_a, -12.345);
    assert_eq!(observation.input_ic_current_a, -12.3);
    assert_eq!(observation.session_amp_hours_ah, 1.25);
    assert_eq!(observation.session_watt_hours_wh, 62.5);
    assert_eq!(observation.cell_voltages_v, [4.0, 4.001]);
    assert_eq!(observation.balancing, [false, true]);
    assert_eq!(observation.temperatures_c, [20.0, 20.01]);
    assert_eq!(observation.ic_temperature_c, 27.5);
    assert_eq!(observation.humidity_sensor_temperature_c, 24.75);
    assert_eq!(observation.humidity_percent, 52.5);
    assert_eq!(observation.highest_cell_temperature_c, 29.25);
    assert_eq!(observation.state_of_charge, 0.875);
    assert_eq!(observation.state_of_health, 0.95);
    assert_eq!(observation.reported_can_id, 42);
    assert_eq!(observation.lifetime_charge_amp_hours_ah, 64.0);
    assert_eq!(observation.lifetime_charge_watt_hours_wh, 1024.0);
    assert_eq!(observation.lifetime_discharge_amp_hours_ah, 32.0);
    assert_eq!(observation.lifetime_discharge_watt_hours_wh, 512.0);
    assert!((observation.pressure_pa - 101_320.0).abs() < 0.01);
    assert_eq!(observation.data_version, Some(7));
    assert_eq!(observation.status.as_deref(), Some("Operational"));
}

#[test]
fn maximum_owned_collections_and_private_diagnostics_remain_bounded_and_redacted() {
    let mut service = VescBmsTelemetryService::new();
    begin(&mut service, direct_configuration(1));
    complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    let snapshot = service.snapshot();
    let payload = values_payload(50, 50, 42, Some((255, &[b'S'; 40])));
    service
        .apply(Command::ReceiveValues {
            service_generation: 1,
            auxiliary_generation: snapshot.auxiliary_generation,
            route: Route::Direct,
            payload,
            observed_at_epoch_ms: i64::MAX,
        })
        .unwrap();
    let observation = service.observation().unwrap();
    assert_eq!(observation.cell_voltages_v.len(), 50);
    assert_eq!(observation.balancing.len(), 50);
    assert_eq!(observation.temperatures_c.len(), 50);
    let debug = format!("{service:?} {observation:?}");
    assert!(!debug.contains("SSSS"));
    assert!(!debug.contains("50.4"));
    assert!(!debug.contains("Operational"));
    assert!(debug.contains("[redacted]"));
}

#[test]
fn stale_mismatched_and_repeated_events_do_not_mutate_current_state() {
    let mut service = VescBmsTelemetryService::new();
    begin(&mut service, direct_configuration(1));
    let before = service.snapshot();
    let stale = service
        .apply(Command::RequestCompleted {
            service_generation: 0,
            auxiliary_generation: before.auxiliary_generation,
            kind: RequestKind::Values,
            route: Some(Route::Direct),
            outcome: RequestOutcome::TimedOut,
        })
        .unwrap();
    assert_eq!(stale, [Effect::Diagnostic(Diagnostic::StaleEvent)]);
    assert_eq!(service.snapshot(), before);

    complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    let current = service.snapshot();
    let repeated = complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    assert_eq!(repeated, [Effect::Diagnostic(Diagnostic::MismatchedEvent)]);
    assert_eq!(service.snapshot(), current);
    let wrong_route = service
        .apply(Command::ReceiveValues {
            service_generation: 1,
            auxiliary_generation: current.auxiliary_generation,
            route: Route::Can(42),
            payload: values_payload(0, 0, 42, None),
            observed_at_epoch_ms: 1,
        })
        .unwrap();
    assert_eq!(
        wrong_route,
        [Effect::Diagnostic(Diagnostic::MismatchedEvent)]
    );
    assert_eq!(service.snapshot(), current);
}

#[test]
fn third_timeout_retains_observation_and_later_poll_recovers() {
    let mut service = VescBmsTelemetryService::new();
    begin(&mut service, direct_configuration(1));
    complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    receive_valid(&mut service, Route::Direct, 1);
    for failure in 1..=3 {
        fire_timer(&mut service);
        let effects = complete(
            &mut service,
            RequestKind::Values,
            Some(Route::Direct),
            RequestOutcome::TimedOut,
        );
        assert_eq!(service.snapshot().consecutive_failures, failure);
        assert!(service.snapshot().has_observation);
        assert_eq!(
            timer(&effects).map(|_| POLL_INTERVAL_MS),
            Some(POLL_INTERVAL_MS)
        );
    }
    assert_eq!(service.snapshot().status, Status::Unavailable);
    assert_eq!(service.observation().unwrap().observed_at_epoch_ms, 1);
    fire_timer(&mut service);
    complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    receive_valid(&mut service, Route::Direct, 2);
    assert_eq!(service.snapshot().status, Status::Attached);
    assert_eq!(service.observation().unwrap().observed_at_epoch_ms, 2);
}

#[test]
fn reset_cancels_timer_clears_private_state_and_invalidates_old_work() {
    let mut service = VescBmsTelemetryService::new();
    begin(&mut service, direct_configuration(1));
    complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    let token = service.snapshot().timer.unwrap();
    receive_valid(&mut service, Route::Direct, 1);
    let effects = service.apply(Command::Reset).unwrap();
    assert!(effects.contains(&Effect::CancelTimer(token)));
    let reset = service.snapshot();
    assert_eq!(reset.service_generation, None);
    assert_eq!(reset.status, Status::Discovering);
    assert!(!reset.has_observation);
    assert_eq!(reset.consecutive_failures, 0);
    let stale = service.apply(Command::TimerFired(token)).unwrap();
    assert_eq!(stale, [Effect::Diagnostic(Diagnostic::StaleEvent)]);
    assert_eq!(service.snapshot(), reset);
}

#[test]
fn enqueue_rejection_counts_once_and_continues_recovery_polling() {
    let mut service = VescBmsTelemetryService::new();
    begin(&mut service, direct_configuration(1));
    let snapshot = service.snapshot();
    let effects = service
        .apply(Command::EnqueueFailed {
            service_generation: 1,
            auxiliary_generation: snapshot.auxiliary_generation,
            kind: RequestKind::Values,
            route: Some(Route::Direct),
        })
        .unwrap();
    assert!(matches!(
        effects.first(),
        Some(Effect::Diagnostic(Diagnostic::EnqueueRejected))
    ));
    assert_eq!(service.snapshot().consecutive_failures, 1);
    assert!(service.snapshot().timer.is_some());
}

#[test]
fn non_positive_observation_time_is_rejected_without_consuming_the_response() {
    let mut service = VescBmsTelemetryService::new();
    begin(&mut service, direct_configuration(1));
    complete(
        &mut service,
        RequestKind::Values,
        Some(Route::Direct),
        RequestOutcome::Response,
    );
    let before = service.snapshot();
    for observed_at_epoch_ms in [i64::MIN, -1, 0] {
        let error = service
            .apply(Command::ReceiveValues {
                service_generation: 1,
                auxiliary_generation: before.auxiliary_generation,
                route: Route::Direct,
                payload: values_payload(0, 0, 42, None),
                observed_at_epoch_ms,
            })
            .unwrap_err();
        assert_eq!(error, ServiceError::InvalidObservationTime);
        assert_eq!(service.snapshot(), before);
    }
    receive_valid(&mut service, Route::Direct, 1);
    assert_eq!(service.snapshot().status, Status::Attached);
}

#[test]
fn every_checked_can_route_has_exact_firmware_and_values_facts() {
    let generation = SchedulerGeneration(17);
    for can_id in 0..=253 {
        let route = Some(Route::Can(can_id));
        assert!(request_facts(RequestKind::Firmware, route, generation).is_ok());
        assert!(request_facts(RequestKind::Values, route, generation).is_ok());
    }
    for can_id in [254, 255] {
        let route = Some(Route::Can(can_id));
        assert_eq!(
            request_facts(RequestKind::Firmware, route, generation),
            Err(ServiceError::InvalidRoute)
        );
        assert_eq!(
            request_facts(RequestKind::Values, route, generation),
            Err(ServiceError::InvalidRoute)
        );
    }
}

#[test]
fn reusable_inventory_accepts_64_can_nodes_and_rejects_65_atomically() {
    let inventory = (0..64)
        .map(|can_id| controller(Route::Can(can_id)))
        .collect::<Vec<_>>();
    let mut service = VescBmsTelemetryService::new();
    let configuration = float_view_vesc_bms_telemetry_service::Configuration {
        service_generation: 1,
        scheduler_generation: SchedulerGeneration(11),
        local_identity: controller(Route::Direct),
        reusable_inventory: Some(inventory.clone()),
        preferred_can_id: None,
    };
    assert!(service.apply(Command::Begin(configuration.clone())).is_ok());
    assert_eq!(service.snapshot().status, Status::NotFound);

    let before = service.snapshot();
    let mut oversized = inventory;
    oversized.push(controller(Route::Can(64)));
    let error = service
        .apply(Command::Begin(
            float_view_vesc_bms_telemetry_service::Configuration {
                service_generation: 2,
                scheduler_generation: SchedulerGeneration(12),
                local_identity: controller(Route::Direct),
                reusable_inventory: Some(oversized),
                preferred_can_id: None,
            },
        ))
        .unwrap_err();
    assert_eq!(
        error,
        ServiceError::DiscoveryInput(
            float_view_auxiliary_node_discovery::InputError::InvalidInventorySize
        )
    );
    assert_eq!(service.snapshot(), before);
}
