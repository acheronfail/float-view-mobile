use float_view_session_history_kernel::{
    GapPolicy, HistoryError, MAX_BUCKETS, MAX_FIELDS, MAX_SERIES_OBSERVATIONS, PersistenceCadence,
    PruneConfig, PruneObservation, PruneSelector, RidePhase, SeriesAccumulator, SeriesConfig,
    SeriesObservation,
};

fn fields(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("field{index}")).collect()
}

fn config(field_count: usize, buckets: usize) -> SeriesConfig {
    SeriesConfig {
        fields: fields(field_count),
        start_elapsed_ms: 0,
        end_elapsed_ms: 10_000,
        buckets,
        gap_policy: GapPolicy::Automatic,
        cadence: PersistenceCadence::default(),
    }
}

#[test]
fn series_configuration_rejects_every_invalid_bound() {
    assert_eq!(
        SeriesAccumulator::new(config(0, 1)).unwrap_err(),
        HistoryError::InvalidFields
    );
    assert_eq!(
        SeriesAccumulator::new(config(MAX_FIELDS + 1, 1)).unwrap_err(),
        HistoryError::InvalidFields
    );
    let mut duplicate = config(2, 1);
    duplicate.fields[1] = duplicate.fields[0].clone();
    assert_eq!(
        SeriesAccumulator::new(duplicate).unwrap_err(),
        HistoryError::InvalidFields
    );
    let mut empty = config(1, 1);
    empty.fields[0].clear();
    assert_eq!(
        SeriesAccumulator::new(empty).unwrap_err(),
        HistoryError::InvalidFields
    );
    let mut oversized = config(1, 1);
    oversized.fields[0] = "x".repeat(65);
    assert_eq!(
        SeriesAccumulator::new(oversized).unwrap_err(),
        HistoryError::InvalidFields
    );
    assert_eq!(
        SeriesAccumulator::new(config(1, 0)).unwrap_err(),
        HistoryError::InvalidBuckets
    );
    assert_eq!(
        SeriesAccumulator::new(config(1, MAX_BUCKETS + 1)).unwrap_err(),
        HistoryError::InvalidBuckets
    );

    let mut range = config(1, 1);
    range.start_elapsed_ms = 2;
    range.end_elapsed_ms = 1;
    assert_eq!(
        SeriesAccumulator::new(range).unwrap_err(),
        HistoryError::InvalidRange
    );
    let mut overflowing = config(1, 1);
    overflowing.end_elapsed_ms = u64::MAX;
    assert_eq!(
        SeriesAccumulator::new(overflowing).unwrap_err(),
        HistoryError::ArithmeticOverflow
    );

    for threshold in [0, 99, 3_600_001, u64::MAX] {
        let mut invalid = config(1, 1);
        invalid.gap_policy = GapPolicy::ExplicitMilliseconds(threshold);
        assert_eq!(
            SeriesAccumulator::new(invalid).unwrap_err(),
            HistoryError::InvalidGapPolicy
        );
    }
    for seconds in [f64::NAN, f64::NEG_INFINITY, 0.099, 3_600.001] {
        assert_eq!(
            GapPolicy::explicit_seconds(seconds),
            Err(HistoryError::InvalidGapPolicy)
        );
    }
    assert_eq!(
        GapPolicy::explicit_seconds(0.1005),
        Ok(GapPolicy::ExplicitMilliseconds(101))
    );

    let mut cadence = config(1, 1);
    cadence.cadence.default_ms = 0;
    assert_eq!(
        SeriesAccumulator::new(cadence).unwrap_err(),
        HistoryError::InvalidCadence
    );
    let mut cadence = config(1, 1);
    cadence.cadence.riding_ms = Some(0);
    assert_eq!(
        SeriesAccumulator::new(cadence).unwrap_err(),
        HistoryError::InvalidCadence
    );
    let mut cadence = config(1, 1);
    cadence.cadence.idle_ms = Some(0);
    assert_eq!(
        SeriesAccumulator::new(cadence).unwrap_err(),
        HistoryError::InvalidCadence
    );
    let mut resolution = config(1, 1);
    resolution.cadence.retained_resolution_seconds = 301;
    assert_eq!(
        SeriesAccumulator::new(resolution).unwrap_err(),
        HistoryError::InvalidResolution
    );
    let mut cadence = config(1, 1);
    cadence.cadence.default_ms = u64::MAX;
    assert_eq!(
        SeriesAccumulator::new(cadence).unwrap_err(),
        HistoryError::ArithmeticOverflow
    );
}

#[test]
fn invalid_series_rows_do_not_partially_mutate_or_publish() {
    let mut accumulator = SeriesAccumulator::new(config(2, 2)).unwrap();
    accumulator
        .append(SeriesObservation {
            elapsed_ms: 0,
            sequence: 1,
            ride_phase: RidePhase::Other,
            values: &[Some(1.0), Some(2.0)],
        })
        .unwrap();
    let before = accumulator.finish().unwrap();
    for (elapsed_ms, sequence, values, expected) in [
        (1, 2, vec![Some(1.0)], HistoryError::InvalidRow),
        (
            1,
            2,
            vec![Some(f64::NAN), Some(2.0)],
            HistoryError::NonFiniteValue,
        ),
        (
            1,
            2,
            vec![Some(f64::INFINITY), Some(2.0)],
            HistoryError::NonFiniteValue,
        ),
        (
            10_001,
            2,
            vec![Some(1.0), Some(2.0)],
            HistoryError::InvalidRow,
        ),
        (0, 1, vec![Some(1.0), Some(2.0)], HistoryError::InvalidOrder),
    ] {
        assert_eq!(
            accumulator.append(SeriesObservation {
                elapsed_ms,
                sequence,
                ride_phase: RidePhase::Other,
                values: &values,
            }),
            Err(expected)
        );
        assert_eq!(accumulator.finish().unwrap(), before);
    }
    accumulator
        .append(SeriesObservation {
            elapsed_ms: 1,
            sequence: 2,
            ride_phase: RidePhase::Other,
            values: &[Some(3.0), None],
        })
        .unwrap();
    assert_eq!(accumulator.finish().unwrap()[0].sequences, [1, 2]);
}

#[test]
fn exact_maximum_series_shape_is_finite_and_deterministic() {
    let mut accumulator = SeriesAccumulator::new(SeriesConfig {
        fields: fields(MAX_FIELDS),
        start_elapsed_ms: 0,
        end_elapsed_ms: 4_095,
        buckets: MAX_BUCKETS,
        gap_policy: GapPolicy::ExplicitMilliseconds(3_600_000),
        cadence: PersistenceCadence::default(),
    })
    .unwrap();
    let patterns = [0.0, -1.0, 1.0, 0.5];
    for elapsed_ms in 0..4_096 {
        let values = vec![Some(patterns[elapsed_ms as usize % 4]); MAX_FIELDS];
        accumulator
            .append(SeriesObservation {
                elapsed_ms,
                sequence: elapsed_ms,
                ride_phase: RidePhase::Riding,
                values: &values,
            })
            .unwrap();
    }
    let columns = accumulator.finish().unwrap();
    assert_eq!(
        columns
            .iter()
            .map(|column| column.values.len())
            .sum::<usize>(),
        MAX_SERIES_OBSERVATIONS
    );
    assert!(
        columns
            .iter()
            .all(|column| column.values.iter().all(|value| value.is_finite()))
    );
}

#[test]
fn prune_configuration_and_rows_fail_closed() {
    for (field_list, resolution, expected) in [
        (Vec::new(), 1, HistoryError::InvalidFields),
        (fields(1), 0, HistoryError::InvalidResolution),
        (fields(1), 301, HistoryError::InvalidResolution),
    ] {
        assert_eq!(
            PruneSelector::new(PruneConfig {
                fields: field_list,
                resolution_seconds: resolution
            })
            .unwrap_err(),
            expected
        );
    }
    let mut selector = PruneSelector::new(PruneConfig {
        fields: fields(2),
        resolution_seconds: 1,
    })
    .unwrap();
    selector
        .append(PruneObservation {
            elapsed_ms: 0,
            sequence: 1,
            ride_phase: RidePhase::Other,
            values: &[Some(1.0), Some(2.0)],
            protected: false,
        })
        .unwrap();
    assert_eq!(
        selector.append(PruneObservation {
            elapsed_ms: 1,
            sequence: 2,
            ride_phase: RidePhase::Other,
            values: &[Some(1.0)],
            protected: false,
        }),
        Err(HistoryError::InvalidRow)
    );
    assert_eq!(
        selector.append(PruneObservation {
            elapsed_ms: 1,
            sequence: 2,
            ride_phase: RidePhase::Other,
            values: &[Some(1.0), Some(f64::NEG_INFINITY)],
            protected: false,
        }),
        Err(HistoryError::NonFiniteValue)
    );
    assert_eq!(
        selector.append(PruneObservation {
            elapsed_ms: 0,
            sequence: 1,
            ride_phase: RidePhase::Other,
            values: &[Some(1.0), Some(2.0)],
            protected: false,
        }),
        Err(HistoryError::InvalidOrder)
    );
    assert_eq!(
        selector.append(PruneObservation {
            elapsed_ms: 0,
            sequence: 0,
            ride_phase: RidePhase::Other,
            values: &[Some(1.0), Some(2.0)],
            protected: false,
        }),
        Err(HistoryError::InvalidOrder)
    );
    let finished = selector.finish().unwrap().unwrap();
    assert_eq!(finished.retained.len(), 1);
}

#[test]
fn empty_prune_stream_finishes_without_fabricating_rows() {
    let selector = PruneSelector::new(PruneConfig {
        fields: fields(1),
        resolution_seconds: 300,
    })
    .unwrap();
    assert_eq!(selector.finish(), Ok(None));
}

#[test]
fn debug_formatting_redacts_history_values_and_source_keys() {
    let mut accumulator = SeriesAccumulator::new(config(1, 1)).unwrap();
    accumulator
        .append(SeriesObservation {
            elapsed_ms: 9_876,
            sequence: 1_234,
            ride_phase: RidePhase::Other,
            values: &[Some(54.321)],
        })
        .unwrap();
    let column = accumulator.finish().unwrap().remove(0);
    let debug = format!("{column:?}");
    assert!(!debug.contains("54.321"));
    assert!(!debug.contains("9876"));
    assert!(!debug.contains("1234"));

    let mut selector = PruneSelector::new(PruneConfig {
        fields: fields(1),
        resolution_seconds: 300,
    })
    .unwrap();
    selector
        .append(PruneObservation {
            elapsed_ms: 9_876,
            sequence: 1_234,
            ride_phase: RidePhase::Other,
            values: &[Some(54.321)],
            protected: true,
        })
        .unwrap();
    let selection = selector.finish().unwrap().unwrap();
    let debug = format!("{selection:?} {:?}", selection.retained[0]);
    assert!(!debug.contains("54.321"));
    assert!(!debug.contains("9876"));
    assert!(!debug.contains("1234"));
}
