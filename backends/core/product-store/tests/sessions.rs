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
