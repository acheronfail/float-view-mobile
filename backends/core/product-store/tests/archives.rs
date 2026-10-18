#[path = "../../session-sqlite/tests/support/mod.rs"]
mod support;
use float_view_product_store::{
    Error, ExportFormat, FileCancellation, ImportSink, ProductStore, limits,
};
use float_view_refloat_telemetry_service::RideState;
use float_view_session_sqlite::{LocationObservation, SessionReader, SessionWriter};
use rusqlite::Connection;
use std::{fs, io::Read};
use support::{TestDirectory, sample};
fn setup() -> (TestDirectory, ProductStore, String) {
    let dir = TestDirectory::new();
    let mut store = ProductStore::open(dir.path()).unwrap();
    store
        .update_settings(0, r#"{"minimumSessionDistance":0}"#)
        .unwrap();
    let b = store
        .ensure_board("PRIVATE-LOCAL-ID", Some("Board"), Some(&[1; 12]), 1)
        .unwrap();
    store
        .update_profile(
            &b.saved_device_id,
            0,
            r#"{"batteryPresetId":"indy-speed-control-20s2p-gt-pack-molicel-p45b"}"#,
            1,
        )
        .unwrap();
    store
        .update_profile(
            &b.saved_device_id,
            1,
            r#"{"batteryMinimumCellVoltageV":3.23,"batteryCapacityAh":8.7}"#,
            1,
        )
        .unwrap();
    let appearance = float_view_product_store::BoardAppearance {
        rail: 4,
        hooks: 3,
        ..Default::default()
    };
    store
        .update_profile(
            &b.saved_device_id,
            2,
            &serde_json::json!({"appearance":appearance}).to_string(),
            1,
        )
        .unwrap();
    let s = store
        .create_session(&b.saved_device_id, 1, u64::MAX)
        .unwrap();
    let mut w = SessionWriter::open(store.session_path(&s.session_id).unwrap()).unwrap();
    let mut samples = [sample(1, 1000), sample(2, 12000), sample(3, 30000)];
    for s in &mut samples {
        s.ride_state = RideState::Riding;
        s.speed_kph = Some(36.);
        s.extensions = vec![
            float_view_refloat_telemetry_service::ExtensionValue {
                name: "extensionA".into(),
                value: 1.,
            },
            float_view_refloat_telemetry_service::ExtensionValue {
                name: "extensionB".into(),
                value: 2.,
            },
        ];
    }
    w.append_samples(&samples).unwrap();
    w.append_fault_history(
        &float_view_refloat_telemetry_service::FaultHistoryObservation {
            elapsed_ms: 30_000,
            observed_at_epoch_ms: 1_030_000,
            records: vec![
                float_view_refloat_telemetry_service::FaultRecord {
                    source_ticks: 298123,
                    active: true,
                    raw_code: 4,
                },
                float_view_refloat_telemetry_service::FaultRecord {
                    source_ticks: 298124,
                    active: false,
                    raw_code: 0,
                },
            ],
        },
    )
    .unwrap();
    w.append_locations(&[
        LocationObservation {
            observed_at_epoch_ms: 1_000_000,
            elapsed_ms: 0,
            latitude: 1.,
            longitude: 2.,
            accuracy_m: Some(3.),
        },
        LocationObservation {
            observed_at_epoch_ms: 1_002_000,
            elapsed_ms: 2000,
            latitude: 4.,
            longitude: 5.,
            accuracy_m: Some(6.),
        },
    ])
    .unwrap();
    w.close().unwrap();
    store
        .close_session(&s.session_id, 100_000, "disconnect")
        .unwrap();
    (dir, store, s.session_id)
}
fn sink(store: &ProductStore, path: &std::path::Path) -> ImportSink {
    let mut input = fs::File::open(path).unwrap();
    let mut sink = store.begin_import(input.metadata().unwrap().len()).unwrap();
    let mut buffer = [0; 65536];
    let mut offset = 0;
    loop {
        let n = input.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        sink.write(offset, &buffer[..n]).unwrap();
        offset += n as u64;
    }
    sink
}

#[test]
fn frozen_public_baseline_archive_imports_without_changing_settings() {
    let root = TestDirectory::new();
    let mut store = ProductStore::open(root.path()).unwrap();
    let bytes = include_bytes!("../../../fixtures/public-baseline/100/recording.floatview");
    let mut input = store.begin_import(bytes.len() as u64).unwrap();
    for (index, chunk) in bytes.chunks(65_536).enumerate() {
        input.write((index * 65_536) as u64, chunk).unwrap();
    }
    let settings = store.settings().unwrap();
    let imported = store
        .attach_import(input.validate(&FileCancellation::default()).unwrap())
        .unwrap();
    assert_eq!(store.settings().unwrap(), settings);
    let mut reader =
        SessionReader::open(store.session_path(&imported.session_id).unwrap()).unwrap();
    let boundary = reader.marker().unwrap();
    assert_eq!(boundary.recorded_location_count, 1);
    assert_eq!(reader.ride_markers((&boundary).into()).unwrap().len(), 1);
    assert!(store.boards().unwrap()[0].local_identifier.is_none());
}
#[test]
fn archive_roundtrip_preserves_portable_identity_and_uses_new_local_ids() {
    let (_dir, mut store, id) = setup();
    let preferred = store.boards().unwrap()[0].saved_device_id.clone();
    store
        .update_settings(
            store.settings().unwrap().revision,
            &serde_json::json!({"autoConnectSavedDeviceId":preferred}).to_string(),
        )
        .unwrap();
    let record = r#"{"v":1,"event":"summary","data":{"append_us":[10,100,200,300]}}"#;
    let mut diagnostics =
        float_view_session_sqlite::DiagnosticWriter::open(&store.session_path(&id).unwrap())
            .unwrap();
    diagnostics.append(&[record.into()]).unwrap();
    drop(diagnostics);
    let cancel = FileCancellation::default();
    let export = store
        .prepare_export(&id, ExportFormat::Archive)
        .unwrap()
        .run(&cancel)
        .unwrap();
    let db = Connection::open(export.path()).unwrap();
    let manifest: String = db
        .query_row(
            "SELECT value FROM float_view_archive_metadata WHERE key='session'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!manifest.contains("PRIVATE-LOCAL-ID"));
    assert!(!manifest.contains("autoConnectSavedDeviceId"));
    drop(db);
    let target = TestDirectory::new();
    let mut imported = ProductStore::open(target.path()).unwrap();
    let value = sink(&imported, export.path()).validate(&cancel).unwrap();
    let settings_before = imported.settings().unwrap();
    let session = imported.attach_import(value).unwrap();
    assert_eq!(imported.settings().unwrap(), settings_before);
    assert_ne!(session.session_id, id);
    let db = Connection::open(imported.session_path(&session.session_id).unwrap()).unwrap();
    assert_eq!(
        db.query_row("SELECT record_json FROM session_diagnostic", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        record
    );
    drop(db);
    let board = imported.boards().unwrap().remove(0);
    let profile = imported.profile(&board.saved_device_id).unwrap();
    assert_eq!(profile.values["batteryCellProfileId"], "molicel-p45b");
    assert_eq!(profile.values["batteryMinimumCellVoltageV"], 3.23);
    assert_eq!(profile.values["batteryCapacityAh"], 8.7);
    assert_eq!(profile.appearance().rail, 4);
    assert_eq!(profile.appearance().hooks, 3);
    assert_eq!(
        profile.values["batteryPresetId"],
        "indy-speed-control-20s2p-gt-pack-molicel-p45b"
    );
    assert_eq!(
        session.device_history_id,
        store.session(&id).unwrap().device_history_id
    );
    assert_eq!(session.recorded_sample_count, 3);
    assert_eq!(session.recorded_location_count, 2);
    assert!(imported.boards().unwrap()[0].local_identifier.is_none());
    assert_eq!(imported.boards().unwrap().len(), 1);
    let mut reader =
        SessionReader::open(imported.session_path(&session.session_id).unwrap()).unwrap();
    assert_eq!(reader.marker().unwrap().recorded_sample_count, 3);
    let boundary = reader.marker().unwrap();
    let events = reader
        .event_page(0, 30_000, 100, None, (&boundary).into())
        .unwrap();
    let faults: Vec<_> = events
        .rows
        .iter()
        .filter(|row| row.kind == "firmwareFaultHistory")
        .collect();
    assert_eq!(faults.len(), 2);
    let fault: serde_json::Value = serde_json::from_str(&faults[0].details_json).unwrap();
    assert_eq!(fault["rawCode"], 4);
    assert_eq!(fault["sourceTicks"], 298123);
    assert_eq!(fault["value"], "absoluteOverCurrent");
    drop(reader);
    let second = imported
        .attach_import(sink(&imported, export.path()).validate(&cancel).unwrap())
        .unwrap();
    assert_ne!(second.session_id, session.session_id);
    assert_eq!(imported.boards().unwrap().len(), 1);
}
#[test]
fn csv_columns_gps_privacy_ties_and_tolerance_match_native_encoding() {
    let (_dir, mut store, id) = setup();
    let cancel = FileCancellation::default();
    let export = store
        .prepare_export(&id, ExportFormat::Csv)
        .unwrap()
        .run(&cancel)
        .unwrap();
    let plain = fs::read_to_string(export.path()).unwrap();
    assert_eq!(plain.lines().next().unwrap().split(',').count(), 54);
    assert!(!plain.contains("gpsLatitude"));
    assert_eq!(
        plain.lines().next().unwrap(),
        include_str!("../../../../conformance/fixtures/session-csv-header.txt").trim_end()
    );
    assert!(
        plain
            .lines()
            .nth(1)
            .unwrap()
            .contains(r#""{""extensionA"":1.0,""extensionB"":2.0}""#)
    );
    drop(export);
    let revision = store.settings().unwrap().revision;
    store
        .update_settings(revision, r#"{"includeGpsLocationWhenExporting":true}"#)
        .unwrap();
    let export = store
        .prepare_export(&id, ExportFormat::Csv)
        .unwrap()
        .run(&cancel)
        .unwrap();
    let text = fs::read_to_string(export.path()).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert!(lines[0].ends_with(",gpsAccuracy,gpsLongitude,gpsLatitude"));
    assert!(lines[1].ends_with(",3.0,2.0,1.0"));
    assert!(lines[2].ends_with(",6.0,5.0,4.0"));
    assert!(lines[3].ends_with(",,,"));
}
#[test]
fn transfer_bounds_offsets_and_cancellation_cleanup() {
    let (dir, store, id) = setup();
    assert!(matches!(
        store.begin_import(limits::MAX_IMPORT_BYTES + 1),
        Err(Error::InvalidInput)
    ));
    let mut a = store.begin_import(limits::MAX_IMPORT_BYTES).unwrap();
    let b = store.begin_import(1).unwrap();
    assert!(matches!(store.begin_import(1), Err(Error::Capacity)));
    assert_eq!(a.write(1, &[0]), Err(Error::InvalidInput));
    assert_eq!(a.write(0, &vec![0; 65537]), Err(Error::InvalidInput));
    a.write(0, &[0; 65536]).unwrap();
    drop(a);
    drop(b);
    let cancel = FileCancellation::default();
    let work = store.prepare_export(&id, ExportFormat::Archive).unwrap();
    cancel.cancel();
    assert!(matches!(work.run(&cancel), Err(Error::Cancelled)));
    assert_eq!(fs::read_dir(dir.file("Exports")).unwrap().count(), 0);
    assert!(fs::read_dir(dir.path()).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("import-")
    }));
}
#[test]
fn malformed_schema_metadata_and_attachment_failure_leave_no_catalog_changes() {
    let (_dir, store, id) = setup();
    let cancel = FileCancellation::default();
    for sql in [
        "CREATE TRIGGER hostile AFTER INSERT ON sample BEGIN DELETE FROM sample; END",
        "UPDATE float_view_archive_metadata SET value='{}' WHERE key='session'",
        "UPDATE float_view_archive_metadata SET value='99' WHERE key='format_version'",
        "UPDATE float_view_archive_metadata SET value=json_set(value,'$.boardProfileSnapshot.batteryCellProfileId','unknown') WHERE key='session'",
        "UPDATE float_view_archive_metadata SET value=json_set(value,'$.boardProfileSnapshot.batteryMinimumCellVoltageV',4.2) WHERE key='session'",
    ] {
        let export = store
            .prepare_export(&id, ExportFormat::Archive)
            .unwrap()
            .run(&cancel)
            .unwrap();
        let db = Connection::open(export.path()).unwrap();
        db.execute_batch(sql).unwrap();
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
        drop(db);
        let target = TestDirectory::new();
        let imported = ProductStore::open(target.path()).unwrap();
        assert!(sink(&imported, export.path()).validate(&cancel).is_err());
        assert!(imported.boards().unwrap().is_empty());
    }
    let export = store
        .prepare_export(&id, ExportFormat::Archive)
        .unwrap()
        .run(&cancel)
        .unwrap();
    let target = TestDirectory::new();
    let mut imported = ProductStore::open(target.path()).unwrap();
    let value = sink(&imported, export.path()).validate(&cancel).unwrap();
    let db = Connection::open(target.file("catalog.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER fail BEFORE INSERT ON session_catalog BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
    assert!(imported.attach_import(value).is_err());
    assert!(imported.boards().unwrap().is_empty());
    assert!(
        imported
            .sessions(None, 100, None, false)
            .unwrap()
            .items
            .is_empty()
    );
    assert!(fs::read_dir(target.path()).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("session-")
    }));
}
#[test]
fn startup_reclaims_staging_and_unattached_owned_session_only() {
    let dir = TestDirectory::new();
    drop(ProductStore::open(dir.path()).unwrap());
    fs::write(dir.file("import-abandoned.floatview.tmp"), b"partial").unwrap();
    fs::write(dir.file("session-abandoned.sqlite"), b"unattached").unwrap();
    fs::write(dir.file("notes.txt"), b"keep").unwrap();
    let store = ProductStore::open(dir.path()).unwrap();
    assert!(!dir.file("session-abandoned.sqlite").exists());
    assert!(!dir.file("import-abandoned.floatview.tmp").exists());
    assert_eq!(fs::read(dir.file("notes.txt")).unwrap(), b"keep");
    drop(store);
}

#[test]
fn exact_import_byte_limit_is_streamed_and_foreign_handle_is_rejected() {
    let (_source_dir, source, id) = setup();
    let cancel = FileCancellation::default();
    let export = source
        .prepare_export(&id, ExportFormat::Archive)
        .unwrap()
        .run(&cancel)
        .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(export.path())
        .unwrap()
        .set_len(limits::MAX_IMPORT_BYTES)
        .unwrap();
    let target_dir = TestDirectory::new();
    let target = ProductStore::open(target_dir.path()).unwrap();
    let validated = sink(&target, export.path()).validate(&cancel).unwrap();
    let foreign_dir = TestDirectory::new();
    let mut foreign = ProductStore::open(foreign_dir.path()).unwrap();
    assert!(matches!(
        foreign.attach_import(validated),
        Err(Error::InvalidInput)
    ));
    assert!(foreign.boards().unwrap().is_empty());
    assert!(
        target
            .sessions(None, 100, None, false)
            .unwrap()
            .items
            .is_empty()
    );
}
#[test]
fn source_delete_waits_for_export_while_another_session_keeps_recording() {
    let (_dir, mut store, id) = setup();
    let cancel = FileCancellation::default();
    let work = store.prepare_export(&id, ExportFormat::Archive).unwrap();
    let path = store.session_path(&id).unwrap();
    store.delete_session(&id).unwrap();
    assert!(path.exists());
    let board = store.boards().unwrap().remove(0);
    let active = store
        .create_session(&board.saved_device_id, 2, u64::MAX)
        .unwrap();
    let mut writer = SessionWriter::open(store.session_path(&active.session_id).unwrap()).unwrap();
    let thread = std::thread::spawn(move || work.run(&cancel).unwrap());
    writer.append_samples(&[sample(1, 0)]).unwrap();
    assert_eq!(writer.marker().unwrap().recorded_sample_count, 1);
    writer.close().unwrap();
    let export = thread.join().unwrap();
    assert!(export.path().exists());
    store.cleanup_deletes().unwrap();
    assert!(!path.exists());
    drop(export);
}

#[test]
fn pruned_archives_preserve_recorded_location_boundary_after_rows_are_removed() {
    let (_dir, mut store, id) = setup();
    let path = store.session_path(&id).unwrap();
    let mut writer = SessionWriter::open(&path).unwrap();
    let more: Vec<_> = (4..=53)
        .map(|i| {
            let mut v = sample(i, 30_000 + (i - 3) * 100);
            v.ride_state = RideState::Riding;
            v
        })
        .collect();
    writer.append_samples(&more).unwrap();
    writer
        .append_locations(&[LocationObservation {
            observed_at_epoch_ms: 1_050_000,
            elapsed_ms: 50_000,
            latitude: 7.,
            longitude: 8.,
            accuracy_m: None,
        }])
        .unwrap();
    writer.close().unwrap();
    store.reconcile_session(&id).unwrap();
    store.begin_maintenance(Some(&id), 10, false, 1).unwrap();
    let work = store.next_maintenance_work(2).unwrap().unwrap();
    store.complete_maintenance_work(work.run(3), 4).unwrap();
    let marker = SessionReader::open(&path).unwrap().marker().unwrap();
    assert!(marker.stored_location_count < marker.recorded_location_count);
    let cancel = FileCancellation::default();
    let export = store
        .prepare_export(&id, ExportFormat::Archive)
        .unwrap()
        .run(&cancel)
        .unwrap();
    let target = TestDirectory::new();
    let mut dest = ProductStore::open(target.path()).unwrap();
    let value = sink(&dest, export.path()).validate(&cancel).unwrap();
    let imported = dest.attach_import(value).unwrap();
    assert_eq!(imported.history_revision, 1);
    assert_eq!(imported.recorded_location_count, 3);
    assert_eq!(imported.stored_location_count, 2);
}

#[test]
fn a_live_worker_lease_prevents_a_second_owner_from_running_startup_cleanup() {
    let (dir, source, id) = setup();
    let export = source.prepare_export(&id, ExportFormat::Archive).unwrap();
    drop(source);
    assert!(matches!(ProductStore::open(dir.path()), Err(Error::Busy)));
    let file = export.run(&FileCancellation::default()).unwrap();
    assert!(matches!(ProductStore::open(dir.path()), Err(Error::Busy)));
    drop(file);
    assert!(ProductStore::open(dir.path()).is_ok());
}

#[test]
fn idle_marker_only_session_survives_recovery_and_archive_roundtrip() {
    let dir = TestDirectory::new();
    let mut store = ProductStore::open(dir.path()).unwrap();
    let board = store
        .ensure_board("marker-board", None, Some(&[8; 12]), 1)
        .unwrap();
    let session = store
        .create_session(&board.saved_device_id, 1, u64::MAX)
        .unwrap();
    let path = store.session_path(&session.session_id).unwrap();
    let mut writer = SessionWriter::open(&path).unwrap();
    writer
        .append_locations(&[LocationObservation {
            elapsed_ms: 0,
            observed_at_epoch_ms: 1,
            latitude: 2.,
            longitude: 3.,
            accuracy_m: Some(4.),
        }])
        .unwrap();
    let (marker, _) = writer
        .append_ride_marker("00000000-0000-4000-8000-000000000001", 500, 501, true)
        .unwrap();
    writer.close().unwrap();
    drop(store);
    let store = ProductStore::open(dir.path()).unwrap();
    assert_eq!(
        store.session(&session.session_id).unwrap().state,
        "interrupted"
    );
    let cancel = FileCancellation::default();
    let export = store
        .prepare_export(&session.session_id, ExportFormat::Archive)
        .unwrap()
        .run(&cancel)
        .unwrap();
    let target = TestDirectory::new();
    let mut target_store = ProductStore::open(target.path()).unwrap();
    let imported = target_store
        .attach_import(
            sink(&target_store, export.path())
                .validate(&cancel)
                .unwrap(),
        )
        .unwrap();
    let mut reader =
        SessionReader::open(target_store.session_path(&imported.session_id).unwrap()).unwrap();
    let boundary = reader.marker().unwrap();
    assert_eq!(
        reader.ride_markers((&boundary).into()).unwrap(),
        vec![marker]
    );
    drop(reader);
    let archive = Connection::open(export.path()).unwrap();
    archive
        .execute(
            "UPDATE session_event SET details_json='{}' WHERE kind='session_marker'",
            [],
        )
        .unwrap();
    drop(archive);
    assert!(
        sink(&target_store, export.path())
            .validate(&cancel)
            .is_err()
    );
    assert!(target_store.session(&imported.session_id).is_ok());
    target_store.delete_session(&imported.session_id).unwrap();
    assert!(target_store.session(&imported.session_id).is_err());
}
