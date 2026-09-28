#[path = "../../session-sqlite/tests/support/mod.rs"]
mod support;
use float_view_product_store::{Error, ProductStore};
use float_view_refloat_telemetry_service::RideState;
use float_view_session_sqlite::{SessionReader, SessionWriter};
use rusqlite::Connection;
use support::{TestDirectory, sample};
fn setup() -> (TestDirectory, ProductStore, Vec<String>) {
    let dir = TestDirectory::new();
    let mut store = ProductStore::open(dir.path()).unwrap();
    store
        .update_settings(0, r#"{"minimumSessionDistance":0}"#)
        .unwrap();
    let board = store
        .ensure_board("local", None, Some(&[1; 12]), 0)
        .unwrap();
    let mut ids = Vec::new();
    for at in 1..=3 {
        let s = store
            .create_session(&board.saved_device_id, at, u64::MAX)
            .unwrap();
        let mut writer = SessionWriter::open(store.session_path(&s.session_id).unwrap()).unwrap();
        let samples: Vec<_> = (1..=100)
            .map(|i| {
                let mut s = sample(i, (i - 1) * 1000);
                s.ride_state = RideState::Riding;
                s.speed_kph = Some(36.);
                s
            })
            .collect();
        writer.append_samples(&samples).unwrap();
        writer.close().unwrap();
        store
            .close_session(&s.session_id, at + 100_000, "disconnect")
            .unwrap();
        ids.push(s.session_id);
    }
    ids.sort();
    (dir, store, ids)
}
#[test]
fn bounded_job_progress_survives_deleted_cursor_and_restart() {
    let (dir, mut store, ids) = setup();
    assert_eq!(
        store.begin_maintenance(None, 0, false, 1),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        store.begin_maintenance(None, 301, false, 1),
        Err(Error::InvalidInput)
    );
    let job = store.begin_maintenance(None, 10, false, 1).unwrap();
    assert_eq!(job.upper_session_id.as_ref(), ids.last());
    assert_eq!(
        store.begin_maintenance(None, 10, false, 1),
        Err(Error::Busy)
    );
    let work = store.next_maintenance_work(2).unwrap().unwrap();
    assert!(matches!(store.next_maintenance_work(2), Err(Error::Busy)));
    assert!(matches!(store.lease_session(&ids[0]), Err(Error::Busy)));
    assert_eq!(store.delete_session(&ids[0]), Err(Error::Busy));
    let job = store.complete_maintenance_work(work.run(3), 4).unwrap();
    assert_eq!(job.completed_sessions, 1);
    assert_eq!(job.last_session_id.as_ref(), Some(&ids[0]));
    store.delete_session(&ids[0]).unwrap();
    drop(store);
    let mut store = ProductStore::open(dir.path()).unwrap();
    while let Some(work) = store.next_maintenance_work(5).unwrap() {
        store.complete_maintenance_work(work.run(6), 7).unwrap();
    }
    let job = store.maintenance().unwrap().unwrap();
    assert_eq!(job.state, "completed");
    assert_eq!(job.completed_sessions, 3);
    for id in &ids[1..] {
        assert_eq!(store.session(id).unwrap().history_revision, 1);
    }
}
#[test]
fn reader_admission_and_cancellation_release_only_after_worker_exit() {
    let (_dir, mut store, ids) = setup();
    let reader = store.lease_session(&ids[0]).unwrap();
    let job = store
        .begin_maintenance(Some(&ids[0]), 10, false, 1)
        .unwrap();
    assert!(matches!(store.next_maintenance_work(2), Err(Error::Busy)));
    drop(reader);
    let work = store.next_maintenance_work(2).unwrap().unwrap();
    let cancellation = work.cancellation();
    store.cancel_maintenance(&job.operation_id, 3).unwrap();
    cancellation.cancel();
    assert!(matches!(store.lease_session(&ids[0]), Err(Error::Busy)));
    let job = store.complete_maintenance_work(work.run(4), 5).unwrap();
    assert_eq!(job.state, "cancelled");
    assert_eq!(job.completed_sessions, 0);
    assert_eq!(store.session(&ids[0]).unwrap().history_revision, 0);
    assert!(store.lease_session(&ids[0]).is_ok());
}
#[test]
fn cancellation_is_not_resurrected_after_process_restart() {
    let (dir, mut store, ids) = setup();
    let job = store
        .begin_maintenance(Some(&ids[0]), 300, false, 1)
        .unwrap();
    let work = store.next_maintenance_work(2).unwrap().unwrap();
    store.cancel_maintenance(&job.operation_id, 3).unwrap();
    drop(work);
    drop(store);
    let mut store = ProductStore::open(dir.path()).unwrap();
    assert_eq!(store.maintenance().unwrap().unwrap().state, "cancelled");
    assert!(store.next_maintenance_work(4).unwrap().is_none());
}
#[test]
fn committed_prune_replays_idempotently_after_catalog_failure() {
    let (dir, mut store, ids) = setup();
    store
        .begin_maintenance(Some(&ids[0]), 10, false, 1)
        .unwrap();
    let work = store.next_maintenance_work(2).unwrap().unwrap();
    let cursor = store
        .sessions(None, 1, None, true)
        .unwrap()
        .next_cursor
        .unwrap();
    let completion = work.run(3);
    let path = dir.file(&format!("session-{}.sqlite", ids[0]));
    let marker = SessionReader::open(&path).unwrap().marker().unwrap();
    assert_eq!(marker.history_revision, 1);
    let catalog = Connection::open(dir.file("catalog.sqlite")).unwrap();
    catalog.execute_batch("CREATE TRIGGER fail_update BEFORE UPDATE ON session_catalog BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
    assert!(store.complete_maintenance_work(completion, 4).is_err());
    catalog.execute_batch("DROP TRIGGER fail_update").unwrap();
    drop(catalog);
    drop(store);
    let mut store = ProductStore::open(dir.path()).unwrap();
    assert!(store.sessions(None, 1, Some(&cursor), true).is_err());
    let work = store.next_maintenance_work(5).unwrap().unwrap();
    let job = store.complete_maintenance_work(work.run(6), 7).unwrap();
    assert_eq!(job.completed_sessions, 1);
    assert_eq!(job.skipped_sessions, 1);
    assert_eq!(SessionReader::open(path).unwrap().marker().unwrap(), marker);
    assert!(store.next_maintenance_work(8).unwrap().is_none());
}
#[test]
fn preview_changes_no_session_data_and_new_job_rejects_active_sessions() {
    let (_dir, mut store, ids) = setup();
    let before = store.session(&ids[0]).unwrap();
    store.begin_maintenance(Some(&ids[0]), 10, true, 1).unwrap();
    let work = store.next_maintenance_work(2).unwrap().unwrap();
    let job = store.complete_maintenance_work(work.run(3), 4).unwrap();
    assert!(job.retained_samples < before.stored_sample_count);
    assert_eq!(store.session(&ids[0]).unwrap(), before);
    assert!(store.next_maintenance_work(5).unwrap().is_none());
    let board = store.boards().unwrap().remove(0);
    let active = store
        .create_session(&board.saved_device_id, 1, u64::MAX)
        .unwrap();
    assert_eq!(
        store.begin_maintenance(Some(&active.session_id), 10, false, 6),
        Err(Error::ActiveSession)
    );
}
