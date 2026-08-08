use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::BuildHasher;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

use crate::StoreError;

pub const MAX_SERIES_FIELDS: usize = 16;
pub const MAX_SERIES_BUCKETS: usize = 1_024;
pub const MAX_SERIES_OBSERVATIONS: usize = 65_536;
pub const MAX_HISTORY_CURSOR_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedBoundary {
    pub sample_sequence: u64,
    pub event_sequence: u64,
    pub location_sequence: u64,
    pub history_revision: u64,
    pub sample_elapsed_millis: u64,
    pub location_elapsed_millis: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElapsedRange {
    pub start_elapsed_millis: u64,
    pub end_elapsed_millis: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeriesPoint {
    pub elapsed_millis: u64,
    pub sequence: u64,
    pub ride_state: Option<String>,
    pub values: HashMap<String, Option<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeriesColumn {
    pub elapsed_millis: Vec<u64>,
    pub sequence: Vec<u64>,
    pub values: Vec<f64>,
    pub generations: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeriesReduction {
    pub fields: Vec<String>,
    pub range: ElapsedRange,
    pub buckets: usize,
    pub threshold_seconds: Option<f64>,
    pub expected_persistence_millis: u64,
    pub expected_riding_persistence_millis: Option<u64>,
    pub expected_idle_persistence_millis: Option<u64>,
    pub retained_resolution_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCursor {
    pub version: u8,
    pub session_id: String,
    pub boundary: PublishedBoundary,
    pub range: ElapsedRange,
    pub elapsed_millis: u64,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    elapsed_millis: u64,
    sequence: u64,
    value: f64,
    generation: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct BucketCandidates {
    first: Option<Candidate>,
    minimum: Option<Candidate>,
    maximum: Option<Candidate>,
    last: Option<Candidate>,
}

pub struct SeriesAccumulator {
    fields: Vec<String>,
    range: ElapsedRange,
    buckets: usize,
    configured_threshold_millis: Option<u64>,
    expected_persistence_millis: u64,
    expected_riding_persistence_millis: Option<u64>,
    expected_idle_persistence_millis: Option<u64>,
    retained_resolution_seconds: u64,
    generations: Vec<u32>,
    by_field: Vec<Vec<Option<BucketCandidates>>>,
    previous_elapsed_millis: Option<u64>,
    previous_ride_state: Option<String>,
}

impl SeriesAccumulator {
    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn new(reduction: SeriesReduction) -> Result<Self, StoreError> {
        let SeriesReduction {
            fields,
            range,
            buckets,
            threshold_seconds,
            expected_persistence_millis,
            expected_riding_persistence_millis,
            expected_idle_persistence_millis,
            retained_resolution_seconds,
        } = reduction;
        if range.start_elapsed_millis > range.end_elapsed_millis
            || fields.is_empty()
            || fields.len() > MAX_SERIES_FIELDS
            || fields.iter().collect::<HashSet<_>>().len() != fields.len()
            || !(1..=MAX_SERIES_BUCKETS).contains(&buckets)
            || expected_persistence_millis == 0
            || expected_riding_persistence_millis == Some(0)
            || expected_idle_persistence_millis == Some(0)
        {
            return Err(StoreError::Invalid);
        }
        let configured_threshold_millis = match threshold_seconds {
            Some(value) if value.is_finite() && (0.1..=3_600.0).contains(&value) => Some(
                u64::try_from(
                    Duration::try_from_secs_f64(value)
                        .map_err(|_| StoreError::Invalid)?
                        .as_millis(),
                )
                .map_err(|_| StoreError::Invalid)?
                .max(1),
            ),
            Some(_) => return Err(StoreError::Invalid),
            None => None,
        };
        let field_count = fields.len();
        Ok(Self {
            fields,
            range,
            buckets,
            configured_threshold_millis,
            expected_persistence_millis,
            expected_riding_persistence_millis,
            expected_idle_persistence_millis,
            retained_resolution_seconds,
            generations: vec![0; field_count],
            by_field: vec![vec![None; buckets]; field_count],
            previous_elapsed_millis: None,
            previous_ride_state: None,
        })
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn append(&mut self, point: &SeriesPoint) -> Result<(), StoreError> {
        if point.elapsed_millis < self.range.start_elapsed_millis
            || point.elapsed_millis > self.range.end_elapsed_millis
            || self
                .previous_elapsed_millis
                .is_some_and(|previous| point.elapsed_millis < previous)
        {
            return Err(StoreError::Invalid);
        }
        let edge_threshold = self.threshold_for_current_phase();
        if self
            .previous_elapsed_millis
            .is_some_and(|previous| point.elapsed_millis.saturating_sub(previous) > edge_threshold)
        {
            for generation in &mut self.generations {
                *generation = generation.saturating_add(1);
            }
        }
        self.previous_elapsed_millis = Some(point.elapsed_millis);
        self.previous_ride_state.clone_from(&point.ride_state);
        let span = self
            .range
            .end_elapsed_millis
            .saturating_sub(self.range.start_elapsed_millis)
            .saturating_add(1)
            .max(1);
        let offset = point
            .elapsed_millis
            .saturating_sub(self.range.start_elapsed_millis);
        let bucket = usize::try_from(
            offset
                .saturating_mul(self.buckets as u64)
                .checked_div(span)
                .unwrap_or(0),
        )
        .unwrap_or(self.buckets - 1)
        .min(self.buckets - 1);
        for (index, field) in self.fields.iter().enumerate() {
            let Some(value) = point.values.get(field).copied().flatten() else {
                self.generations[index] = self.generations[index].saturating_add(1);
                continue;
            };
            if !value.is_finite() {
                return Err(StoreError::Invalid);
            }
            let candidate = Candidate {
                elapsed_millis: point.elapsed_millis,
                sequence: point.sequence,
                value,
                generation: self.generations[index],
            };
            let mut candidates = self.by_field[index][bucket].unwrap_or_default();
            if candidates
                .first
                .is_none_or(|current| earlier(candidate, current))
            {
                candidates.first = Some(candidate);
            }
            if candidates.minimum.is_none_or(|current| {
                candidate.value < current.value
                    || (candidate.value.total_cmp(&current.value) == Ordering::Equal
                        && earlier(candidate, current))
            }) {
                candidates.minimum = Some(candidate);
            }
            if candidates.maximum.is_none_or(|current| {
                candidate.value > current.value
                    || (candidate.value.total_cmp(&current.value) == Ordering::Equal
                        && earlier(candidate, current))
            }) {
                candidates.maximum = Some(candidate);
            }
            if candidates
                .last
                .is_none_or(|current| earlier(current, candidate))
            {
                candidates.last = Some(candidate);
            }
            self.by_field[index][bucket] = Some(candidates);
        }
        Ok(())
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn finish(self) -> Result<BTreeMap<String, SeriesColumn>, StoreError> {
        let mut output = BTreeMap::new();
        let mut observation_count = 0;
        for (index, field) in self.fields.into_iter().enumerate() {
            let mut selected = BTreeMap::<(u64, u64), Candidate>::new();
            for candidates in self.by_field[index].iter().flatten() {
                for candidate in [
                    candidates.first,
                    candidates.minimum,
                    candidates.maximum,
                    candidates.last,
                ]
                .into_iter()
                .flatten()
                {
                    selected.insert((candidate.elapsed_millis, candidate.sequence), candidate);
                }
            }
            observation_count += selected.len();
            if observation_count > MAX_SERIES_OBSERVATIONS {
                return Err(StoreError::Invalid);
            }
            let values: Vec<_> = selected.into_values().collect();
            output.insert(
                field,
                SeriesColumn {
                    elapsed_millis: values.iter().map(|value| value.elapsed_millis).collect(),
                    sequence: values.iter().map(|value| value.sequence).collect(),
                    values: values.iter().map(|value| value.value).collect(),
                    generations: values.iter().map(|value| value.generation).collect(),
                },
            );
        }
        Ok(output)
    }

    fn threshold_for_current_phase(&self) -> u64 {
        self.configured_threshold_millis.unwrap_or_else(|| {
            let persistence = if self.previous_ride_state.as_deref() == Some("riding") {
                self.expected_riding_persistence_millis
                    .unwrap_or(self.expected_persistence_millis)
            } else {
                self.expected_idle_persistence_millis
                    .unwrap_or(self.expected_persistence_millis)
            };
            1_000
                .max(persistence.saturating_mul(3))
                .max(self.retained_resolution_seconds.saturating_mul(3_000))
        })
    }
}

/// # Errors
/// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
pub fn reduce_series(
    rows: &[SeriesPoint],
    reduction: SeriesReduction,
) -> Result<BTreeMap<String, SeriesColumn>, StoreError> {
    let range = reduction.range;
    let mut accumulator = SeriesAccumulator::new(reduction)?;
    let mut ordered = rows.to_vec();
    ordered.sort_by_key(key);
    for point in &ordered {
        if point.elapsed_millis >= range.start_elapsed_millis
            && point.elapsed_millis <= range.end_elapsed_millis
        {
            accumulator.append(point)?;
        }
    }
    accumulator.finish()
}

/// # Errors
/// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
pub fn select_prune_rows<Protected: BuildHasher, Events: BuildHasher>(
    rows: &[SeriesPoint],
    fields: &[String],
    target_resolution_seconds: u64,
    protected_sequences: &HashSet<u64, Protected>,
    event_source_sequences: &HashSet<u64, Events>,
) -> Result<Vec<u64>, StoreError> {
    if !(1..=300).contains(&target_resolution_seconds)
        || fields.is_empty()
        || fields.len() > MAX_SERIES_FIELDS
    {
        return Err(StoreError::Invalid);
    }
    let mut ordered = rows.to_vec();
    ordered.sort_by_key(key);
    let Some(first) = ordered.first() else {
        return Ok(Vec::new());
    };
    let mut keep: HashSet<u64> = protected_sequences.iter().copied().collect();
    keep.extend(event_source_sequences.iter().copied());
    keep.insert(first.sequence);
    keep.insert(
        ordered
            .last()
            .map_or(first.sequence, |point| point.sequence),
    );
    for field in fields {
        if let Some(point) = ordered
            .iter()
            .find(|point| point.values.get(field).is_some_and(Option::is_some))
        {
            keep.insert(point.sequence);
        }
    }
    let width = target_resolution_seconds.saturating_mul(1_000);
    let mut buckets = BTreeMap::<u64, Vec<&SeriesPoint>>::new();
    for point in &ordered {
        buckets
            .entry(point.elapsed_millis / width)
            .or_default()
            .push(point);
    }
    for points in buckets.values() {
        let Some(first) = points.first() else {
            continue;
        };
        let last = points.last().copied().unwrap_or(first);
        keep.insert(first.sequence);
        keep.insert(last.sequence);
        for field in fields {
            let values: Vec<_> = points
                .iter()
                .filter_map(|point| {
                    point
                        .values
                        .get(field)
                        .copied()
                        .flatten()
                        .map(|v| (*point, v))
                })
                .collect();
            if let Some((point, _)) = values.iter().min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| key(left.0).cmp(&key(right.0)))
            }) {
                keep.insert(point.sequence);
            }
            if let Some((point, _)) = values.iter().max_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| key(right.0).cmp(&key(left.0)))
            }) {
                keep.insert(point.sequence);
            }
        }
    }
    Ok(ordered
        .iter()
        .filter(|point| keep.contains(&point.sequence))
        .map(|point| point.sequence)
        .collect())
}

/// # Errors
/// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
pub fn encode_history_cursor(cursor: &HistoryCursor) -> Result<String, StoreError> {
    validate_cursor(cursor)?;
    let encoded =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(cursor).map_err(|_| StoreError::Invalid)?);
    if encoded.len() > MAX_HISTORY_CURSOR_BYTES {
        Err(StoreError::Invalid)
    } else {
        Ok(encoded)
    }
}

/// # Errors
/// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
pub fn decode_history_cursor(value: &str) -> Result<HistoryCursor, StoreError> {
    if value.is_empty() || value.len() > MAX_HISTORY_CURSOR_BYTES {
        return Err(StoreError::Invalid);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| StoreError::Invalid)?;
    let cursor: HistoryCursor = serde_json::from_slice(&bytes).map_err(|_| StoreError::Invalid)?;
    validate_cursor(&cursor)?;
    Ok(cursor)
}

/// # Errors
/// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
pub fn page_series(
    rows: &[SeriesPoint],
    limit: usize,
    cursor: Option<&HistoryCursor>,
    context: Option<&HistoryCursor>,
) -> Result<(Vec<SeriesPoint>, Option<HistoryCursor>), StoreError> {
    if !(1..=2_000).contains(&limit) {
        return Err(StoreError::Invalid);
    }
    let mut ordered = rows.to_vec();
    ordered.sort_by_key(key);
    let start = cursor.map_or(0, |cursor| {
        ordered.partition_point(|point| {
            (point.elapsed_millis, point.sequence) <= (cursor.elapsed_millis, cursor.sequence)
        })
    });
    let page: Vec<_> = ordered.iter().skip(start).take(limit).cloned().collect();
    let next = if start + page.len() < ordered.len() {
        let base = cursor.or(context).ok_or(StoreError::Invalid)?;
        let last = page.last().ok_or(StoreError::Invalid)?;
        Some(HistoryCursor {
            elapsed_millis: last.elapsed_millis,
            sequence: last.sequence,
            ..base.clone()
        })
    } else {
        None
    };
    Ok((page, next))
}

fn validate_cursor(cursor: &HistoryCursor) -> Result<(), StoreError> {
    if cursor.version != 1
        || cursor.session_id.is_empty()
        || cursor.session_id.len() > 128
        || cursor.range.start_elapsed_millis > cursor.range.end_elapsed_millis
        || cursor.elapsed_millis < cursor.range.start_elapsed_millis
        || cursor.elapsed_millis > cursor.range.end_elapsed_millis
    {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn earlier(left: Candidate, right: Candidate) -> bool {
    (left.elapsed_millis, left.sequence) < (right.elapsed_millis, right.sequence)
}
fn key(point: &SeriesPoint) -> (u64, u64) {
    (point.elapsed_millis, point.sequence)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(sequence: u64, elapsed: u64, voltage: Option<f64>) -> SeriesPoint {
        SeriesPoint {
            elapsed_millis: elapsed,
            sequence,
            ride_state: Some("idle".into()),
            values: HashMap::from([("voltage".into(), voltage)]),
        }
    }

    #[test]
    fn reducer_preserves_bucket_roles_and_splits_gaps_and_nulls() {
        let rows = vec![
            point(1, 0, Some(80.0)),
            point(2, 100, Some(70.0)),
            point(3, 200, Some(90.0)),
            point(4, 4_000, None),
            point(5, 4_100, Some(75.0)),
        ];
        let reduced = reduce_series(
            &rows,
            SeriesReduction {
                fields: vec!["voltage".into()],
                range: ElapsedRange {
                    start_elapsed_millis: 0,
                    end_elapsed_millis: 5_000,
                },
                buckets: 1,
                threshold_seconds: None,
                expected_persistence_millis: 1_000,
                expected_riding_persistence_millis: Some(100),
                expected_idle_persistence_millis: Some(1_000),
                retained_resolution_seconds: 0,
            },
        )
        .expect("reduced");
        let column = &reduced["voltage"];
        assert_eq!(column.sequence, vec![1, 2, 3, 5]);
        assert_eq!(column.generations, vec![0, 0, 0, 2]);
    }

    #[test]
    fn cursors_bind_snapshot_range_and_page_boundary() {
        let context = HistoryCursor {
            version: 1,
            session_id: "session".into(),
            boundary: PublishedBoundary {
                sample_sequence: 5,
                event_sequence: 2,
                location_sequence: 1,
                history_revision: 3,
                sample_elapsed_millis: 5_000,
                location_elapsed_millis: 4_000,
            },
            range: ElapsedRange {
                start_elapsed_millis: 0,
                end_elapsed_millis: 10_000,
            },
            elapsed_millis: 0,
            sequence: 0,
        };
        let encoded = encode_history_cursor(&context).expect("encode");
        assert_eq!(decode_history_cursor(&encoded).expect("decode"), context);
        let rows = vec![
            point(1, 0, Some(1.0)),
            point(2, 1, Some(2.0)),
            point(3, 2, Some(3.0)),
        ];
        let (first, next) = page_series(&rows, 2, None, Some(&context)).expect("page");
        assert_eq!(first.len(), 2);
        let (second, end) = page_series(&rows, 2, next.as_ref(), None).expect("page");
        assert_eq!(second[0].sequence, 3);
        assert!(end.is_none());
    }

    #[test]
    fn prune_selection_retains_edges_extrema_and_protected_evidence() {
        let rows = vec![
            point(1, 0, Some(5.0)),
            point(2, 100, Some(1.0)),
            point(3, 200, Some(9.0)),
            point(4, 300, Some(4.0)),
        ];
        let selected = select_prune_rows(
            &rows,
            &["voltage".into()],
            1,
            &HashSet::from([2]),
            &HashSet::new(),
        )
        .expect("prune");
        assert_eq!(selected, vec![1, 2, 3, 4]);
    }
}
