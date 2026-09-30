use float_view_product_store::{
    Error, ProductStore, RoutingHint, Settings, board_identity, limits,
};
use rusqlite::Connection;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let parent = std::env::temp_dir().canonicalize().unwrap();
        let path = parent.join(format!(
            "float-view-products-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn open(&self) -> ProductStore {
        ProductStore::open(&self.0).unwrap()
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn current_storage_reopens_without_changing_revision_or_sql() {
    let root = Root::new();
    let mut store = root.open();
    assert_eq!(store.settings().unwrap(), Settings::default());
    let value = store
        .update_settings(
            0,
            r#"{"minimumSessionDistance":42,"locationRecordingEnabled":true}"#,
        )
        .unwrap();
    drop(store);
    assert_eq!(root.open().settings().unwrap(), value);
    assert_eq!(
        fs::read_to_string(root.0.join(".schema-version")).unwrap(),
        "13"
    );
    let conn = Connection::open(root.0.join("catalog.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
}
#[test]
fn outdated_storage_resets_only_owned_files() {
    let root = Root::new();
    fs::write(root.0.join(".schema-version"), "11").unwrap();
    fs::write(root.0.join("catalog.sqlite"), "old database").unwrap();
    fs::write(root.0.join("session-old.sqlite"), "old session").unwrap();
    fs::write(root.0.join("personal-note.txt"), "keep").unwrap();
    let store = root.open();
    assert_eq!(store.settings().unwrap().revision, 0);
    assert!(!root.0.join("session-old.sqlite").exists());
    assert_eq!(
        fs::read_to_string(root.0.join("personal-note.txt")).unwrap(),
        "keep"
    );
}
#[test]
fn ordinary_corruption_and_schema_drift_never_reset() {
    let root = Root::new();
    drop(root.open());
    fs::write(root.0.join("catalog.sqlite"), "corruption").unwrap();
    assert!(matches!(ProductStore::open(&root.0), Err(Error::Corrupt)));
    assert_eq!(
        fs::read(root.0.join("catalog.sqlite")).unwrap(),
        b"corruption"
    );
    let root = Root::new();
    drop(root.open());
    let conn = Connection::open(root.0.join("catalog.sqlite")).unwrap();
    conn.execute_batch("CREATE TABLE unexpected(private TEXT)")
        .unwrap();
    drop(conn);
    assert!(matches!(ProductStore::open(&root.0), Err(Error::Corrupt)));
    assert_eq!(
        fs::read_to_string(root.0.join(".schema-version")).unwrap(),
        "13"
    );
}
#[test]
fn unavailable_or_escaping_root_fails_without_creating_parent() {
    let root = Root::new();
    assert!(ProductStore::open(&root.0.join("missing/child")).is_err());
    assert!(!root.0.join("missing").exists());
    assert!(matches!(
        ProductStore::open(&root.0.join("../escape")),
        Err(Error::UnsafePath)
    ));
    assert!(matches!(
        ProductStore::open(std::path::Path::new("relative")),
        Err(Error::UnsafePath)
    ));
    fs::write(root.0.join("file"), "not a directory").unwrap();
    assert!(matches!(
        ProductStore::open(&root.0.join("file")),
        Err(Error::UnsafePath)
    ));
}
#[cfg(unix)]
#[test]
fn symlinks_and_hardlinks_never_authorize_reset_or_catalog_access() {
    use std::os::unix::fs::symlink;
    let root = Root::new();
    let outside = Root::new();
    let secret = outside.0.join("secret");
    fs::write(&secret, "keep").unwrap();
    symlink(&secret, root.0.join("catalog.sqlite")).unwrap();
    assert!(matches!(
        ProductStore::open(&root.0),
        Err(Error::UnsafePath)
    ));
    assert_eq!(fs::read_to_string(&secret).unwrap(), "keep");
    fs::remove_file(root.0.join("catalog.sqlite")).unwrap();
    fs::hard_link(&secret, root.0.join("catalog.sqlite")).unwrap();
    assert!(matches!(
        ProductStore::open(&root.0),
        Err(Error::UnsafePath)
    ));
    let parent = Root::new();
    symlink(&outside.0, parent.0.join("alias")).unwrap();
    assert!(matches!(
        ProductStore::open(&parent.0.join("alias")),
        Err(Error::UnsafePath)
    ));
    fs::remove_file(root.0.join("catalog.sqlite")).unwrap();
    fs::create_dir(root.0.join("Exports")).unwrap();
    symlink(&secret, root.0.join("Exports/export.csv")).unwrap();
    fs::write(
        root.0.join("session-keep.sqlite"),
        "not deleted before validation",
    )
    .unwrap();
    assert!(matches!(
        ProductStore::open(&root.0),
        Err(Error::UnsafePath)
    ));
    assert!(root.0.join("session-keep.sqlite").exists());
}
#[test]
fn settings_validate_atomically_and_cas_across_connections() {
    let root = Root::new();
    let mut a = root.open();
    assert!(matches!(ProductStore::open(&root.0), Err(Error::Busy)));
    let first=a.update_settings(0,r#"{"idleProbeIntervalMs":-1,"ridingPersistIntervalMs":100000,"historyGapThresholdSeconds":0.01}"#).unwrap();
    assert_eq!(
        (
            first.idle_probe_interval_ms,
            first.riding_persist_interval_ms,
            first.history_gap_threshold_seconds
        ),
        (100, 10_000, Some(0.1))
    );
    assert_eq!(
        a.update_settings(0, "{}"),
        Err(Error::Conflict { revision: 1 })
    );
    for invalid in [
        r#"{"idleProbeIntervalMs":true}"#,
        r#"{"idleProbeIntervalMs":1.5}"#,
        r#"{"revision":3}"#,
        r#"{"themePreference":"auto"}"#,
        r#"{"minimumSessionDistance":null}"#,
        r#"{"locationRecordingEnabled":1}"#,
        r#"{"idleProbeIntervalMs":500,"unknown":true}"#,
    ] {
        assert_eq!(a.update_settings(1, invalid), Err(Error::InvalidInput));
        assert_eq!(a.settings().unwrap(), first);
    }
    let large = format!(
        "{{\"unknown\":\"{}\"}}",
        "x".repeat(limits::MAX_SETTINGS_BYTES)
    );
    assert_eq!(a.update_settings(1, &large), Err(Error::InvalidInput));
    assert_eq!(
        a.update_settings(1, r#"{"historyGapThresholdSeconds":null}"#)
            .unwrap()
            .revision,
        2
    );
    let external = Connection::open(root.0.join("catalog.sqlite")).unwrap();
    external
        .execute("UPDATE settings SET revision=3", [])
        .unwrap();
    assert_eq!(
        a.update_settings(2, "{}"),
        Err(Error::Conflict { revision: 3 })
    );
}
#[test]
fn sql_revision_is_authoritative_no_legacy_json_fallback() {
    let root = Root::new();
    drop(root.open());
    let conn = Connection::open(root.0.join("catalog.sqlite")).unwrap();
    conn.execute("UPDATE settings SET revision=7", []).unwrap();
    drop(conn);
    let mut store = root.open();
    assert_eq!(store.settings().unwrap().revision, 7);
    assert_eq!(store.update_settings(7, "{}").unwrap().revision, 8);
}
#[test]
fn canonical_identity_vectors_and_absent_identity() {
    let uuid: Vec<u8> = (1..=12).collect();
    assert_eq!(
        board_identity(Some(&uuid)).unwrap(),
        "vesc-v1-3ad3f33d03efba4de40aeb78f4513d6b5480595764b3a9e46dcc694025fa28ec"
    );
    for uuid in [
        None,
        Some([0u8; 12].as_slice()),
        Some([255u8; 12].as_slice()),
        Some([1u8; 11].as_slice()),
    ] {
        assert_eq!(board_identity(uuid), None);
    }
}
#[test]
fn imported_board_associates_on_explicit_save_without_leaking_private_identifiers() {
    let root = Root::new();
    let mut store = root.open();
    let uuid: Vec<u8> = (1..=12).collect();
    let identity = board_identity(Some(&uuid)).unwrap();
    let imported = store
        .import_board(
            &identity,
            Some(" Import "),
            r#"{"motor":"hub","seriesCount":16,"parallelCount":2}"#,
            1,
        )
        .unwrap();
    assert_eq!(imported.local_identifier, None);
    assert_eq!(imported.display_name(), "Import");
    let saved = store
        .ensure_board("private-platform-address", Some("BLE"), Some(&uuid), 2)
        .unwrap();
    assert_eq!(saved.saved_device_id, imported.saved_device_id);
    assert_eq!(saved.last_seen_at_ms, Some(2));
    assert_eq!(store.boards().unwrap().len(), 1);
    let serialized = serde_json::to_string(&saved).unwrap();
    assert!(!serialized.contains("private-platform-address"));
    assert!(!serialized.contains("0102030405060708090a0b0c"));
    assert!(
        store
            .profile(&saved.saved_device_id)
            .unwrap()
            .missing_fields()
            .is_empty()
    );
}
#[test]
fn revisions_names_profiles_and_route_invalidation_are_atomic() {
    let root = Root::new();
    let mut store = root.open();
    let b = store.ensure_board("local", None, None, 1).unwrap();
    let id = &b.saved_device_id;
    assert_eq!(
        store.profile(id).unwrap().missing_fields(),
        ["seriesCount", "parallelCount", "motor"]
    );
    store.update_board(id, 0, Some(&"a".repeat(128))).unwrap();
    assert_eq!(
        store.update_board(id, 1, Some(&"é".repeat(65))),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        store.update_board(id, 0, None),
        Err(Error::Conflict { revision: 1 })
    );
    assert_eq!(
        store
            .update_board(id, 1, Some("   "))
            .unwrap()
            .custom_display_name,
        None
    );
    let hint = RoutingHint {
        can_id: 42,
        controller_uuid: Some([1; 12]),
    };
    store.set_routing_hint(id, Some(&hint), 1).unwrap();
    assert_eq!(store.routing_hint(id).unwrap(), Some(hint.clone()));
    assert_eq!(
        store.update_profile(id, 0, r#"{"seriesCount":true,"preferredCanId":42}"#, 2),
        Err(Error::InvalidInput)
    );
    assert_eq!(store.routing_hint(id).unwrap(), Some(hint));
    let p = store
        .update_profile(
            id,
            0,
            r#"{"seriesCount":16,"parallelCount":2,"motor":"hub","preferredCanId":42}"#,
            2,
        )
        .unwrap();
    assert!(p.missing_fields().is_empty());
    assert!(store.routing_hint(id).unwrap().is_none());
    assert_eq!(
        store.update_profile(id, 0, "{}", 3),
        Err(Error::Conflict { revision: 1 })
    );
    assert_eq!(
        store.update_profile(id, 1, r#"{"preferredCanId":254}"#, 3),
        Err(Error::InvalidInput)
    );
    assert_eq!(store.profile(id).unwrap(), p);
}
#[test]
fn board_capacity_is_checked_before_insertion_but_allows_existing_board() {
    let root = Root::new();
    let mut store = root.open();
    for i in 0..limits::MAX_BOARDS {
        store
            .ensure_board(&format!("local-{i}"), None, None, 1)
            .unwrap();
    }
    assert_eq!(
        store.ensure_board("overflow", None, None, 1),
        Err(Error::Capacity)
    );
    store
        .ensure_board("local-0", Some("updated"), None, 2)
        .unwrap();
    assert_eq!(store.boards().unwrap().len(), limits::MAX_BOARDS);
}
#[test]
fn catalog_lock_wait_is_bounded_and_failed_mutation_does_not_advance_revision() {
    let root = Root::new();
    let mut store = root.open();
    let other = Connection::open(root.0.join("catalog.sqlite")).unwrap();
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    let start = std::time::Instant::now();
    assert_eq!(store.update_settings(0, "{}"), Err(Error::Busy));
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    other.execute_batch("ROLLBACK").unwrap();
    assert_eq!(store.settings().unwrap().revision, 0);
}

#[test]
fn empty_existing_catalog_is_corruption_not_a_reset() {
    let root = Root::new();
    drop(root.open());
    fs::write(root.0.join("catalog.sqlite"), []).unwrap();
    assert!(matches!(ProductStore::open(&root.0), Err(Error::Corrupt)));
    assert_eq!(
        fs::metadata(root.0.join("catalog.sqlite")).unwrap().len(),
        0
    );
}
#[cfg(unix)]
#[test]
fn protected_root_is_reported_without_reset() {
    use std::os::unix::fs::PermissionsExt;
    let root = Root::new();
    drop(root.open());
    let path = root.0.join("catalog.sqlite");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    let result = ProductStore::open(&root.0);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(result, Err(Error::Io)));
    assert_eq!(
        fs::read_to_string(root.0.join(".schema-version")).unwrap(),
        "13"
    );
    assert_eq!(root.open().settings().unwrap(), Settings::default());
}
fn tune_fixture() -> (
    float_view_product_store::TuneIdentity,
    float_view_product_store::TuneValues,
    float_view_refloat_configuration_codec::ConfigurationSchema,
) {
    use float_view_product_store::*;
    use float_view_refloat_configuration_codec::*;
    let field = ConfigurationField {
        name: "gain".into(),
        long_name: "Gain".into(),
        field_type: FieldType::Integer,
        wire_type: Some(WireType::Uint8),
        default_value: ConfigurationValue::Integer(2),
        minimum: Some(0.),
        maximum: Some(10.),
        step: Some(1.),
        precision: None,
        scale: None,
        suffix: String::new(),
        choices: vec![],
        description: String::new(),
    };
    let schema = ConfigurationSchema {
        fingerprint: "a".repeat(64),
        wire_signature: 42,
        fields: vec![field],
        groups: vec![],
    };
    let identity = TuneIdentity {
        package_name: "refloat".into(),
        package_version: "1.4.0".into(),
        schema_fingerprint: schema.fingerprint.clone(),
        wire_signature: schema.wire_signature,
    };
    (
        identity,
        [("gain".into(), TuneValue::Integer(2))].into(),
        schema,
    )
}
#[test]
fn tunes_require_exact_identity_complete_values_and_revision() {
    use float_view_product_store::*;
    let root = Root::new();
    let mut store = root.open();
    let (identity, values, schema) = tune_fixture();
    let tune = store
        .create_tune("Tune", None, &identity, &values, &schema, 1)
        .unwrap();
    assert_eq!(tune.revision, 1);
    assert_eq!(
        store.create_tune("Tune", None, &identity, &TuneValues::new(), &schema, 1),
        Err(Error::InvalidInput)
    );
    let wrong_values = [("gain".into(), TuneValue::Integer(11))].into();
    assert_eq!(
        store.create_tune("Tune", None, &identity, &wrong_values, &schema, 1),
        Err(Error::InvalidInput)
    );
    let mut wrong = identity.clone();
    wrong.wire_signature += 1;
    assert_eq!(
        store.create_tune("Tune", None, &wrong, &values, &schema, 1),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        store.update_tune(&tune.configuration_id, 0, "new", None, &values, &schema, 2),
        Err(Error::Conflict { revision: 1 })
    );
    let updated = store
        .update_tune(
            &tune.configuration_id,
            1,
            &"é".repeat(64),
            Some(&"x".repeat(1024)),
            &values,
            &schema,
            2,
        )
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(
        store.create_tune(&"é".repeat(65), None, &identity, &values, &schema, 2),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        store.delete_tune(&tune.configuration_id, 1),
        Err(Error::Conflict { revision: 2 })
    );
    store.delete_tune(&tune.configuration_id, 2).unwrap();
    assert_eq!(store.tune(&tune.configuration_id), Err(Error::NotFound));
}
#[test]
fn tune_keyset_cursor_binds_every_identity_component() {
    let root = Root::new();
    let mut store = root.open();
    let (identity, values, schema) = tune_fixture();
    for _ in 0..3 {
        store
            .create_tune("same time", None, &identity, &values, &schema, 1)
            .unwrap();
    }
    let first = store.tunes(&identity, 1, None).unwrap();
    let cursor = first.next_cursor.as_deref().unwrap();
    let second = store.tunes(&identity, 1, Some(cursor)).unwrap();
    assert_ne!(
        first.items[0].configuration_id,
        second.items[0].configuration_id
    );
    for part in 0..3 {
        let mut changed = identity.clone();
        match part {
            0 => changed.package_version = "other".into(),
            1 => changed.schema_fingerprint = "b".repeat(64),
            _ => changed.wire_signature += 1,
        };
        assert_eq!(
            store.tunes(&changed, 1, Some(cursor)),
            Err(Error::InvalidInput)
        );
    }
    assert_eq!(store.tunes(&identity, 51, None), Err(Error::InvalidInput));
    assert_eq!(
        store.tunes(&identity, 1, Some(&"a".repeat(513))),
        Err(Error::InvalidInput)
    );
}
fn cache_identity(version: &str) -> float_view_product_store::SchemaIdentity {
    float_view_product_store::SchemaIdentity {
        firmware_version: "6.05".into(),
        hardware_name: "controller".into(),
        package_version: version.into(),
        custom_index: 0,
        hardware_configuration_crc: 42,
    }
}
#[test]
fn cache_checks_size_digest_identity_and_bounded_eviction() {
    let root = Root::new();
    let mut store = root.open();
    let identity = cache_identity("1.4.0");
    let payload = vec![1; limits::MAX_SCHEMA_BYTES];
    store.store_schema(&identity, &payload).unwrap();
    assert_eq!(store.cached_schema(&identity).unwrap(), Some(payload));
    assert_eq!(
        store.store_schema(&identity, &vec![0; limits::MAX_SCHEMA_BYTES + 1]),
        Err(Error::InvalidInput)
    );
    let path = fs::read_dir(root.0.join("refloat-schema-cache"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    envelope["digest"] = "bad".into();
    fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
    assert_eq!(store.cached_schema(&identity).unwrap(), None);
    assert!(!path.exists());
    store.store_schema(&identity, &[1]).unwrap();
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    envelope["identity"]["hardwareName"] = "different".into();
    fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
    assert_eq!(store.cached_schema(&identity).unwrap(), None);
    for i in 0..10 {
        store
            .store_schema(&cache_identity(&format!("1.4.{i}")), &[1])
            .unwrap();
    }
    assert_eq!(
        fs::read_dir(root.0.join("refloat-schema-cache"))
            .unwrap()
            .count(),
        limits::MAX_SCHEMA_CACHE_ENTRIES
    );
    let mut disabled = identity;
    disabled.hardware_configuration_crc = 0;
    assert_eq!(store.cached_schema(&disabled).unwrap(), None);
}

#[test]
fn exclusive_root_owner_excludes_another_process_and_releases_on_exit() {
    const KEY: &str = "FLOAT_VIEW_TEST_LOCK_ROOT";
    if let Some(path) = std::env::var_os(KEY) {
        assert!(matches!(
            ProductStore::open(std::path::Path::new(&path)),
            Err(Error::Busy)
        ));
        return;
    }
    let root = Root::new();
    let owner = root.open();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "exclusive_root_owner_excludes_another_process_and_releases_on_exit",
        ])
        .env(KEY, &root.0)
        .status()
        .unwrap();
    assert!(status.success());
    drop(owner);
    assert!(ProductStore::open(&root.0).is_ok());
}

#[test]
fn crash_during_session_creation_removes_only_issued_temporary_names() {
    let root = Root::new();
    drop(root.open());
    let base = ".session-00112233-4455-6677-8899-aabbccddeeff.sqlite.float-view-12.3.tmp";
    for suffix in ["", "-wal", "-shm", "-journal"] {
        fs::write(
            root.0.join(format!("{base}{suffix}")),
            "interrupted creation",
        )
        .unwrap();
    }
    let unrelated = root.0.join(".session-personal.sqlite.float-view-12.3.tmp");
    fs::write(&unrelated, "keep").unwrap();
    drop(root.open());
    for suffix in ["", "-wal", "-shm", "-journal"] {
        assert!(!root.0.join(format!("{base}{suffix}")).exists());
    }
    assert_eq!(fs::read_to_string(unrelated).unwrap(), "keep");
}

#[test]
fn battery_profile_validation_boundaries_conflicts_and_reopen() {
    let root = Root::new();
    let mut store = root.open();
    let b = store
        .ensure_board("battery-board", None, Some(&[7; 12]), 1)
        .unwrap();
    let id = &b.saved_device_id;
    let original = store.profile(id).unwrap();
    for patch in [
        r#"{"batteryCellProfileId":"unknown"}"#,
        r#"{"batteryCellProfileId":4}"#,
        r#"{"batteryMinimumCellVoltageV":2.699}"#,
        r#"{"batteryMinimumCellVoltageV":4.2}"#,
        r#"{"batteryMinimumCellVoltageV":"NaN"}"#,
        r#"{"batteryMinimumCellVoltageV":1e999}"#,
    ] {
        assert!(
            store
                .update_profile(id, original.revision, patch, 2)
                .is_err()
        );
        assert_eq!(store.profile(id).unwrap(), original);
    }
    let mut revision = original.revision;
    for voltage in [2.7, 3., 4.199999] {
        let updated = store.update_profile(id, revision, &format!(r#"{{"batteryCellProfileId":"molicel-p45b","batteryMinimumCellVoltageV":{voltage}}}"#), 3).unwrap();
        assert_eq!(updated.missing_fields(), original.missing_fields());
        revision = updated.revision;
    }
    assert!(matches!(
        store.update_profile(id, 0, r#"{"batteryCellProfileId":"generic-linear"}"#, 4),
        Err(Error::Conflict { .. })
    ));
    let expected = store.profile(id).unwrap();
    drop(store);
    assert_eq!(root.open().profile(id).unwrap(), expected);
}

#[test]
fn preset_selection_is_atomic_and_overrides_survive_reopen_and_import() {
    let root = Root::new();
    let mut store = root.open();
    let b = store
        .ensure_board("preset", None, Some(&[9; 12]), 1)
        .unwrap();
    let id = &b.saved_device_id;
    let preset = "floatwheel-adv2-samsung-50s";
    let first = store
        .update_profile(
            id,
            0,
            &format!(r#"{{"batteryPresetId":"{preset}","motor":"keep"}}"#),
            2,
        )
        .unwrap();
    assert_eq!(first.values["seriesCount"], 20);
    assert_eq!(first.values["parallelCount"], 2);
    assert_eq!(first.values["batteryCapacityAh"], 10.);
    assert_eq!(first.values["batteryCellProfileId"], "samsung-50s");
    let lock = Connection::open(root.0.join("catalog.sqlite")).unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert_eq!(
        store.update_profile(
            id,
            1,
            r#"{"batteryPresetId":"fungineers-funwheel-x7s-molicel-p30b"}"#,
            3
        ),
        Err(Error::Busy)
    );
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(store.profile(id).unwrap(), first);

    for patch in [
        r#"{"batteryPresetId":"unknown"}"#,
        r#"{"batteryPresetId":7}"#,
        r#"{"batteryPresetId":"floatwheel-adv2-samsung-50s","batteryCapacityAh":null}"#,
        r#"{"batteryCapacityAh":0.09}"#,
        r#"{"batteryCapacityAh":200.1}"#,
        r#"{"batteryCapacityAh":"9"}"#,
    ] {
        assert!(store.update_profile(id, 1, patch, 3).is_err());
        assert_eq!(store.profile(id).unwrap(), first);
    }
    assert!(matches!(
        store.update_profile(
            id,
            0,
            r#"{"batteryPresetId":"fungineers-funwheel-x7s-molicel-p30b"}"#,
            3
        ),
        Err(Error::Conflict { .. })
    ));
    let override_ = store
        .update_profile(
            id,
            1,
            r#"{"batteryCapacityAh":8.7,"batteryMinimumCellVoltageV":3.2}"#,
            3,
        )
        .unwrap();
    assert_eq!(override_.values["batteryPresetId"], preset);
    let imported = store
        .import_board(
            "imported-preset",
            None,
            &serde_json::to_string(&override_.values).unwrap(),
            4,
        )
        .unwrap();
    assert_eq!(
        store.profile(&imported.saved_device_id).unwrap().values,
        override_.values
    );
    assert!(
        store
            .import_board(
                "invalid-preset",
                None,
                r#"{"batteryPresetId":"floatwheel-adv2-samsung-50s","seriesCount":18}"#,
                4
            )
            .is_err()
    );
    let cleared = store
        .update_profile(id, 2, r#"{"batteryCapacityAh":null}"#, 4)
        .unwrap();
    assert_eq!(cleared.values["batteryPresetId"], preset);
    drop(store);
    let mut store = root.open();
    assert_eq!(store.profile(id).unwrap(), cleared);
    let reset = store
        .update_profile(id, 3, &format!(r#"{{"batteryPresetId":"{preset}"}}"#), 5)
        .unwrap();
    assert_eq!(reset.values["batteryCapacityAh"], 10.);
    assert!(reset.values["batteryMinimumCellVoltageV"].is_null());
    assert_eq!(reset.values["motor"], "keep");
    let curve = store
        .update_profile(id, 4, r#"{"batteryCellProfileId":"generic-linear"}"#, 6)
        .unwrap();
    assert!(curve.values["batteryPresetId"].is_null());
    assert_eq!(curve.values["batteryCapacityAh"], 10.);
    let count = store
        .update_profile(id, 5, r#"{"parallelCount":3}"#, 7)
        .unwrap();
    assert!(count.values["batteryCapacityAh"].is_null());
    let count = store
        .update_profile(id, 6, r#"{"seriesCount":22,"batteryCapacityAh":0.1}"#, 8)
        .unwrap();
    assert_eq!(count.values["batteryCapacityAh"], 0.1);
    let count = store
        .update_profile(id, 7, r#"{"batteryCapacityAh":200}"#, 9)
        .unwrap();
    assert_eq!(count.values["batteryCapacityAh"], 200);
}
