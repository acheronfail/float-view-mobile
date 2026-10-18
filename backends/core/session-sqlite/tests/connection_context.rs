mod support;
use float_view_session_sqlite::*;
use support::{TestDirectory, sample};
fn id(n: usize) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}
fn context() -> ConnectionContext {
    let firmware = ContextFirmware {
        major: 6,
        minor: 6,
        test_version: 2,
        hardware_name: Some("Thor300".into()),
        firmware_name: None,
    };
    ConnectionContext {
        version: 1,
        refloat_version: "1.3.0".into(),
        controller: firmware.clone(),
        transport: ContextFirmware {
            firmware_name: Some("VESC Express".into()),
            ..firmware
        },
        connection: ContextRoute::Can { can_id: 42 },
    }
}
fn writer(path: &std::path::Path) -> SessionWriter {
    SessionWriter::create(
        path,
        SessionMetadata {
            expected_riding_persistence_ms: 100,
            expected_idle_persistence_ms: 1000,
            committed_at_epoch_ms: 1_000_000,
        },
    )
    .unwrap()
}
#[test]
fn context_is_closed_and_bounded_without_private_identity() {
    let value = context();
    assert!(value.valid());
    let mut legacy = value.clone();
    legacy.refloat_version = "0.9".into();
    legacy.connection = ContextRoute::Direct;
    assert!(legacy.valid());
    for invalid in ["1", "1.2.3.4", "1.256", "1.-2", "1.2.foo"] {
        legacy.refloat_version = invalid.into();
        assert!(!legacy.valid());
    }
    let mut raw = serde_json::to_value(&value).unwrap();
    raw["controller"]["controllerUuid"] = serde_json::json!("private");
    assert!(serde_json::from_value::<ConnectionContext>(raw).is_err());
    let mut invalid = value.clone();
    invalid.controller.hardware_name = Some("é".repeat(65));
    assert!(!invalid.valid());
    invalid = value.clone();
    invalid.connection = ContextRoute::Can { can_id: 254 };
    assert!(!invalid.valid());
    invalid = value;
    invalid.version = 2;
    assert!(!invalid.valid());
}
#[test]
fn contexts_are_atomic_idempotent_and_preserve_unchanged_reconnects() {
    let dir = TestDirectory::new();
    let path = dir.file("context.db");
    let mut w = writer(&path);
    let first = w
        .append_connection_context(&id(1), 0, 1_000_000, context())
        .unwrap();
    assert_eq!(first.recorded_sample_count, 0);
    assert_eq!(first.committed_event_count, 1);
    assert_eq!(
        w.append_connection_context(&id(1), 100, 1_000_100, context())
            .unwrap(),
        first
    );
    w.append_connection_context(&id(2), 0, 1_000_000, context())
        .unwrap();
    let mut invalid = context();
    invalid.version = 2;
    assert!(
        w.append_connection_context(&id(3), 100, 1_000_100, invalid)
            .is_err()
    );
    assert_eq!(w.marker().unwrap().committed_event_count, 2);
    w.close().unwrap();
    let mut w = SessionWriter::open(&path).unwrap();
    assert_eq!(
        w.append_connection_context(&id(2), 10, 1_000_010, context())
            .unwrap()
            .committed_event_count,
        2
    );
    let boundary = w.marker().unwrap();
    let mut reader = SessionReader::open(&path).unwrap();
    assert_eq!(
        reader.connection_contexts((&first).into()).unwrap().len(),
        1
    );
    let events = reader.connection_contexts((&boundary).into()).unwrap();
    assert_eq!(events.len(), 2);
    assert!(events[0].context == events[1].context);
}
#[test]
fn context_survives_pruning_with_no_sample_foreign_key() {
    let dir = TestDirectory::new();
    let path = dir.file("prune.db");
    let mut w = writer(&path);
    w.append_connection_context(&id(1), 0, 1_000_000, context())
        .unwrap();
    w.append_samples(&(1..=50).map(|n| sample(n, n * 100)).collect::<Vec<_>>())
        .unwrap();
    w.append_connection_context(&id(3), 5_050, 1_005_050, context())
        .unwrap();
    w.append_samples(&(51..=100).map(|n| sample(n, n * 100)).collect::<Vec<_>>())
        .unwrap();
    w.append_connection_context(&id(2), 10_050, 1_010_050, context())
        .unwrap();
    let before = w.marker().unwrap();
    w.close().unwrap();
    let expected = SessionReader::open(&path)
        .unwrap()
        .connection_contexts((&before).into())
        .unwrap();
    let mut pruner = SessionPruner::open(&path).unwrap();
    let plan = pruner.preview(2).unwrap();
    pruner.commit(plan, 1_020_000).unwrap();
    let mut reader = SessionReader::open(&path).unwrap();
    let after = reader.marker().unwrap();
    assert!(after.stored_sample_count < before.stored_sample_count);
    assert_eq!(
        reader.connection_contexts((&after).into()).unwrap(),
        expected
    );
}
#[test]
fn cap_checks_after_duplicate_and_invalid_import_data_is_rejected() {
    let dir = TestDirectory::new();
    let path = dir.file("cap.db");
    let mut w = writer(&path);
    for n in 0..1000 {
        w.append_connection_context(&id(n), n as u64, 1_000_000 + n as i64, context())
            .unwrap();
    }
    assert!(
        w.append_connection_context(&id(999), 999, 1_000_999, context())
            .is_ok()
    );
    assert_eq!(
        w.append_connection_context(&id(1000), 1000, 1_001_000, context()),
        Err(WriterError::Full)
    );
    w.close().unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE session_event SET details_json=json_set(details_json,'$.context.version',2) WHERE sequence=1",[]).unwrap();
    drop(db);
    assert!(SessionReader::open(&path).is_err());
    assert!(SessionWriter::open(&path).is_err());
}
#[test]
fn failed_insert_rolls_back_boundary_and_retries_without_partial_event() {
    let dir = TestDirectory::new();
    let path = dir.file("failed.db");
    let mut w = writer(&path);
    let before = w.marker().unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER reject_context BEFORE UPDATE ON commit_marker BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    assert!(
        w.append_connection_context(&id(1), 100, 1_000_100, context())
            .is_err()
    );
    assert_eq!(w.marker().unwrap(), before);
    db.execute_batch("DROP TRIGGER reject_context;").unwrap();
    let after = w
        .append_connection_context(&id(1), 100, 1_000_100, context())
        .unwrap();
    assert_eq!(after.committed_event_count, 1);
    assert_eq!(after.committed_event_sequence, 1);
}

#[test]
fn changed_firmware_and_route_do_not_rewrite_previous_context() {
    let dir = TestDirectory::new();
    let path = dir.file("changed.db");
    let mut w = writer(&path);
    let mut first = context();
    first.connection = ContextRoute::Direct;
    w.append_connection_context(&id(1), 0, 1_000_000, first.clone())
        .unwrap();
    let mut second = context();
    second.refloat_version = "1.4.0".into();
    second.controller.minor = 7;
    let boundary = w
        .append_connection_context(&id(2), 500, 1_000_500, second.clone())
        .unwrap();
    let rows = SessionReader::open(&path)
        .unwrap()
        .connection_contexts((&boundary).into())
        .unwrap();
    assert!(rows[0].context == first);
    assert!(rows[1].context == second);
}

#[test]
fn untrusted_context_payloads_and_counts_fail_without_modifying_storage() {
    for oversized in [false, true] {
        let dir = TestDirectory::new();
        let path = dir.file("untrusted.db");
        let mut w = writer(&path);
        w.append_connection_context(&id(1), 0, 1_000_000, context())
            .unwrap();
        w.close().unwrap();
        let db = rusqlite::Connection::open(&path).unwrap();
        if oversized {
            db.execute(
                "UPDATE session_event SET details_json=?",
                [" ".repeat(4097)],
            )
            .unwrap();
        } else {
            db.execute_batch("WITH RECURSIVE numbers(n) AS (VALUES(2) UNION ALL SELECT n+1 FROM numbers WHERE n<1001) INSERT INTO session_event(sequence,monotonic_elapsed_ms,observed_at_ms,kind,details_json) SELECT n,n,1000000+n,'connectionContext',json_set((SELECT details_json FROM session_event WHERE sequence=1),'$.sequence',n,'$.elapsedMs',n,'$.observedAtMs',1000000+n,'$.operationId',printf('00000000-0000-4000-8000-%012x',n)) FROM numbers; UPDATE commit_marker SET committed_event_sequence=1001,committed_event_count=1001,committed_elapsed_ms=1001;").unwrap();
        }
        drop(db);
        let before = std::fs::read(&path).unwrap();
        assert!(SessionReader::open(&path).is_err());
        assert!(SessionWriter::open(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
