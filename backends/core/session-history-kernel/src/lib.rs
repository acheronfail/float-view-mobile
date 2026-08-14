//! Bounded platform-neutral historical-series and prune-selection behavior.
//!
//! This safe candidate consumes ordered session facts and returns owned values
//! for a future Rust SQLite/runtime owner. It owns no database, persistence
//! port, cursor codec, transport, callback, host ABI, executor, or OS behavior
//! and remains deliberately unselected by every production host.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::fmt;

pub const MAX_FIELDS: usize = 16;
pub const MAX_FIELD_NAME_BYTES: usize = 64;
pub const MAX_BUCKETS: usize = 1_024;
pub const MAX_SERIES_OBSERVATIONS: usize = 65_536;
pub const MAX_PRUNE_RESOLUTION_SECONDS: u64 = 300;
pub const MAX_ORDINARY_ROWS_PER_BUCKET: usize = 8;
pub const MAX_PROTECTED_ROWS_PER_BUCKET: usize = 65_536;

const MIN_GAP_THRESHOLD_MS: u64 = 100;
const MAX_GAP_THRESHOLD_MS: u64 = 3_600_000;
const MAX_RETAINED_RESOLUTION_SECONDS: u64 = 300;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryError {
    InvalidFields,
    InvalidRange,
    InvalidBuckets,
    InvalidGapPolicy,
    InvalidCadence,
    InvalidResolution,
    InvalidRow,
    InvalidOrder,
    NonFiniteValue,
    ArithmeticOverflow,
    GenerationExhausted,
    ObservationLimit,
    ProtectedLimit,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidFields => "invalid session-history fields",
            Self::InvalidRange => "invalid session-history range",
            Self::InvalidBuckets => "invalid session-history bucket count",
            Self::InvalidGapPolicy => "invalid session-history gap policy",
            Self::InvalidCadence => "invalid session-history cadence",
            Self::InvalidResolution => "invalid session-history resolution",
            Self::InvalidRow => "invalid session-history row",
            Self::InvalidOrder => "invalid session-history row order",
            Self::NonFiniteValue => "non-finite session-history value",
            Self::ArithmeticOverflow => "session-history arithmetic overflow",
            Self::GenerationExhausted => "session-history generation exhausted",
            Self::ObservationLimit => "session-history observation limit exceeded",
            Self::ProtectedLimit => "session-history protected-row limit exceeded",
        })
    }
}

impl std::error::Error for HistoryError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RidePhase {
    Riding,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GapPolicy {
    Automatic,
    ExplicitMilliseconds(u64),
}

impl GapPolicy {
    pub fn explicit_seconds(seconds: f64) -> Result<Self, HistoryError> {
        if !seconds.is_finite() || !(0.1..=3_600.0).contains(&seconds) {
            return Err(HistoryError::InvalidGapPolicy);
        }
        let milliseconds = (seconds * 1_000.0).round();
        if !(MIN_GAP_THRESHOLD_MS as f64..=MAX_GAP_THRESHOLD_MS as f64).contains(&milliseconds) {
            return Err(HistoryError::InvalidGapPolicy);
        }
        Ok(Self::ExplicitMilliseconds(milliseconds as u64))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersistenceCadence {
    pub default_ms: u64,
    pub riding_ms: Option<u64>,
    pub idle_ms: Option<u64>,
    pub retained_resolution_seconds: u64,
}

impl Default for PersistenceCadence {
    fn default() -> Self {
        Self {
            default_ms: 1_000,
            riding_ms: None,
            idle_ms: None,
            retained_resolution_seconds: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeriesConfig {
    pub fields: Vec<String>,
    pub start_elapsed_ms: u64,
    pub end_elapsed_ms: u64,
    pub buckets: usize,
    pub gap_policy: GapPolicy,
    pub cadence: PersistenceCadence,
}

#[derive(Clone, Copy)]
pub struct SeriesObservation<'a> {
    pub elapsed_ms: u64,
    pub sequence: u64,
    pub ride_phase: RidePhase,
    pub values: &'a [Option<f64>],
}

impl fmt::Debug for SeriesObservation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SeriesObservation")
            .field("value_count", &self.values.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq)]
pub struct SeriesColumn {
    pub field: String,
    pub elapsed_ms: Vec<u64>,
    pub sequences: Vec<u64>,
    pub values: Vec<f64>,
    pub generations: Vec<u32>,
}

impl fmt::Debug for SeriesColumn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SeriesColumn")
            .field("field", &self.field)
            .field("observation_count", &self.values.len())
            .finish()
    }
}

#[derive(Clone, Copy)]
struct Key {
    elapsed_ms: u64,
    sequence: u64,
}

impl Key {
    const fn new(elapsed_ms: u64, sequence: u64) -> Self {
        Self {
            elapsed_ms,
            sequence,
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.elapsed_ms == other.elapsed_ms && self.sequence == other.sequence
    }
}

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        self.elapsed_ms
            .cmp(&other.elapsed_ms)
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

#[derive(Clone, Copy)]
struct SeriesCandidate {
    key: Key,
    value: f64,
    generation: u32,
}

#[derive(Clone, Copy, Default)]
struct SeriesRoles {
    first: Option<SeriesCandidate>,
    minimum: Option<SeriesCandidate>,
    maximum: Option<SeriesCandidate>,
    last: Option<SeriesCandidate>,
}

pub struct SeriesAccumulator {
    fields: Vec<String>,
    start_elapsed_ms: u64,
    span: u64,
    end_elapsed_ms: u64,
    bucket_count: usize,
    gap_policy: GapPolicy,
    automatic_riding_threshold_ms: u64,
    automatic_other_threshold_ms: u64,
    generations: Vec<u32>,
    roles: Vec<Vec<SeriesRoles>>,
    previous_key: Option<Key>,
    previous_phase: RidePhase,
}

impl fmt::Debug for SeriesAccumulator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SeriesAccumulator")
            .field("field_count", &self.fields.len())
            .field("bucket_count", &self.bucket_count)
            .field("has_previous", &self.previous_key.is_some())
            .finish()
    }
}

impl SeriesAccumulator {
    pub fn new(config: SeriesConfig) -> Result<Self, HistoryError> {
        validate_fields(&config.fields)?;
        if config.start_elapsed_ms > config.end_elapsed_ms {
            return Err(HistoryError::InvalidRange);
        }
        let span = config
            .end_elapsed_ms
            .checked_sub(config.start_elapsed_ms)
            .and_then(|value| value.checked_add(1))
            .ok_or(HistoryError::ArithmeticOverflow)?;
        if !(1..=MAX_BUCKETS).contains(&config.buckets) {
            return Err(HistoryError::InvalidBuckets);
        }
        validate_gap_policy(config.gap_policy)?;
        let (riding_threshold, other_threshold) = automatic_thresholds(config.cadence)?;
        let roles = (0..config.fields.len())
            .map(|_| vec![SeriesRoles::default(); config.buckets])
            .collect();
        Ok(Self {
            generations: vec![0; config.fields.len()],
            fields: config.fields,
            start_elapsed_ms: config.start_elapsed_ms,
            span,
            end_elapsed_ms: config.end_elapsed_ms,
            bucket_count: config.buckets,
            gap_policy: config.gap_policy,
            automatic_riding_threshold_ms: riding_threshold,
            automatic_other_threshold_ms: other_threshold,
            roles,
            previous_key: None,
            previous_phase: RidePhase::Other,
        })
    }

    pub fn append(&mut self, observation: SeriesObservation<'_>) -> Result<(), HistoryError> {
        if observation.values.len() != self.fields.len()
            || observation.elapsed_ms < self.start_elapsed_ms
            || observation.elapsed_ms > self.end_elapsed_ms
        {
            return Err(HistoryError::InvalidRow);
        }
        if observation
            .values
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(HistoryError::NonFiniteValue);
        }
        let key = Key::new(observation.elapsed_ms, observation.sequence);
        if self.previous_key.is_some_and(|previous| key <= previous) {
            return Err(HistoryError::InvalidOrder);
        }

        let has_gap = self.previous_key.is_some_and(|previous| {
            let threshold = match self.gap_policy {
                GapPolicy::ExplicitMilliseconds(value) => value,
                GapPolicy::Automatic => match self.previous_phase {
                    RidePhase::Riding => self.automatic_riding_threshold_ms,
                    RidePhase::Other => self.automatic_other_threshold_ms,
                },
            };
            observation.elapsed_ms - previous.elapsed_ms > threshold
        });
        let mut next_generations = self.generations.clone();
        if has_gap {
            increment_all(&mut next_generations)?;
        }
        for (generation, value) in next_generations.iter_mut().zip(observation.values) {
            if value.is_none() {
                *generation = generation
                    .checked_add(1)
                    .ok_or(HistoryError::GenerationExhausted)?;
            }
        }

        let offset = observation.elapsed_ms - self.start_elapsed_ms;
        let bucket =
            ((u128::from(offset) * self.bucket_count as u128) / u128::from(self.span)) as usize;
        if bucket >= self.bucket_count {
            return Err(HistoryError::ArithmeticOverflow);
        }
        for (field_index, value) in observation.values.iter().enumerate() {
            let Some(value) = value else { continue };
            let candidate = SeriesCandidate {
                key,
                value: *value,
                generation: next_generations[field_index],
            };
            update_series_roles(&mut self.roles[field_index][bucket], candidate);
        }
        self.generations = next_generations;
        self.previous_key = Some(key);
        self.previous_phase = observation.ride_phase;
        Ok(())
    }

    pub fn finish(&self) -> Result<Vec<SeriesColumn>, HistoryError> {
        let mut columns = Vec::with_capacity(self.fields.len());
        let mut total = 0usize;
        for (field_index, field) in self.fields.iter().enumerate() {
            let mut selected = Vec::with_capacity(self.bucket_count.saturating_mul(4));
            for roles in &self.roles[field_index] {
                selected.extend(
                    [roles.first, roles.minimum, roles.maximum, roles.last]
                        .into_iter()
                        .flatten(),
                );
            }
            selected.sort_by_key(|candidate| candidate.key);
            selected.dedup_by_key(|candidate| candidate.key);
            total = total
                .checked_add(selected.len())
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if total > MAX_SERIES_OBSERVATIONS {
                return Err(HistoryError::ObservationLimit);
            }
            columns.push(SeriesColumn {
                field: field.clone(),
                elapsed_ms: selected
                    .iter()
                    .map(|candidate| candidate.key.elapsed_ms)
                    .collect(),
                sequences: selected
                    .iter()
                    .map(|candidate| candidate.key.sequence)
                    .collect(),
                values: selected.iter().map(|candidate| candidate.value).collect(),
                generations: selected
                    .iter()
                    .map(|candidate| candidate.generation)
                    .collect(),
            });
        }
        Ok(columns)
    }
}

fn update_series_roles(roles: &mut SeriesRoles, candidate: SeriesCandidate) {
    if roles
        .first
        .is_none_or(|current| candidate.key < current.key)
    {
        roles.first = Some(candidate);
    }
    if roles.minimum.is_none_or(|current| {
        candidate.value < current.value
            || (candidate.value == current.value && candidate.key < current.key)
    }) {
        roles.minimum = Some(candidate);
    }
    if roles.maximum.is_none_or(|current| {
        candidate.value > current.value
            || (candidate.value == current.value && candidate.key < current.key)
    }) {
        roles.maximum = Some(candidate);
    }
    if roles.last.is_none_or(|current| candidate.key > current.key) {
        roles.last = Some(candidate);
    }
}

fn increment_all(generations: &mut [u32]) -> Result<(), HistoryError> {
    for generation in generations {
        *generation = generation
            .checked_add(1)
            .ok_or(HistoryError::GenerationExhausted)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PruneConfig {
    pub fields: Vec<String>,
    pub resolution_seconds: u64,
}

#[derive(Clone, Copy)]
pub struct PruneObservation<'a> {
    pub elapsed_ms: u64,
    pub sequence: u64,
    pub ride_phase: RidePhase,
    pub values: &'a [Option<f64>],
    pub protected: bool,
}

impl fmt::Debug for PruneObservation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PruneObservation")
            .field("value_count", &self.values.len())
            .field("protected", &self.protected)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetentionKind {
    Protected,
    Ordinary,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RetainedRow {
    pub elapsed_ms: u64,
    pub sequence: u64,
    pub kind: RetentionKind,
}

impl fmt::Debug for RetainedRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetainedRow")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PruneRole {
    First,
    Last,
    Minimum(usize),
    Maximum(usize),
}

#[derive(Clone, PartialEq)]
pub struct PruneBucketSelection {
    pub bucket_index: u64,
    pub retained: Vec<RetainedRow>,
    pub uncovered_roles: Vec<PruneRole>,
}

impl fmt::Debug for PruneBucketSelection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PruneBucketSelection")
            .field("retained_count", &self.retained.len())
            .field("uncovered_role_count", &self.uncovered_roles.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy)]
struct ValueCandidate {
    key: Key,
    value: f64,
}

#[derive(Clone, Copy, Default)]
struct FieldExtrema {
    minimum: Option<ValueCandidate>,
    maximum: Option<ValueCandidate>,
}

struct PruneBucketState {
    index: u64,
    first: Key,
    last: Key,
    protected: Vec<Key>,
    extrema: Vec<FieldExtrema>,
}

impl PruneBucketState {
    fn new(index: u64, key: Key, field_count: usize) -> Self {
        Self {
            index,
            first: key,
            last: key,
            protected: Vec::new(),
            extrema: vec![FieldExtrema::default(); field_count],
        }
    }

    fn protect(&mut self, key: Key) -> Result<(), HistoryError> {
        if self.protected.last().copied() == Some(key) {
            return Ok(());
        }
        if self.protected.len() >= MAX_PROTECTED_ROWS_PER_BUCKET {
            return Err(HistoryError::ProtectedLimit);
        }
        self.protected.push(key);
        Ok(())
    }

    fn observe(&mut self, key: Key, values: &[Option<f64>]) {
        self.last = key;
        for (extrema, value) in self.extrema.iter_mut().zip(values) {
            let Some(value) = value else { continue };
            let candidate = ValueCandidate { key, value: *value };
            if extrema.minimum.is_none_or(|current| {
                candidate.value < current.value
                    || (candidate.value == current.value && candidate.key < current.key)
            }) {
                extrema.minimum = Some(candidate);
            }
            if extrema.maximum.is_none_or(|current| {
                candidate.value > current.value
                    || (candidate.value == current.value && candidate.key < current.key)
            }) {
                extrema.maximum = Some(candidate);
            }
        }
    }
}

pub struct PruneSelector {
    fields: Vec<String>,
    bucket_width_ms: u64,
    seen_present: Vec<bool>,
    current: Option<PruneBucketState>,
    previous_key: Option<Key>,
    previous_phase: RidePhase,
}

impl fmt::Debug for PruneSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PruneSelector")
            .field("field_count", &self.fields.len())
            .field("has_current_bucket", &self.current.is_some())
            .finish()
    }
}

impl PruneSelector {
    pub fn new(config: PruneConfig) -> Result<Self, HistoryError> {
        validate_fields(&config.fields)?;
        if !(1..=MAX_PRUNE_RESOLUTION_SECONDS).contains(&config.resolution_seconds) {
            return Err(HistoryError::InvalidResolution);
        }
        let bucket_width_ms = config
            .resolution_seconds
            .checked_mul(1_000)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        Ok(Self {
            seen_present: vec![false; config.fields.len()],
            fields: config.fields,
            bucket_width_ms,
            current: None,
            previous_key: None,
            previous_phase: RidePhase::Other,
        })
    }

    pub fn append(
        &mut self,
        observation: PruneObservation<'_>,
    ) -> Result<Option<PruneBucketSelection>, HistoryError> {
        if observation.values.len() != self.fields.len() {
            return Err(HistoryError::InvalidRow);
        }
        if observation
            .values
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(HistoryError::NonFiniteValue);
        }
        let key = Key::new(observation.elapsed_ms, observation.sequence);
        if self.previous_key.is_some_and(|previous| key <= previous) {
            return Err(HistoryError::InvalidOrder);
        }
        let bucket_index = observation.elapsed_ms / self.bucket_width_ms;
        let phase_transition = self
            .previous_key
            .is_some_and(|_| observation.ride_phase != self.previous_phase);

        if phase_transition {
            let previous = self.previous_key.ok_or(HistoryError::InvalidRow)?;
            self.current
                .as_mut()
                .ok_or(HistoryError::InvalidRow)?
                .protect(previous)?;
        }

        let finalized = if self
            .current
            .as_ref()
            .is_some_and(|state| state.index != bucket_index)
        {
            self.current.take().map(finalize_prune_bucket)
        } else {
            None
        };
        if self.current.is_none() {
            self.current = Some(PruneBucketState::new(bucket_index, key, self.fields.len()));
        }

        let first_row = self.previous_key.is_none();
        let first_present = observation
            .values
            .iter()
            .enumerate()
            .any(|(index, value)| value.is_some() && !self.seen_present[index]);
        if first_row || phase_transition || observation.protected || first_present {
            self.current
                .as_mut()
                .ok_or(HistoryError::InvalidRow)?
                .protect(key)?;
        }
        for (seen, value) in self.seen_present.iter_mut().zip(observation.values) {
            *seen |= value.is_some();
        }
        self.current
            .as_mut()
            .ok_or(HistoryError::InvalidRow)?
            .observe(key, observation.values);
        self.previous_key = Some(key);
        self.previous_phase = observation.ride_phase;
        Ok(finalized)
    }

    pub fn finish(mut self) -> Result<Option<PruneBucketSelection>, HistoryError> {
        if let Some(previous) = self.previous_key {
            self.current
                .as_mut()
                .ok_or(HistoryError::InvalidRow)?
                .protect(previous)?;
        }
        Ok(self.current.take().map(finalize_prune_bucket))
    }

    #[cfg(test)]
    fn state_counts(&self) -> (usize, usize) {
        self.current.as_ref().map_or((0, 0), |state| {
            let candidates = 2 + state.extrema.len() * 2;
            (state.protected.len(), candidates)
        })
    }
}

fn finalize_prune_bucket(mut state: PruneBucketState) -> PruneBucketSelection {
    state.protected.sort_unstable();
    state.protected.dedup();

    let mut role_candidates = Vec::with_capacity(2 + state.extrema.len() * 2);
    role_candidates.push((PruneRole::First, state.first));
    role_candidates.push((PruneRole::Last, state.last));
    for (index, extrema) in state.extrema.iter().enumerate() {
        if let Some(candidate) = extrema.minimum {
            role_candidates.push((PruneRole::Minimum(index), candidate.key));
        }
        if let Some(candidate) = extrema.maximum {
            role_candidates.push((PruneRole::Maximum(index), candidate.key));
        }
    }

    let mut uncovered: Vec<(PruneRole, Key)> = role_candidates
        .into_iter()
        .filter(|(_, key)| state.protected.binary_search(key).is_err())
        .collect();
    let mut candidates: Vec<Key> = uncovered.iter().map(|(_, key)| *key).collect();
    candidates.sort_unstable();
    candidates.dedup();
    let mut ordinary = Vec::with_capacity(MAX_ORDINARY_ROWS_PER_BUCKET);
    while !uncovered.is_empty() && ordinary.len() < MAX_ORDINARY_ROWS_PER_BUCKET {
        let Some(choice) = candidates
            .iter()
            .copied()
            .filter(|key| !ordinary.contains(key))
            .max_by(|left, right| {
                let left_coverage = uncovered.iter().filter(|(_, key)| key == left).count();
                let right_coverage = uncovered.iter().filter(|(_, key)| key == right).count();
                left_coverage
                    .cmp(&right_coverage)
                    .then_with(|| right.cmp(left))
            })
        else {
            break;
        };
        ordinary.push(choice);
        uncovered.retain(|(_, key)| *key != choice);
    }

    let mut retained: Vec<RetainedRow> = state
        .protected
        .iter()
        .map(|key| RetainedRow {
            elapsed_ms: key.elapsed_ms,
            sequence: key.sequence,
            kind: RetentionKind::Protected,
        })
        .chain(ordinary.iter().map(|key| RetainedRow {
            elapsed_ms: key.elapsed_ms,
            sequence: key.sequence,
            kind: RetentionKind::Ordinary,
        }))
        .collect();
    retained.sort_by_key(|row| Key::new(row.elapsed_ms, row.sequence));
    retained.dedup_by_key(|row| (row.elapsed_ms, row.sequence));
    PruneBucketSelection {
        bucket_index: state.index,
        retained,
        uncovered_roles: uncovered.into_iter().map(|(role, _)| role).collect(),
    }
}

fn validate_fields(fields: &[String]) -> Result<(), HistoryError> {
    if !(1..=MAX_FIELDS).contains(&fields.len())
        || fields
            .iter()
            .any(|field| field.is_empty() || field.len() > MAX_FIELD_NAME_BYTES)
        || fields
            .iter()
            .enumerate()
            .any(|(index, field)| fields[..index].contains(field))
    {
        return Err(HistoryError::InvalidFields);
    }
    Ok(())
}

fn validate_gap_policy(policy: GapPolicy) -> Result<(), HistoryError> {
    match policy {
        GapPolicy::Automatic => Ok(()),
        GapPolicy::ExplicitMilliseconds(value)
            if (MIN_GAP_THRESHOLD_MS..=MAX_GAP_THRESHOLD_MS).contains(&value) =>
        {
            Ok(())
        }
        GapPolicy::ExplicitMilliseconds(_) => Err(HistoryError::InvalidGapPolicy),
    }
}

fn automatic_thresholds(cadence: PersistenceCadence) -> Result<(u64, u64), HistoryError> {
    if cadence.default_ms == 0 || cadence.riding_ms == Some(0) || cadence.idle_ms == Some(0) {
        return Err(HistoryError::InvalidCadence);
    }
    if cadence.retained_resolution_seconds > MAX_RETAINED_RESOLUTION_SECONDS {
        return Err(HistoryError::InvalidResolution);
    }
    let resolution_threshold = cadence
        .retained_resolution_seconds
        .checked_mul(3_000)
        .ok_or(HistoryError::ArithmeticOverflow)?;
    let threshold = |persistence: u64| {
        persistence
            .checked_mul(3)
            .map(|value| value.max(1_000).max(resolution_threshold))
            .ok_or(HistoryError::ArithmeticOverflow)
    };
    Ok((
        threshold(cadence.riding_ms.unwrap_or(cadence.default_ms))?,
        threshold(cadence.idle_ms.unwrap_or(cadence.default_ms))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(fields: usize, buckets: usize) -> SeriesConfig {
        SeriesConfig {
            fields: (0..fields).map(|index| format!("field{index}")).collect(),
            start_elapsed_ms: 0,
            end_elapsed_ms: 10_000,
            buckets,
            gap_policy: GapPolicy::Automatic,
            cadence: PersistenceCadence::default(),
        }
    }

    #[test]
    fn generation_exhaustion_is_transactional() {
        let mut accumulator = SeriesAccumulator::new(config(1, 1)).unwrap();
        accumulator.generations[0] = u32::MAX;
        let before = format!("{accumulator:?}");
        assert_eq!(
            accumulator.append(SeriesObservation {
                elapsed_ms: 0,
                sequence: 1,
                ride_phase: RidePhase::Other,
                values: &[None],
            }),
            Err(HistoryError::GenerationExhausted)
        );
        assert_eq!(format!("{accumulator:?}"), before);
        assert!(accumulator.previous_key.is_none());
    }

    #[test]
    fn prune_state_never_retains_ordinary_raw_rows() {
        let mut selector = PruneSelector::new(PruneConfig {
            fields: vec!["value".into()],
            resolution_seconds: 300,
        })
        .unwrap();
        for sequence in 0..10_000 {
            selector
                .append(PruneObservation {
                    elapsed_ms: sequence,
                    sequence,
                    ride_phase: RidePhase::Other,
                    values: &[Some(sequence as f64)],
                    protected: false,
                })
                .unwrap();
        }
        let (protected, candidates) = selector.state_counts();
        assert_eq!(protected, 1);
        assert_eq!(candidates, 4);
    }

    #[test]
    fn protected_bucket_bound_fails_before_mutation() {
        let mut state = PruneBucketState::new(0, Key::new(0, 0), 1);
        for sequence in 0..MAX_PROTECTED_ROWS_PER_BUCKET as u64 {
            state.protect(Key::new(sequence, sequence)).unwrap();
        }
        assert_eq!(
            state.protect(Key::new(u64::MAX, u64::MAX)),
            Err(HistoryError::ProtectedLimit)
        );
        assert_eq!(state.protected.len(), MAX_PROTECTED_ROWS_PER_BUCKET);
    }
}
