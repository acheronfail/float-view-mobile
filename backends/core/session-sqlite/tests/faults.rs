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
                    source_ticks: 12300,
                    active: true,
                    raw_code: 4,
                },
                FaultRecord {
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
