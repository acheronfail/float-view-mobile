mod support;

use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use float_view_refloat_telemetry_service::{ExtensionValue, PackageState, RideState};
use float_view_session_history_kernel::GapPolicy;
use float_view_session_sqlite::{
    FieldSource, LocationObservation, PublishedBoundary, ReaderError, SampleKey, SessionMetadata,
    SessionReader, SessionWriter,
};
use rusqlite::Connection;
use support::{TestDirectory, sample};

const REQUESTS: &str = include_str!("../../../fixtures/session-sqlite/reader-requests.tsv");
const EXPECTED: &str = include_str!("../../../fixtures/session-sqlite/reader-expected.tsv");

fn metadata() -> SessionMetadata {
    SessionMetadata {
        expected_riding_persistence_ms: 100,
        expected_idle_persistence_ms: 1_000,
        committed_at_epoch_ms: 999_000,
    }
}

fn fixture(directory: &TestDirectory, name: &str) -> (std::path::PathBuf, PublishedBoundary) {
    let path = directory.file(name);
    let mut writer = SessionWriter::create(&path, metadata()).expect("create fixture");
    let mut samples = [
        sample(1, 0),
        sample(2, 1_000),
        sample(3, 12_000),
        sample(4, 12_000),
    ];
    samples[0].speed_kph = Some(10.0);
    samples[0].package_state = Some(PackageState::Ready);
    samples[1].speed_kph = Some(20.0);
    samples[1].package_state = Some(PackageState::Running);
    samples[1].ride_state = RideState::Riding;
    samples[1].extensions.push(extension("fixtureValue", 2.5));
    samples[2].ride_state = RideState::Riding;
    samples[2].extensions.push(extension("fixtureValue", 3.5));
    samples[3].speed_kph = Some(5.0);
    samples[3].package_state = Some(PackageState::Ready);
    samples[3].extensions.push(extension("fixtureValue", 4.5));
    writer.append_samples(&samples).expect("write samples");
    writer
        .append_locations(&[
            location(0, 1.0, 2.0),
            location(10_000, 1.1, 2.1),
            location(20_001, 1.2, 2.2),
        ])
        .expect("write locations");
    let marker = writer.marker().expect("marker");
    writer.close().expect("close fixture");
    let boundary = PublishedBoundary::from(&marker);
    (path, boundary)
}

#[test]
fn literal_requests_project_metadata_fields_pages_and_reduction() {
    assert_eq!(data_lines(REQUESTS).len(), 8);
    assert_eq!(data_lines(EXPECTED).len(), 8);
    let directory = TestDirectory::new();
    let (path, boundary) = fixture(&directory, "reader.sqlite3");
    let mut reader = SessionReader::open(&path).expect("open reader");

    let metadata = reader.metadata().expect("metadata");
    assert_eq!(metadata.expected_riding_persistence_ms, 100);
    assert_eq!(metadata.expected_idle_persistence_ms, 1_000);
    assert_eq!(metadata.retained_resolution_seconds, 0);
    assert_eq!(reader.marker().expect("marker").history_revision, 0);
    let fields = reader.fields().expect("fields");
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.name.as_str(), field.source))
            .collect::<Vec<_>>(),
        vec![
            ("fixtureValue", FieldSource::Extension),
            ("speedKph", FieldSource::Stable)
        ]
    );

    let selected = vec!["speedKph".to_owned(), "fixtureValue".to_owned()];
    let first = reader
        .sample_page(&selected, 0, 30_000, 2, None, boundary)
        .expect("first sample page");
    assert_eq!(first.rows.len(), 2);
    assert_eq!(first.rows[0].values, vec![Some(10.0), None]);
    assert_eq!(
        first.next,
        Some(SampleKey {
            elapsed_ms: 1_000,
            sequence: 2
        })
    );
    let second = reader
        .sample_page(&selected, 0, 30_000, 2, first.next, boundary)
        .expect("second sample page");
    assert_eq!(second.rows.len(), 2);
    assert_eq!(
        second.rows[0].key,
        SampleKey {
            elapsed_ms: 12_000,
            sequence: 3
        }
    );
    assert_eq!(second.next, None);

    let reduced = reader
        .reduce_series(&selected, 0, 30_000, 2, GapPolicy::Automatic, boundary)
        .expect("series");
    assert_eq!(reduced.source_rows, 4);
    assert_eq!(reduced.columns.len(), 2);
    assert!(
        reduced
            .columns
            .iter()
            .all(|column| column.values.len() <= 8)
    );

    let events = reader
        .event_page(0, 30_000, 2, None, boundary)
        .expect("events");
    assert_eq!(events.rows.len(), 2);
    assert_eq!(events.next, Some(events.rows[1].key));
    assert!(events.rows.iter().all(|event| event.kind == "packageState"
        || event.kind == "alertReason"
        || event.kind == "firmwareFault"));

    let locations = reader
        .location_page(0, 30_000, 2, None, boundary)
        .expect("locations");
    assert_eq!(locations.rows.len(), 2);
    assert_eq!(locations.next_location_sequence, Some(2));
    let locations = reader
        .location_page(0, 30_000, 2, locations.next_location_sequence, boundary)
        .expect("continued locations");
    assert_eq!(locations.rows.len(), 1);
    assert_eq!(locations.next_location_sequence, None);
    reader.close().expect("close reader");
}

#[test]
fn validates_boundaries_fields_ranges_pages_and_malformed_durable_values() {
    let directory = TestDirectory::new();
    let (path, boundary) = fixture(&directory, "bounds.sqlite3");
    let mut reader = SessionReader::open(&path).expect("open reader");
    let fields = vec!["speedKph".to_owned()];
    for invalid in [
        PublishedBoundary {
            history_revision: 1,
            ..boundary
        },
        PublishedBoundary {
            sample_sequence: boundary.sample_sequence + 1,
            ..boundary
        },
        PublishedBoundary {
            event_sequence: boundary.event_sequence + 1,
            ..boundary
        },
        PublishedBoundary {
            location_sequence: boundary.location_sequence + 1,
            ..boundary
        },
    ] {
        assert_eq!(
            reader
                .sample_page(&fields, 0, 30_000, 1, None, invalid)
                .expect_err("boundary"),
            ReaderError::InvalidInput
        );
    }
    for invalid_fields in [
        Vec::new(),
        vec!["speedKph".to_owned(), "speedKph".to_owned()],
        vec!["unknown".to_owned()],
        (0..17).map(|index| format!("f{index}")).collect(),
    ] {
        assert_eq!(
            reader
                .sample_page(&invalid_fields, 0, 30_000, 1, None, boundary)
                .expect_err("fields"),
            ReaderError::InvalidInput
        );
    }
    assert_eq!(
        reader
            .sample_page(&fields, 2, 1, 1, None, boundary)
            .expect_err("range"),
        ReaderError::InvalidInput
    );
    assert_eq!(
        reader
            .sample_page(&fields, 0, 1, 0, None, boundary)
            .expect_err("limit"),
        ReaderError::InvalidInput
    );
    assert_eq!(
        reader
            .event_page(0, 1, 501, None, boundary)
            .expect_err("event limit"),
        ReaderError::InvalidInput
    );
    assert_eq!(
        reader
            .location_page(0, 1, 2_001, None, boundary)
            .expect_err("location limit"),
        ReaderError::InvalidInput
    );
    reader.close().expect("close reader");

    Connection::open(&path)
        .expect("open mutation connection")
        .execute(
            "UPDATE sample SET speed_kph = ? WHERE sequence = 1",
            [f64::INFINITY],
        )
        .expect("write malformed numeric");
    let mut reader = SessionReader::open(&path).expect("reopen reader");
    assert_eq!(
        reader
            .sample_page(&fields, 0, 30_000, 10, None, boundary)
            .expect_err("nonfinite"),
        ReaderError::Incompatible
    );
}

#[test]
fn rejects_missing_corrupt_incompatible_and_malformed_reader_state_without_mutation() {
    let directory = TestDirectory::new();
    assert_eq!(
        SessionReader::open(directory.file("missing.sqlite3")).expect_err("missing"),
        ReaderError::NotFound
    );
    let corrupt = directory.file("corrupt.sqlite3");
    fs::write(&corrupt, b"private payload").expect("write corrupt");
    assert_eq!(
        SessionReader::open(&corrupt).expect_err("corrupt"),
        ReaderError::Corrupt
    );
    assert_eq!(
        fs::read(&corrupt).expect("unchanged corrupt"),
        b"private payload"
    );

    let incompatible = directory.file("incompatible.sqlite3");
    Connection::open(&incompatible)
        .expect("open")
        .execute("CREATE TABLE unrelated(value)", [])
        .expect("schema");
    let before = fs::read(&incompatible).expect("before");
    assert_eq!(
        SessionReader::open(&incompatible).expect_err("incompatible"),
        ReaderError::Incompatible
    );
    assert_eq!(fs::read(&incompatible).expect("after"), before);

    let locked = directory.file("locked.sqlite3");
    SessionWriter::create(&locked, metadata())
        .expect("create locked database")
        .close()
        .expect("close locked database");
    let locker = Connection::open(&locked).expect("open locker");
    locker
        .execute_batch("PRAGMA journal_mode = DELETE; BEGIN EXCLUSIVE;")
        .expect("hold exclusive lock");
    assert_eq!(
        SessionReader::open(&locked).expect_err("locked reader"),
        ReaderError::Busy
    );
    locker.execute_batch("ROLLBACK;").expect("release lock");
}

#[test]
fn active_wal_writer_advances_while_reader_uses_a_published_boundary() {
    let directory = TestDirectory::new();
    let (path, old_boundary) = fixture(&directory, "wal.sqlite3");
    let mut reader = SessionReader::open(&path).expect("open reader");
    let mut writer = SessionWriter::open(&path).expect("open writer");
    let (sent, received) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut next = sample(5, 13_000);
        next.speed_kph = Some(30.0);
        let marker = writer.append_samples(&[next]).expect("advance writer");
        sent.send(marker).expect("send marker");
    });
    let old = reader
        .sample_page(&["speedKph".to_owned()], 0, 30_000, 20, None, old_boundary)
        .expect("old published read");
    assert_eq!(old.rows.len(), 4);
    let marker = received
        .recv_timeout(Duration::from_secs(2))
        .expect("writer progress");
    handle.join().expect("writer thread");
    assert_eq!(marker.committed_sample_sequence, 5);
    let current = PublishedBoundary::from(&reader.marker().expect("new marker"));
    let new = reader
        .sample_page(&["speedKph".to_owned()], 0, 30_000, 20, None, current)
        .expect("new read");
    assert_eq!(new.rows.len(), 5);
}

#[test]
fn rejects_source_ceiling_and_malformed_event_or_location_rows_without_partial_results() {
    let directory = TestDirectory::new();
    let (large_path, large_boundary) = large_database(&directory, "ceiling.sqlite3", 200_001);
    let mut reader = SessionReader::open(&large_path).expect("large reader");
    assert_eq!(
        reader
            .reduce_series(
                &["speedKph".to_owned()],
                0,
                200_001,
                1_024,
                GapPolicy::Automatic,
                large_boundary,
            )
            .expect_err("source ceiling"),
        ReaderError::InvalidInput
    );

    let (event_path, boundary) = fixture(&directory, "malformed-event.sqlite3");
    Connection::open(&event_path)
        .expect("event mutation")
        .execute(
            "UPDATE session_event SET details_json = '[]' WHERE sequence = 1",
            [],
        )
        .expect("malform event");
    let mut reader = SessionReader::open(&event_path).expect("event reader");
    assert_eq!(
        reader
            .event_page(0, 30_000, 500, None, boundary)
            .expect_err("malformed details"),
        ReaderError::Incompatible
    );

    let (location_path, boundary) = fixture(&directory, "malformed-location.sqlite3");
    Connection::open(&location_path)
        .expect("location mutation")
        .execute(
            "UPDATE location SET monotonic_elapsed_ms = 5000 WHERE location_sequence = 3",
            [],
        )
        .expect("malform location ordering");
    let mut reader = SessionReader::open(&location_path).expect("location reader");
    assert_eq!(
        reader
            .location_page(0, 30_000, 2_000, None, boundary)
            .expect_err("malformed location order"),
        ReaderError::Incompatible
    );
}

#[test]
fn interruption_is_payload_free_and_reader_remains_reusable() {
    let directory = TestDirectory::new();
    let (path, boundary) = large_database(&directory, "interrupt.sqlite3", 200_000);
    let mut reader = SessionReader::open(&path).expect("reader");
    let interrupter = reader.interrupter();
    let interrupt_thread = thread::spawn(move || {
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(1));
            interrupter.interrupt();
        }
    });
    let result = reader.reduce_series(
        &["speedKph".to_owned()],
        0,
        200_000,
        1_024,
        GapPolicy::Automatic,
        boundary,
    );
    interrupt_thread.join().expect("interrupt thread");
    assert_eq!(result.expect_err("cancelled scan"), ReaderError::Cancelled);
    assert_eq!(
        reader
            .marker()
            .expect("reader reusable")
            .committed_sample_sequence,
        200_000
    );
}

fn large_database(
    directory: &TestDirectory,
    name: &str,
    count: u64,
) -> (std::path::PathBuf, PublishedBoundary) {
    let path = directory.file(name);
    SessionWriter::create(&path, metadata())
        .expect("create large database")
        .close()
        .expect("close large writer");
    let mut database = Connection::open(&path).expect("open large database");
    let transaction = database.transaction().expect("large transaction");
    {
        let mut insert = transaction
            .prepare("INSERT INTO sample(sequence, observed_at_ms, monotonic_elapsed_ms, ride_state, speed_kph) VALUES (?, ?, ?, 'riding', ?)")
            .expect("prepare insert");
        for sequence in 1..=count {
            insert
                .execute(rusqlite::params![
                    i64::try_from(sequence).expect("sequence"),
                    1_000_000_i64 + i64::try_from(sequence).expect("observed"),
                    i64::try_from(sequence - 1).expect("elapsed"),
                    sequence as f64,
                ])
                .expect("insert large row");
        }
    }
    transaction
        .execute(
            "INSERT INTO session_numeric_field(field_name, source, first_sequence) VALUES ('speedKph', 'stable', 1)",
            [],
        )
        .expect("field evidence");
    transaction
        .execute(
            "UPDATE commit_marker SET committed_sample_sequence = ?, recorded_sample_count = ?, stored_sample_count = ?, committed_elapsed_ms = ?, committed_at_ms = ? WHERE singleton = 1",
            rusqlite::params![
                i64::try_from(count).expect("count"),
                i64::try_from(count).expect("count"),
                i64::try_from(count).expect("count"),
                i64::try_from(count - 1).expect("elapsed"),
                1_000_000_i64 + i64::try_from(count).expect("commit time"),
            ],
        )
        .expect("large marker");
    transaction.commit().expect("commit large database");
    drop(database);
    (
        path,
        PublishedBoundary {
            history_revision: 0,
            sample_sequence: count,
            event_sequence: 0,
            location_sequence: 0,
        },
    )
}

fn extension(name: &str, value: f64) -> ExtensionValue {
    ExtensionValue {
        name: name.to_owned(),
        value,
    }
}

fn location(elapsed_ms: u64, latitude: f64, longitude: f64) -> LocationObservation {
    LocationObservation {
        observed_at_epoch_ms: 1_000_000 + elapsed_ms as i64,
        elapsed_ms,
        latitude,
        longitude,
        accuracy_m: Some(4.0),
    }
}

fn data_lines(input: &str) -> Vec<&str> {
    input
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

#[test]
fn cancellation_flag_is_sticky_and_does_not_interrupt_another_reader() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let directory = TestDirectory::new();
    let (path, boundary) = large_database(&directory, "cancellable.sqlite3", 10_000);
    let cancelled = Arc::new(AtomicBool::new(true));
    assert_eq!(
        SessionReader::open_cancellable(&path, cancelled.clone()).unwrap_err(),
        ReaderError::Cancelled
    );
    cancelled.store(false, Ordering::Release);
    let mut reader = SessionReader::open_cancellable(&path, cancelled.clone()).unwrap();
    let mut independent = SessionReader::open(&path).unwrap();
    cancelled.store(true, Ordering::Release);
    assert_eq!(
        reader
            .reduce_series(
                &["speedKph".into()],
                0,
                10_000,
                1024,
                GapPolicy::Automatic,
                boundary
            )
            .unwrap_err(),
        ReaderError::Cancelled
    );
    assert_eq!(
        independent
            .sample_page(&["speedKph".into()], 0, 10_000, 2, None, boundary)
            .unwrap()
            .rows
            .len(),
        2
    );
}
