mod support;
use float_view_refloat_telemetry_service::{FaultHistoryObservation, FaultRecord};
use float_view_session_sqlite::{SessionMetadata, SessionReader, SessionWriter};
use support::{TestDirectory, sample};

#[test]
fn persists_fault_start_end_and_latch_without_rewriting_active_samples() {
    let dir = TestDirectory::new();
    let path = dir.file("fault.sqlite");
    let mut writer = SessionWriter::create(
        &path,
        SessionMetadata {
            expected_riding_persistence_ms: 100,
            expected_idle_persistence_ms: 1000,
            committed_at_epoch_ms: 1_000_000,
        },
    )
    .unwrap();
    let mut first = sample(1, 100);
    first.fatal_error_latched = Some(true);
    writer.append_samples(&[first.clone()]).unwrap();
    writer
        .append_fault_history(&FaultHistoryObservation {
            source_clock: Some(float_view_refloat_telemetry_service::FaultClock {
                ticks: 12350,
                elapsed_ms: 105,
            }),
            elapsed_ms: 110,
            observed_at_epoch_ms: 1_000_110,
            records: vec![
                FaultRecord {
                    kind: float_view_refloat_telemetry_service::AlertKind::FirmwareFault,
                    source_ticks: 12300,
                    active: true,
                    raw_code: 4,
                },
                FaultRecord {
                    kind: float_view_refloat_telemetry_service::AlertKind::FirmwareFault,
                    source_ticks: 12350,
                    active: false,
                    raw_code: 0,
                },
            ],
        })
        .unwrap();
    let mut next = sample(2, 120);
    next.fatal_error_latched = Some(true);
    writer.append_samples(&[next.clone()]).unwrap();
    writer.close().unwrap();
    // Reopening retains transition deduplication as well as the historical raw code.
    let mut writer = SessionWriter::open(&path).unwrap();
    next.sequence = 3;
    next.elapsed_ms = 130;
    writer.append_samples(&[next]).unwrap();
    let mut clear = sample(4, 140);
    clear.fatal_error_latched = Some(false);
    let boundary = writer.append_samples(&[clear]).unwrap();
    writer.close().unwrap();
    let mut reader = SessionReader::open(&path).unwrap();
    let page = reader
        .event_page(0, 200, 100, None, (&boundary).into())
        .unwrap();
    let faults: Vec<_> = page
        .rows
        .iter()
        .filter(|r| r.kind == "firmwareFaultHistory")
        .collect();
    assert_eq!(faults.len(), 2);
    let start: serde_json::Value = serde_json::from_str(&faults[0].details_json).unwrap();
    assert_eq!(
        start,
        serde_json::json!({"value":"absoluteOverCurrent","active":true,"rawCode":4,"sourceTicks":12300,"occurredElapsedMs":100})
    );
    assert!(faults.iter().all(|r| r.source_sample_sequence.is_none()));
    let latches: Vec<_> = page
        .rows
        .iter()
        .filter(|r| r.kind == "fatalErrorLatched")
        .collect();
    assert_eq!(latches.len(), 2);
    assert_eq!(latches[1].details_json, r#"{"value":"cleared"}"#);
}

#[test]
fn timeout_records_reopen_without_becoming_firmware_faults() {
    use float_view_refloat_telemetry_service::{AlertKind, StopCondition};
    let dir = TestDirectory::new();
    let path = dir.file("timeout.sqlite");
    let mut writer = SessionWriter::create(
        &path,
        SessionMetadata {
            expected_riding_persistence_ms: 100,
            expected_idle_persistence_ms: 1000,
            committed_at_epoch_ms: 1_000_000,
        },
    )
    .unwrap();
    let mut first = sample(1, 100);
    first.stop_condition = Some(StopCondition::ImuTimeout);
    first.imu_timeout_active = Some(true);
    writer.append_samples(&[first]).unwrap();
    let marker = writer
        .append_fault_history(&FaultHistoryObservation {
            source_clock: None,
            elapsed_ms: 110,
            observed_at_epoch_ms: 1_000_110,
            records: vec![
                FaultRecord {
                    kind: AlertKind::ImuTimeout,
                    source_ticks: 100,
                    active: true,
                    raw_code: 0,
                },
                FaultRecord {
                    kind: AlertKind::ImuTimeout,
                    source_ticks: 101,
                    active: false,
                    raw_code: 0,
                },
            ],
        })
        .unwrap();
    writer.close().unwrap();
    let mut writer = SessionWriter::open(&path).unwrap();
    assert!(
        writer
            .append_fault_history(&FaultHistoryObservation {
                source_clock: None,
                elapsed_ms: 120,
                observed_at_epoch_ms: 1_000_120,
                records: vec![FaultRecord {
                    kind: AlertKind::ImuTimeout,
                    source_ticks: 102,
                    active: true,
                    raw_code: 4
                }]
            })
            .is_err()
    );
    writer.close().unwrap();
    let mut reader = SessionReader::open(&path).unwrap();
    let rows = reader
        .event_page(0, 200, 100, None, (&marker).into())
        .unwrap()
        .rows;
    let timeouts: Vec<_> = rows
        .iter()
        .filter(|r| r.kind == "imuTimeoutHistory")
        .collect();
    assert_eq!(timeouts.len(), 2);
    assert!(!rows.iter().any(|r| r.kind == "firmwareFaultHistory"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&timeouts[0].details_json).unwrap(),
        serde_json::json!({"active":true,"sourceTicks":100,"occurredElapsedMs":null})
    );
    assert!(
        rows.iter()
            .any(|r| r.kind == "stopCondition" && r.details_json.contains("imuTimeout"))
    );
}

#[test]
fn mixed_timeout_history_rolls_back_as_one_transaction() {
    use float_view_refloat_telemetry_service::AlertKind;
    let dir = TestDirectory::new();
    let path = dir.file("rollback.sqlite");
    let mut writer = SessionWriter::create(
        &path,
        SessionMetadata {
            expected_riding_persistence_ms: 100,
            expected_idle_persistence_ms: 1000,
            committed_at_epoch_ms: 1_000_000,
        },
    )
    .unwrap();
    let before = writer.marker().unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER reject_clear BEFORE INSERT ON session_event WHEN NEW.kind='imuTimeoutHistory' AND json_extract(NEW.details_json,'$.active')=0 BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let observation = FaultHistoryObservation {
        source_clock: None,
        elapsed_ms: 100,
        observed_at_epoch_ms: 1_000_100,
        records: vec![
            FaultRecord {
                kind: AlertKind::ImuTimeout,
                source_ticks: 100,
                active: true,
                raw_code: 0,
            },
            FaultRecord {
                kind: AlertKind::ImuTimeout,
                source_ticks: 101,
                active: false,
                raw_code: 0,
            },
        ],
    };
    assert!(writer.append_fault_history(&observation).is_err());
    assert_eq!(writer.marker().unwrap(), before);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM session_event", [], |r| r
            .get::<_, u64>(0))
            .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER reject_clear;").unwrap();
    writer.append_fault_history(&observation).unwrap();
}
