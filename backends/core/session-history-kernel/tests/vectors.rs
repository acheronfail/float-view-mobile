mod support;

use float_view_session_history_kernel::{
    GapPolicy, PersistenceCadence, PruneRole, RetentionKind, RidePhase, SeriesAccumulator,
    SeriesConfig, SeriesObservation,
};

use support::*;

const SERIES: &str = include_str!("../../../fixtures/session-history/series.tsv");
const SERIES_EXPECTED: &str = include_str!("../../../fixtures/session-history/series-expected.tsv");
const PRUNE: &str = include_str!("../../../fixtures/session-history/prune.tsv");
const PRUNE_EXPECTED: &str = include_str!("../../../fixtures/session-history/prune-expected.tsv");
const PRUNE_EIGHT_CAP: &str = include_str!("../../../fixtures/session-history/prune-eight-cap.tsv");
const PRUNE_EIGHT_CAP_EXPECTED: &str =
    include_str!("../../../fixtures/session-history/prune-eight-cap-expected.tsv");

#[test]
fn shared_series_vector_preserves_m4_ties_gaps_and_late_fields() {
    let rows = parse_series_rows(SERIES).unwrap();
    let fields = ["speedKph", "packVoltageV", "extensionLate"];
    let mut accumulator = SeriesAccumulator::new(SeriesConfig {
        fields: fields.iter().map(|field| (*field).to_owned()).collect(),
        start_elapsed_ms: 0,
        end_elapsed_ms: 10_000,
        buckets: 4,
        gap_policy: GapPolicy::explicit_seconds(2.0).unwrap(),
        cadence: PersistenceCadence::default(),
    })
    .unwrap();
    for row in &rows {
        accumulator
            .append(SeriesObservation {
                elapsed_ms: row.elapsed_ms,
                sequence: row.sequence,
                ride_phase: row.ride_phase,
                values: &row.values,
            })
            .unwrap();
    }
    let columns = accumulator.finish().unwrap();
    assert_eq!(
        columns
            .iter()
            .map(|column| column.field.as_str())
            .collect::<Vec<_>>(),
        fields
    );

    let actual: Vec<_> = columns
        .iter()
        .flat_map(|column| {
            (0..column.values.len()).map(|index| {
                format!(
                    "{}\t{}\t{}\t{}\t{}",
                    column.field,
                    column.sequences[index],
                    column.elapsed_ms[index],
                    column.values[index],
                    column.generations[index]
                )
            })
        })
        .collect();
    assert_eq!(actual, data_lines(SERIES_EXPECTED).collect::<Vec<_>>());
}

#[test]
fn automatic_gap_uses_previous_phase_and_retained_resolution() {
    let mut accumulator = SeriesAccumulator::new(SeriesConfig {
        fields: vec!["speedKph".into()],
        start_elapsed_ms: 0,
        end_elapsed_ms: 10_000,
        buckets: 1,
        gap_policy: GapPolicy::Automatic,
        cadence: PersistenceCadence {
            default_ms: 1_000,
            riding_ms: Some(500),
            idle_ms: Some(2_000),
            retained_resolution_seconds: 0,
        },
    })
    .unwrap();
    for (elapsed_ms, phase, value) in [
        (0, RidePhase::Riding, 1.0),
        (1_501, RidePhase::Riding, 0.0),
        (4_502, RidePhase::Other, 3.0),
        (10_000, RidePhase::Other, 2.0),
    ] {
        accumulator
            .append(SeriesObservation {
                elapsed_ms,
                sequence: elapsed_ms,
                ride_phase: phase,
                values: &[Some(value)],
            })
            .unwrap();
    }
    assert_eq!(accumulator.finish().unwrap()[0].generations, [0, 1, 2, 2]);

    let mut retained = SeriesAccumulator::new(SeriesConfig {
        fields: vec!["speedKph".into()],
        start_elapsed_ms: 0,
        end_elapsed_ms: 5_000,
        buckets: 1,
        gap_policy: GapPolicy::Automatic,
        cadence: PersistenceCadence {
            default_ms: 100,
            riding_ms: None,
            idle_ms: None,
            retained_resolution_seconds: 2,
        },
    })
    .unwrap();
    for elapsed_ms in [0, 5_000] {
        retained
            .append(SeriesObservation {
                elapsed_ms,
                sequence: elapsed_ms,
                ride_phase: RidePhase::Other,
                values: &[Some(elapsed_ms as f64)],
            })
            .unwrap();
    }
    assert_eq!(retained.finish().unwrap()[0].generations, [0, 0]);
}

#[test]
fn shared_prune_vector_preserves_semantic_sources_and_finalizes_buckets() {
    let rows = parse_prune_rows(PRUNE).unwrap();
    let selections = select(&rows, &["speedKph", "packVoltageV", "extensionLate"], 5).unwrap();
    assert_eq!(
        selections
            .iter()
            .map(|selection| selection.bucket_index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert!(
        selections
            .iter()
            .all(|selection| selection.uncovered_roles.is_empty())
    );
    let actual: Vec<_> = selections
        .iter()
        .flat_map(|selection| {
            selection.retained.iter().map(|row| {
                let kind = match row.kind {
                    RetentionKind::Protected => "protected",
                    RetentionKind::Ordinary => "ordinary",
                };
                format!("{}\t{}\t{kind}", selection.bucket_index, row.sequence)
            })
        })
        .collect();
    assert_eq!(actual, data_lines(PRUNE_EXPECTED).collect::<Vec<_>>());
    let protected: Vec<_> = selections
        .iter()
        .flat_map(|selection| &selection.retained)
        .filter(|row| row.kind == RetentionKind::Protected)
        .map(|row| row.sequence)
        .collect();
    assert!(protected.contains(&11));
    assert!(protected.contains(&12));
}

#[test]
fn shared_dense_prune_vector_enforces_eight_ordinary_rows() {
    let rows = parse_dense_prune_rows(PRUNE_EIGHT_CAP).unwrap();
    let field_names: Vec<_> = (0..10).map(|index| format!("f{index}")).collect();
    let fields: Vec<_> = field_names.iter().map(String::as_str).collect();
    let selections = select(&rows, &fields, 5).unwrap();
    assert_eq!(selections.len(), 1);
    let selection = &selections[0];
    assert_eq!(
        selection
            .retained
            .iter()
            .filter(|row| row.kind == RetentionKind::Ordinary)
            .count(),
        8
    );
    assert_eq!(selection.uncovered_roles, [PruneRole::Minimum(9)]);
    let actual: Vec<_> = selection
        .retained
        .iter()
        .map(|row| {
            format!(
                "{}\t{}",
                row.sequence,
                match row.kind {
                    RetentionKind::Protected => "protected",
                    RetentionKind::Ordinary => "ordinary",
                }
            )
        })
        .collect();
    let expected: Vec<_> = data_lines(PRUNE_EIGHT_CAP_EXPECTED)
        .filter(|line| !line.starts_with("# uncovered"))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn repeated_coarser_selection_never_invents_a_source_row() {
    let rows = parse_prune_rows(PRUNE).unwrap();
    let fields = ["speedKph", "packVoltageV", "extensionLate"];
    let fine = select(&rows, &fields, 1).unwrap();
    let fine_sequences: Vec<_> = fine
        .iter()
        .flat_map(|selection| &selection.retained)
        .map(|row| row.sequence)
        .collect();
    let retained_rows: Vec<_> = rows
        .iter()
        .filter(|row| fine_sequences.contains(&row.sequence))
        .cloned()
        .map(|mut row| {
            row.protected = true;
            row
        })
        .collect();
    let coarse = select(&retained_rows, &fields, 5).unwrap();
    let coarse_sequences: Vec<_> = coarse
        .iter()
        .flat_map(|selection| &selection.retained)
        .map(|row| row.sequence)
        .collect();
    assert_eq!(coarse_sequences, fine_sequences);
    assert!(
        coarse_sequences
            .iter()
            .all(|sequence| rows.iter().any(|row| row.sequence == *sequence))
    );
}

#[test]
fn riding_transition_across_bucket_edge_protects_both_source_rows() {
    let rows = [
        Row {
            sequence: 1,
            elapsed_ms: 999,
            ride_phase: RidePhase::Riding,
            protected: false,
            values: vec![Some(1.0)],
        },
        Row {
            sequence: 2,
            elapsed_ms: 1_000,
            ride_phase: RidePhase::Other,
            protected: false,
            values: vec![Some(2.0)],
        },
    ];
    let selections = select(&rows, &["speedKph"], 1).unwrap();
    assert_eq!(selections.len(), 2);
    assert!(selections.iter().all(|selection| {
        selection.retained.len() == 1 && selection.retained[0].kind == RetentionKind::Protected
    }));
}

#[test]
fn fixture_reader_rejects_malformed_or_non_finite_rows() {
    assert!(parse_series_rows("1\t2\tother\t1\t2").is_err());
    assert!(parse_prune_rows("1\t2\tother\tfalse\t1\tNaN\t3").is_err());
    assert!(parse_dense_prune_rows("1\t2\t1").is_err());
}
