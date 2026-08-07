#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::InterruptHandle;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    CatalogStore, ElapsedRange, HistoryCursor, MAX_EVENT_ROWS, MAX_HISTORY_CURSOR_BYTES,
    MAX_LOCATION_ROWS, MAX_SAMPLE_PAGE_ROWS, MAX_SERIES_BUCKETS, MAX_SERIES_FIELDS,
    MAX_SOURCE_ROWS, PublishedBoundary, SessionReader, StoreError, decode_history_cursor,
    encode_history_cursor, page_series, reduce_series,
};

pub const MAX_SESSION_DATA_REQUEST_BYTES: usize = 16_384;
pub const MAX_SESSION_DATA_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Default)]
pub struct SessionDataCancellation {
    cancelled: AtomicBool,
    interrupt: Mutex<Option<InterruptHandle>>,
}

impl SessionDataCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        if let Ok(interrupt) = self.interrupt.lock()
            && let Some(interrupt) = interrupt.as_ref()
        {
            interrupt.interrupt();
        }
    }

    fn attach(&self, interrupt: InterruptHandle) -> Result<(), StoreError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(StoreError::Invalid);
        }
        *self.interrupt.lock().map_err(|_| StoreError::Poisoned)? = Some(interrupt);
        if self.cancelled.load(Ordering::Acquire) {
            self.cancel();
            return Err(StoreError::Invalid);
        }
        Ok(())
    }

    fn detach(&self) {
        if let Ok(mut interrupt) = self.interrupt.lock() {
            *interrupt = None;
        }
    }

    fn check(&self) -> Result<(), StoreError> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(StoreError::Invalid)
        } else {
            Ok(())
        }
    }
}

pub struct SessionDataService {
    catalog: Arc<CatalogStore>,
}

impl SessionDataService {
    #[must_use]
    pub const fn new(catalog: Arc<CatalogStore>) -> Self {
        Self { catalog }
    }

    pub fn query_json(
        &self,
        body: &[u8],
        cancellation: &SessionDataCancellation,
    ) -> Result<Vec<u8>, StoreError> {
        if body.is_empty() || body.len() > MAX_SESSION_DATA_REQUEST_BYTES {
            return Err(StoreError::Invalid);
        }
        let request: SessionDataRequest =
            serde_json::from_slice(body).map_err(|_| StoreError::Invalid)?;
        request.validate()?;
        cancellation.check()?;
        let reader = self
            .catalog
            .acquire_read_lease(&request.session_id)?
            .open()?;
        cancellation.attach(reader.interrupt_handle())?;
        let result = query(&reader, &request, cancellation);
        cancellation.detach();
        let value = result?;
        let encoded = serde_json::to_vec(&value).map_err(|_| StoreError::Invalid)?;
        if encoded.len() > MAX_SESSION_DATA_RESPONSE_BYTES {
            return Err(StoreError::Invalid);
        }
        Ok(encoded)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    history_revision: u64,
    committed_sample_sequence: u64,
    committed_event_sequence: u64,
    committed_elapsed_ms: u64,
    committed_location_sequence: u64,
    committed_location_elapsed_ms: u64,
}

impl Snapshot {
    fn boundary(self) -> PublishedBoundary {
        PublishedBoundary {
            sample_sequence: self.committed_sample_sequence,
            event_sequence: self.committed_event_sequence,
            location_sequence: self.committed_location_sequence,
            history_revision: self.history_revision,
            sample_elapsed_millis: self.committed_elapsed_ms,
            location_elapsed_millis: self.committed_location_elapsed_ms,
        }
    }

    fn valid(self) -> bool {
        [
            self.history_revision,
            self.committed_sample_sequence,
            self.committed_event_sequence,
            self.committed_elapsed_ms,
            self.committed_location_sequence,
            self.committed_location_elapsed_ms,
        ]
        .into_iter()
        .all(|value| value <= MAX_SAFE_INTEGER)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Range {
    start_elapsed_ms: u64,
    end_elapsed_ms: u64,
}

impl Range {
    const fn storage(self) -> ElapsedRange {
        ElapsedRange {
            start_elapsed_millis: self.start_elapsed_ms,
            end_elapsed_millis: self.end_elapsed_ms,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GapPolicy {
    threshold_seconds: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionDataRequest {
    kind: String,
    session_id: String,
    snapshot: Snapshot,
    range: Range,
    #[serde(default)]
    fields: Vec<String>,
    buckets: Option<usize>,
    gap_policy: Option<GapPolicy>,
    mode: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
}

impl SessionDataRequest {
    fn validate(&self) -> Result<(), StoreError> {
        if self.session_id.is_empty()
            || self.session_id.len() > 128
            || !self.snapshot.valid()
            || self.range.start_elapsed_ms > self.range.end_elapsed_ms
            || self.range.end_elapsed_ms > MAX_SAFE_INTEGER
            || self
                .cursor
                .as_ref()
                .is_some_and(|cursor| cursor.is_empty() || cursor.len() > MAX_HISTORY_CURSOR_BYTES)
        {
            return Err(StoreError::Invalid);
        }
        match self.kind.as_str() {
            "series" => {
                validate_fields(&self.fields)?;
                if !self
                    .buckets
                    .is_some_and(|value| (1..=MAX_SERIES_BUCKETS).contains(&value))
                    || self.gap_policy.as_ref().is_none_or(|policy| {
                        policy.threshold_seconds.is_some_and(|value| {
                            !value.is_finite() || !(0.1..=3_600.0).contains(&value)
                        })
                    })
                {
                    return Err(StoreError::Invalid);
                }
            }
            "samples" => {
                validate_fields(&self.fields)?;
                if self.mode.as_deref() != Some("window")
                    || !self
                        .limit
                        .is_some_and(|value| (1..=MAX_SAMPLE_PAGE_ROWS).contains(&value))
                {
                    return Err(StoreError::Invalid);
                }
            }
            "events" => {
                if !self
                    .limit
                    .is_some_and(|value| (1..=MAX_EVENT_ROWS).contains(&value))
                {
                    return Err(StoreError::Invalid);
                }
            }
            "locations" => {
                if !self
                    .limit
                    .is_some_and(|value| (1..=MAX_LOCATION_ROWS).contains(&value))
                {
                    return Err(StoreError::Invalid);
                }
            }
            _ => return Err(StoreError::Invalid),
        }
        Ok(())
    }
}

fn validate_fields(fields: &[String]) -> Result<(), StoreError> {
    if fields.is_empty()
        || fields.len() > MAX_SERIES_FIELDS
        || fields
            .iter()
            .any(|field| field.is_empty() || field.len() > 64)
        || fields.iter().collect::<HashSet<_>>().len() != fields.len()
    {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn query(
    reader: &SessionReader,
    request: &SessionDataRequest,
    cancellation: &SessionDataCancellation,
) -> Result<Value, StoreError> {
    if reader.boundary() != request.snapshot.boundary() {
        return Err(StoreError::Conflict {
            current_revision: reader.boundary().history_revision,
        });
    }
    let descriptors = request
        .fields
        .iter()
        .map(|field| Ok(json!({ "name": field, "source": reader.field_source(field)? })))
        .collect::<Result<Vec<_>, StoreError>>()?;
    cancellation.check()?;
    match request.kind.as_str() {
        "series" => series(reader, request, descriptors),
        "samples" => samples(reader, request, descriptors),
        "events" => events(reader, request),
        "locations" => locations(reader, request),
        _ => Err(StoreError::Invalid),
    }
}

fn base(request: &SessionDataRequest) -> Map<String, Value> {
    let mut output = Map::new();
    output.insert("kind".into(), json!(request.kind));
    output.insert("sessionId".into(), json!(request.session_id));
    output.insert("snapshot".into(), json!(request.snapshot));
    output.insert("range".into(), json!(request.range));
    output
}

fn series(
    reader: &SessionReader,
    request: &SessionDataRequest,
    descriptors: Vec<Value>,
) -> Result<Value, StoreError> {
    let range = request.range.storage();
    let rows = reader.query_series(range, &request.fields, MAX_SOURCE_ROWS)?;
    let configuration = reader.history_configuration()?;
    let threshold = request
        .gap_policy
        .as_ref()
        .ok_or(StoreError::Invalid)?
        .threshold_seconds;
    let columns = reduce_series(
        &rows,
        request.fields.clone(),
        range,
        request.buckets.ok_or(StoreError::Invalid)?,
        threshold,
        1_000,
        Some(configuration.riding_persistence_millis),
        Some(configuration.idle_persistence_millis),
        configuration.retained_resolution_seconds,
    )?;
    let mut output = base(request);
    output.insert("gapPolicy".into(), json!({ "thresholdSeconds": threshold }));
    output.insert("fields".into(), Value::Array(descriptors));
    output.insert(
        "columns".into(),
        Value::Object(
            columns
                .into_iter()
                .map(|(field, column)| {
                    (
                        field,
                        json!({
                            "elapsedMs": column.elapsed_millis,
                            "sequence": column.sequence,
                            "values": column.values,
                            "generations": column.generations,
                        }),
                    )
                })
                .collect(),
        ),
    );
    Ok(Value::Object(output))
}

fn cursor(
    request: &SessionDataRequest,
    raw: Option<&str>,
) -> Result<Option<HistoryCursor>, StoreError> {
    let Some(raw) = raw else { return Ok(None) };
    let cursor = decode_history_cursor(raw)?;
    if cursor.session_id != request.session_id
        || cursor.boundary != request.snapshot.boundary()
        || cursor.range != request.range.storage()
    {
        return Err(StoreError::Invalid);
    }
    Ok(Some(cursor))
}

fn seed(request: &SessionDataRequest) -> HistoryCursor {
    HistoryCursor {
        version: 1,
        session_id: request.session_id.clone(),
        boundary: request.snapshot.boundary(),
        range: request.range.storage(),
        elapsed_millis: request.range.start_elapsed_ms,
        sequence: 0,
    }
}

fn samples(
    reader: &SessionReader,
    request: &SessionDataRequest,
    descriptors: Vec<Value>,
) -> Result<Value, StoreError> {
    let rows = reader.query_series(request.range.storage(), &request.fields, MAX_SOURCE_ROWS)?;
    let decoded = cursor(request, request.cursor.as_deref())?;
    let context = seed(request);
    let (points, next) = page_series(
        &rows,
        request.limit.ok_or(StoreError::Invalid)?,
        decoded.as_ref(),
        Some(&context),
    )?;
    let values: BTreeMap<_, _> = request
        .fields
        .iter()
        .map(|field| {
            (
                field.clone(),
                points
                    .iter()
                    .map(|point| point.values.get(field).copied().flatten())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let mut output = base(request);
    output.insert("mode".into(), json!("window"));
    output.insert("fields".into(), Value::Array(descriptors));
    output.insert(
        "elapsedMs".into(),
        json!(
            points
                .iter()
                .map(|point| point.elapsed_millis)
                .collect::<Vec<_>>()
        ),
    );
    output.insert(
        "sequence".into(),
        json!(
            points
                .iter()
                .map(|point| point.sequence)
                .collect::<Vec<_>>()
        ),
    );
    output.insert("values".into(), json!(values));
    output.insert(
        "nextCursor".into(),
        next.as_ref()
            .map(encode_history_cursor)
            .transpose()?
            .map_or(Value::Null, Value::String),
    );
    Ok(Value::Object(output))
}

fn events(reader: &SessionReader, request: &SessionDataRequest) -> Result<Value, StoreError> {
    let decoded = cursor(request, request.cursor.as_deref())?;
    let limit = request.limit.ok_or(StoreError::Invalid)?;
    let mut rows = reader.events_page(
        request.range.storage(),
        decoded
            .as_ref()
            .map(|value| (value.elapsed_millis, value.sequence)),
        limit,
    )?;
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    let next = if has_more {
        rows.last().map(|last| HistoryCursor {
            elapsed_millis: last.elapsed_millis,
            sequence: last.sequence,
            ..seed(request)
        })
    } else {
        None
    };
    let values = rows
        .iter()
        .map(|event| {
            serde_json::from_str::<Value>(&event.details_json)
                .ok()
                .and_then(|value| {
                    value
                        .get("value")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let mut output = base(request);
    output.insert(
        "page".into(),
        json!({
            "elapsedMs": rows.iter().map(|event| event.elapsed_millis).collect::<Vec<_>>(),
            "sequence": rows.iter().map(|event| event.sequence).collect::<Vec<_>>(),
            "sourceSampleSequence": rows.iter().map(|event| event.source_sample_sequence.unwrap_or(0)).collect::<Vec<_>>(),
            "kinds": rows.iter().map(|event| &event.kind).collect::<Vec<_>>(),
            "values": values,
        }),
    );
    output.insert(
        "nextCursor".into(),
        next.as_ref()
            .map(encode_history_cursor)
            .transpose()?
            .map_or(Value::Null, Value::String),
    );
    Ok(Value::Object(output))
}

fn locations(reader: &SessionReader, request: &SessionDataRequest) -> Result<Value, StoreError> {
    let decoded = cursor(request, request.cursor.as_deref())?;
    let limit = request.limit.ok_or(StoreError::Invalid)?;
    let mut rows = reader.locations_page(
        request.range.storage(),
        decoded.as_ref().map_or(0, |value| value.sequence),
        limit,
    )?;
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    let next = if has_more {
        rows.last().map(|last| HistoryCursor {
            elapsed_millis: last.elapsed_millis,
            sequence: last.sequence,
            ..seed(request)
        })
    } else {
        None
    };
    let points = rows
        .iter()
        .map(|point| {
            Ok(json!({
                "locationSequence": point.sequence,
                "elapsedMs": point.elapsed_millis,
                "latitude": point.latitude,
                "longitude": point.longitude,
                "accuracyM": point.accuracy_m,
            "observedAt": format_iso8601(point.observed_at_millis)?,
            }))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let boundary = reader.boundary();
    let mut output = base(request);
    output.insert("points".into(), Value::Array(points));
    output.insert("gaps".into(), json!([]));
    output.insert(
        "nextCursor".into(),
        next.as_ref()
            .map(encode_history_cursor)
            .transpose()?
            .map_or(Value::Null, Value::String),
    );
    output.insert(
        "locationBoundary".into(),
        json!({
            "lastSequence": boundary.location_sequence,
            "lastElapsedMs": boundary.location_elapsed_millis,
        }),
    );
    Ok(Value::Object(output))
}

pub fn format_iso8601(millis: i64) -> Result<String, StoreError> {
    if millis < 0 {
        return Err(StoreError::Invalid);
    }
    let seconds = millis / 1_000;
    let days = seconds / 86_400;
    let second_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        second_of_day / 3_600,
        second_of_day % 3_600 / 60,
        second_of_day % 60,
        millis % 1_000
    ))
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let era_days = days_since_epoch + 719_468;
    let era = era_days / 146_097;
    let day_of_era = era_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewSession, SessionSample, SessionStore};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn root() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "float-view-session-data-{}-{nonce}",
            std::process::id()
        ))
    }

    fn sample(sequence: u64) -> SessionSample {
        SessionSample {
            sequence,
            observed_at_millis: i64::try_from(sequence * 100).unwrap(),
            slow_observed_at_millis: None,
            monotonic_elapsed_millis: sequence * 100,
            ride_state: "riding".into(),
            package_state: Some("running".into()),
            package_mode: Some("normal".into()),
            raw_pack_voltage_v: Some(80.0 - f64::from(u32::try_from(sequence).unwrap())),
            duty_cycle: Some(f64::from(u32::try_from(sequence).unwrap()) / 10.0),
            speed_kph: Some(f64::from(u32::try_from(sequence).unwrap())),
            extension_json: "{}".into(),
        }
    }

    #[test]
    fn timestamp_format_is_stable() {
        assert_eq!(format_iso8601(0).unwrap(), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format_iso8601(1_704_067_200_123).unwrap(),
            "2024-01-01T00:00:00.123Z"
        );
    }

    #[test]
    fn validated_columnar_series_and_sample_pages_bind_the_published_snapshot() {
        let root = root();
        let sessions = SessionStore::new(CatalogStore::open(&root).unwrap());
        sessions
            .create(&NewSession {
                session_id: "session",
                job_id: "job",
                device_history_id: "history",
                device_snapshot_json: "{}",
                profile_snapshot_json: "{}",
                profile_revision: 0,
                started_at_millis: 0,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .unwrap();
        sessions
            .append_samples("session", &[sample(1), sample(2), sample(3)], 300)
            .unwrap();
        sessions.flush_catalog("session").unwrap();
        let service = SessionDataService::new(sessions.catalog_arc());
        let snapshot = json!({
            "historyRevision": 1,
            "committedSampleSequence": 3,
            "committedEventSequence": 0,
            "committedElapsedMs": 300,
            "committedLocationSequence": 0,
            "committedLocationElapsedMs": 0
        });
        let series = json!({
            "kind": "series", "sessionId": "session", "snapshot": snapshot,
            "range": { "startElapsedMs": 0, "endElapsedMs": 300 },
            "fields": ["packVoltageV", "dutyCycle"], "buckets": 2,
            "gapPolicy": { "thresholdSeconds": null }
        });
        let encoded = service
            .query_json(
                &serde_json::to_vec(&series).unwrap(),
                &SessionDataCancellation::default(),
            )
            .unwrap();
        let result: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(result["kind"], "series");
        assert_eq!(result["fields"][0]["source"], "stable");
        assert_eq!(
            result["columns"]["packVoltageV"]["values"]
                .as_array()
                .unwrap()
                .len(),
            3
        );

        let samples = json!({
            "kind": "samples", "sessionId": "session", "snapshot": snapshot,
            "range": { "startElapsedMs": 0, "endElapsedMs": 300 },
            "fields": ["packVoltageV"], "mode": "window", "limit": 2
        });
        let first: Value = serde_json::from_slice(
            &service
                .query_json(
                    &serde_json::to_vec(&samples).unwrap(),
                    &SessionDataCancellation::default(),
                )
                .unwrap(),
        )
        .unwrap();
        assert_eq!(first["sequence"], json!([1, 2]));
        let mut second_request = samples;
        second_request["cursor"] = first["nextCursor"].clone();
        let second: Value = serde_json::from_slice(
            &service
                .query_json(
                    &serde_json::to_vec(&second_request).unwrap(),
                    &SessionDataCancellation::default(),
                )
                .unwrap(),
        )
        .unwrap();
        assert_eq!(second["sequence"], json!([3]));

        let stale = json!({
            "kind": "samples", "sessionId": "session",
            "snapshot": { "historyRevision": 0, "committedSampleSequence": 3,
              "committedEventSequence": 0, "committedElapsedMs": 300,
              "committedLocationSequence": 0, "committedLocationElapsedMs": 0 },
            "range": { "startElapsedMs": 0, "endElapsedMs": 300 },
            "fields": ["packVoltageV"], "mode": "window", "limit": 2
        });
        assert!(matches!(
            service.query_json(
                &serde_json::to_vec(&stale).unwrap(),
                &SessionDataCancellation::default()
            ),
            Err(StoreError::Conflict { .. })
        ));
        drop(service);
        drop(sessions);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancellation_and_request_bounds_fail_closed() {
        let root = root();
        let catalog = Arc::new(CatalogStore::open(&root).unwrap());
        let service = SessionDataService::new(catalog);
        let cancellation = SessionDataCancellation::default();
        cancellation.cancel();
        assert!(matches!(
            service.query_json(b"{}", &cancellation),
            Err(StoreError::Invalid)
        ));
        assert!(matches!(
            service.query_json(
                &vec![b'x'; MAX_SESSION_DATA_REQUEST_BYTES + 1],
                &SessionDataCancellation::default()
            ),
            Err(StoreError::Invalid)
        ));
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }
}
