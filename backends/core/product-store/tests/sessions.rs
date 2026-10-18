#[path = "../../session-sqlite/tests/support/mod.rs"]
mod support;
use float_view_product_store::{Error, ProductStore};
use float_view_refloat_telemetry_service::RideState;
use float_view_session_sqlite::SessionWriter;
use rusqlite::Connection;
use support::{TestDirectory, sample};

fn setup() -> (TestDirectory, ProductStore, String) {
    let dir = TestDirectory::new();
    let mut store = ProductStore::open(dir.path()).unwrap();
    store
        .update_settings(0, r#"{"minimumSessionDistance":0}"#)
        .unwrap();
    let board = store
        .ensure_board("local", Some("Before"), Some(&[1; 12]), 1)
        .unwrap();
    (dir, store, board.saved_device_id)
}
fn record(store: &ProductStore, id: &str) {
    let mut writer = SessionWriter::open(store.session_path(id).unwrap()).unwrap();
    let mut samples = [sample(1, 0), sample(2, 1_000)];
    for s in &mut samples {
        s.ride_state = RideState::Riding;
        s.speed_kph = Some(36.);
    }
    writer.append_samples(&samples).unwrap();
    writer.close().unwrap();
}
#[test]
fn fault_only_sessions_survive_distance_filter_and_restart() {
    for restart in [false, true] {
        let (dir, mut store, board) = setup();
        store
            .update_settings(
                store.settings().unwrap().revision,
                r#"{"minimumSessionDistance":1000}"#,
            )
            .unwrap();
        let s = store.create_session(&board, 1, u64::MAX).unwrap();
        let mut writer = SessionWriter::open(store.session_path(&s.session_id).unwrap()).unwrap();
        writer
            .append_fault_history(
                &float_view_refloat_telemetry_service::FaultHistoryObservation {
                    source_clock: None,
                    elapsed_ms: 100,
                    observed_at_epoch_ms: 1_000_100,
                    records: vec![float_view_refloat_telemetry_service::FaultRecord {
                        source_ticks: 100,
                        active: true,
                        raw_code: 4,
                    }],
                },
            )
            .unwrap();
        writer.close().unwrap();
        if restart {
            drop(store);
            store = ProductStore::open(dir.path()).unwrap();
            assert_eq!(store.session(&s.session_id).unwrap().state, "interrupted");
        } else {
            assert!(
                store
                    .close_session(&s.session_id, 1_000_200, "disconnect")
                    .unwrap()
                    .is_some()
            );
        }
        assert_eq!(
            store.session(&s.session_id).unwrap().recorded_sample_count,
            0
        );
    }
}

#[test]
fn session_snapshots_are_immutable_and_creation_is_storage_gated() {
    let (_dir, mut store, board) = setup();
    assert_eq!(
        store.create_session(&board, 1, 262_143_999),
        Err(Error::Full)
    );
    assert!(
        store
            .sessions(None, 100, None, false)
            .unwrap()
            .items
            .is_empty()
    );
    let first = store.create_session(&board, 1, 262_144_000).unwrap();
    store.update_board(&board, 0, Some("After")).unwrap();
    store
        .update_profile(&board, 0, r#"{"motor":"new"}"#, 2)
        .unwrap();
    assert_eq!(
        store.session(&first.session_id).unwrap().device_snapshot["displayName"],
        "Before"
    );
    assert!(
        store
            .session(&first.session_id)
            .unwrap()
            .profile_snapshot
            .get("motor")
            .is_none()
    );
    assert!(matches!(
        store.create_session(&board, 2, u64::MAX),
        Err(Error::Capacity)
    ));
    assert_eq!(store.forget_board(&board), Err(Error::ActiveSession));
}
#[test]
fn split_commit_failure_preserves_the_durable_session_and_reconciles_on_restart() {
    let (dir, mut store, board) = setup();
    let s = store.create_session(&board, 1, u64::MAX).unwrap();
    let catalog = Connection::open(dir.file("catalog.sqlite")).unwrap();
    catalog.execute_batch("CREATE TRIGGER fail_catalog BEFORE UPDATE ON session_catalog BEGIN SELECT RAISE(ABORT,'injected failure'); END").unwrap();
    record(&store, &s.session_id);
    assert!(store.reconcile_session(&s.session_id).is_err());
    assert_eq!(
        store.session(&s.session_id).unwrap().recorded_sample_count,
        0
    );
    let writer = SessionWriter::open(store.session_path(&s.session_id).unwrap()).unwrap();
    assert_eq!(writer.marker().unwrap().recorded_sample_count, 2);
    writer.close().unwrap();
    catalog.execute_batch("DROP TRIGGER fail_catalog").unwrap();
    drop(catalog);
    drop(store);
    let recovered = ProductStore::open(dir.path()).unwrap();
    let s = recovered.session(&s.session_id).unwrap();
    assert_eq!(s.recorded_sample_count, 2);
    assert_eq!(s.state, "interrupted");
    assert_eq!(s.integrity, "recovered");
    drop(recovered);
    assert_eq!(
        ProductStore::open(dir.path())
            .unwrap()
            .session(&s.session_id)
            .unwrap()
            .recorded_sample_count,
        2
    );
}
#[test]
fn failed_session_transaction_does_not_publish_or_advance_catalog() {
    let (_dir, mut store, board) = setup();
    let s = store.create_session(&board, 1, u64::MAX).unwrap();
    let path = store.session_path(&s.session_id).unwrap();
    let mut writer = SessionWriter::open(&path).unwrap();
    let blocker = Connection::open(&path).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(writer.append_samples(&[sample(1, 0)]).is_err());
    blocker.execute_batch("ROLLBACK").unwrap();
    writer.close().unwrap();
    let marker = store.reconcile_session(&s.session_id).unwrap();
    assert_eq!(marker.recorded_sample_count, 0);
    assert_eq!(
        store.session(&s.session_id).unwrap().recorded_sample_count,
        0
    );
}
#[test]
fn keyset_pages_bind_query_revision_and_restart_and_handle_equal_timestamps() {
    let (_dir, mut store, board) = setup();
    for _ in 0..3 {
        let s = store.create_session(&board, 1, u64::MAX).unwrap();
        record(&store, &s.session_id);
        store.close_session(&s.session_id, 2, "test").unwrap();
    }
    let page = store.sessions(None, 1, None, false).unwrap();
    let cursor = page.next_cursor.unwrap();
    let next = store.sessions(None, 1, Some(&cursor), false).unwrap();
    assert_ne!(page.items[0].session_id, next.items[0].session_id);
    assert_eq!(
        store.sessions(Some("different"), 1, Some(&cursor), false),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        store.sessions(None, 1, Some(&cursor), true),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        store.sessions(None, 101, None, false),
        Err(Error::InvalidInput)
    );
    store.delete_session(&next.items[0].session_id).unwrap();
    assert!(matches!(
        store.sessions(None, 1, Some(&cursor), false),
        Err(Error::Conflict { .. })
    ));
}
#[test]
fn active_and_unreadable_sessions_are_never_deleted_as_empty() {
    let (dir, mut store, board) = setup();
    let s = store.create_session(&board, 1, u64::MAX).unwrap();
    assert_eq!(
        store.delete_session(&s.session_id),
        Err(Error::ActiveSession)
    );
    let path = store.session_path(&s.session_id).unwrap();
    std::fs::write(&path, "broken session").unwrap();
    drop(store);
    assert!(ProductStore::open(dir.path()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"broken session");
}
#[test]
fn deletion_waits_for_reader_and_export_leases_and_can_be_cancelled() {
    let (_dir, mut store, board) = setup();
    let s = store.create_session(&board, 1, u64::MAX).unwrap();
    record(&store, &s.session_id);
    store.close_session(&s.session_id, 2, "done").unwrap();
    let reader = store.lease_session(&s.session_id).unwrap();
    let export = store.lease_session(&s.session_id).unwrap();
    store.delete_session(&s.session_id).unwrap();
    assert!(reader.path().exists());
    assert!(matches!(store.session(&s.session_id), Err(Error::NotFound)));
    assert!(store.cancel_pending_delete(&s.session_id).unwrap());
    assert!(store.session(&s.session_id).is_ok());
    store.delete_session(&s.session_id).unwrap();
    drop(reader);
    store.cleanup_deletes().unwrap();
    assert!(export.path().exists());
    let path = export.path().to_owned();
    drop(export);
    store.cleanup_deletes().unwrap();
    assert!(!path.exists());
    assert!(!store.cancel_pending_delete(&s.session_id).unwrap());
    store.delete_session(&s.session_id).unwrap();
}
#[test]
fn interruption_removes_provisional_sessions_but_forget_retains_leased_files_until_cleanup() {
    let (dir, mut store, board) = setup();
    let provisional = store.create_session(&board, 1, u64::MAX).unwrap();
    let path = store.session_path(&provisional.session_id).unwrap();
    drop(store);
    let mut store = ProductStore::open(dir.path()).unwrap();
    assert!(!path.exists());
    let s = store.create_session(&board, 1, u64::MAX).unwrap();
    record(&store, &s.session_id);
    store.close_session(&s.session_id, 2, "done").unwrap();
    let lease = store.lease_session(&s.session_id).unwrap();
    let path = lease.path().to_owned();
    store.forget_board(&board).unwrap();
    assert!(!store.cancel_pending_delete(&s.session_id).unwrap());
    assert!(matches!(store.board(&board), Err(Error::NotFound)));
    assert!(path.exists());
    drop(lease);
    store.cleanup_deletes().unwrap();
    assert!(!path.exists());
}

#[test]
fn a_catalog_path_escape_is_rejected_before_opening_the_session() {
    let (dir, mut store, board) = setup();
    let s = store.create_session(&board, 1, u64::MAX).unwrap();
    let db = Connection::open(dir.file("catalog.sqlite")).unwrap();
    db.execute(
        "UPDATE session_catalog SET file_name='../outside.sqlite' WHERE session_id=?",
        [&s.session_id],
    )
    .unwrap();
    assert_eq!(store.session_path(&s.session_id), Err(Error::UnsafePath));
}
#[test]
fn catalog_insertion_failure_cleans_only_the_unattached_session() {
    let (dir, mut store, board) = setup();
    let db = Connection::open(dir.file("catalog.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON session_catalog BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
    assert!(store.create_session(&board, 1, u64::MAX).is_err());
    assert!(
        store
            .sessions(None, 100, None, false)
            .unwrap()
            .items
            .is_empty()
    );
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("session-")
    }));
    assert!(store.board(&board).is_ok());
}
#[test]
fn restart_invalidates_old_session_cursor_even_when_revision_matches() {
    let (dir, mut store, board) = setup();
    for _ in 0..2 {
        let s = store.create_session(&board, 1, u64::MAX).unwrap();
        record(&store, &s.session_id);
        store.close_session(&s.session_id, 2, "done").unwrap();
    }
    let cursor = store
        .sessions(None, 1, None, false)
        .unwrap()
        .next_cursor
        .unwrap();
    drop(store);
    let store = ProductStore::open(dir.path()).unwrap();
    assert!(matches!(
        store.sessions(None, 1, Some(&cursor), false),
        Err(Error::Conflict { .. })
    ));
}

#[test]
fn storage_totals_count_owned_files_and_sidecars_per_board() {
    let (dir, mut store, board) = setup();
    let history = store.board(&board).unwrap().device_history_id;
    assert_eq!(
        store.board_session_storage_bytes(&history, None).unwrap(),
        0
    );
    let first = store.create_session(&board, 1, u64::MAX).unwrap();
    let path = store.session_path(&first.session_id).unwrap();
    let database_bytes = std::fs::metadata(&path).unwrap().len();
    std::fs::write(format!("{}-wal", path.display()), [0; 123]).unwrap();
    std::fs::write(format!("{}-shm", path.display()), [0; 456]).unwrap();
    assert_eq!(
        store.session_storage_bytes(&first.session_id).unwrap(),
        database_bytes + 579
    );
    assert_eq!(
        store.board_session_storage_bytes(&history, None).unwrap(),
        database_bytes + 579
    );
    let other = store
        .ensure_board("other", Some("Other"), Some(&[2; 12]), 2)
        .unwrap();
    assert_eq!(
        store
            .board_session_storage_bytes(&other.device_history_id, None)
            .unwrap(),
        0
    );
    // Shared storage and unrelated temporary files are outside session totals.
    std::fs::write(dir.file("unrelated.tmp"), [0; 999]).unwrap();
    assert_eq!(
        store.board_session_storage_bytes(&history, None).unwrap(),
        database_bytes + 579
    );
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        store.board_session_storage_bytes(&history, None),
        Err(Error::Io)
    );
    assert!(store.session(&first.session_id).is_ok());
}

#[test]
fn storage_totals_include_unloaded_sessions_and_follow_deletion() {
    let (_dir, mut store, board) = setup();
    let history = store.board(&board).unwrap().device_history_id;
    let first = store.create_session(&board, 1, u64::MAX).unwrap();
    record(&store, &first.session_id);
    store
        .close_session(&first.session_id, 2000, "test")
        .unwrap();
    let second = store.create_session(&board, 2001, u64::MAX).unwrap();
    record(&store, &second.session_id);
    store
        .close_session(&second.session_id, 4000, "test")
        .unwrap();
    let first_bytes = store.session_storage_bytes(&first.session_id).unwrap();
    let second_bytes = store.session_storage_bytes(&second.session_id).unwrap();
    let page = store.sessions(Some(&history), 1, None, false).unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.next_cursor.is_some());
    assert_eq!(
        store.board_session_storage_bytes(&history, None).unwrap(),
        first_bytes + second_bytes
    );
    store.delete_session(&first.session_id).unwrap();
    assert_eq!(
        store.board_session_storage_bytes(&history, None).unwrap(),
        second_bytes
    );
    store.delete_session(&second.session_id).unwrap();
    assert_eq!(
        store.board_session_storage_bytes(&history, None).unwrap(),
        0
    );
}

#[test]
fn cleanup_failure_after_board_removal_does_not_leave_auto_connect_selection() {
    let (_dir, mut store, board) = setup();
    let selected = store
        .update_settings(
            store.settings().unwrap().revision,
            &serde_json::json!({"autoConnectSavedDeviceId":board}).to_string(),
        )
        .unwrap();
    let session = store.create_session(&board, 1, u64::MAX).unwrap();
    assert_eq!(store.forget_board(&board), Err(Error::ActiveSession));
    assert_eq!(store.settings().unwrap(), selected);
    record(&store, &session.session_id);
    store.close_session(&session.session_id, 2, "done").unwrap();
    let path = store.session_path(&session.session_id).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(store.forget_board(&board).is_err());
    assert!(matches!(store.board(&board), Err(Error::NotFound)));
    assert_eq!(store.settings().unwrap().auto_connect_saved_device_id, None);
    assert_eq!(store.settings().unwrap().revision, selected.revision + 1);
}
