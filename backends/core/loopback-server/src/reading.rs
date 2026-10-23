//! Read-only session leases use the existing runtime reader without its mutable owner.
use crate::{
    facts::*,
    projection,
    protocol::{Failure, HistoryQuery},
};
use float_view_application_runtime::session::{self, SessionReader};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
struct Cancel(Arc<AtomicBool>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
async fn run<T: Send + 'static>(
    source: SessionSource,
    lease: Option<float_view_backend_service::SessionLease>,
    permit: Arc<tokio::sync::OwnedSemaphorePermit>,
    operation: impl FnOnce(&mut SessionReader, SessionMetadata) -> Result<T, Failure> + Send + 'static,
) -> Result<T, Failure> {
    if source.path.len() > 4096 || !std::path::Path::new(&source.path).is_absolute() {
        return Err(Failure::Internal);
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let _guard = Cancel(cancelled.clone());
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let _lease = lease;
        let mut reader = SessionReader::open_cancellable(&source.path, cancelled)
            .map_err(|_| Failure::Unavailable)?;
        let metadata = reader.metadata().map_err(|_| Failure::Unavailable)?;
        let recorded_fields: Vec<_> = reader
            .fields()
            .map_err(|_| Failure::Unavailable)?
            .into_iter()
            .filter(|field| field.first_sequence <= source.boundary.committed_sample_sequence)
            .collect();
        let extension_fields = recorded_fields
            .iter()
            .filter(|field| field.source == session::FieldSource::Extension)
            .map(|field| field.name.clone())
            .collect();
        let recorded_fields = recorded_fields
            .into_iter()
            .map(|field| field.name)
            .collect();
        let prune = PruneMetadata {
            target_resolution_seconds: (metadata.retained_resolution_seconds > 0)
                .then_some(metadata.retained_resolution_seconds),
            recorded_sample_count: source.boundary.recorded_sample_count,
            stored_sample_count: source.boundary.stored_sample_count,
            policy: metadata.prune_policy,
        };
        let marker_count = reader
            .ride_markers(session::PublishedBoundary {
                history_revision: source.boundary.history_revision,
                sample_sequence: source.boundary.committed_sample_sequence,
                event_sequence: source.boundary.committed_event_sequence,
                location_sequence: source.boundary.committed_location_sequence,
            })
            .map_err(|_| Failure::Unavailable)?
            .len();
        let connection_context_count = reader
            .connection_contexts(session::PublishedBoundary {
                history_revision: source.boundary.history_revision,
                sample_sequence: source.boundary.committed_sample_sequence,
                event_sequence: source.boundary.committed_event_sequence,
                location_sequence: source.boundary.committed_location_sequence,
            })
            .map_err(|_| Failure::Unavailable)?
            .len();
        let metadata = SessionMetadata {
            connection_context_count,
            marker_count,
            summary: source.summary,
            recorded_fields,
            extension_fields,
            boundary: source.boundary,
            prune,
        };
        operation(&mut reader, metadata)
    })
    .await
    .map_err(|_| Failure::Internal)?
}
pub async fn metadata(
    source: SessionSource,
    lease: Option<float_view_backend_service::SessionLease>,
    permit: Arc<tokio::sync::OwnedSemaphorePermit>,
) -> Result<SessionMetadata, Failure> {
    run(source, lease, permit, |_, metadata| Ok(metadata)).await
}
pub async fn read(
    source: SessionSource,
    lease: Option<float_view_backend_service::SessionLease>,
    query: HistoryQuery,
    permit: Arc<tokio::sync::OwnedSemaphorePermit>,
) -> Result<(SessionMetadata, HistoryResult, bool), Failure> {
    run(source, lease, permit, move |reader, metadata| {
        if query.fields.iter().any(|field| {
            !projection::NUMERIC_FIELDS.contains(&field.as_str())
                && !metadata.extension_fields.contains(field)
        }) {
            return Err(Failure::InvalidRequest);
        }
        let boundary = session::PublishedBoundary {
            history_revision: query.snapshot.history_revision,
            sample_sequence: query.snapshot.committed_sample_sequence,
            event_sequence: query.snapshot.committed_event_sequence,
            location_sequence: query.snapshot.committed_location_sequence,
        };
        let start = query.range.start_elapsed_ms;
        let end = query.range.end_elapsed_ms;
        let after =
            query
                .after_elapsed_ms
                .zip(query.after_sequence)
                .map(|(elapsed_ms, sequence)| session::SampleKey {
                    elapsed_ms,
                    sequence,
                });
        let mut more = false;
        let value = match query.kind.as_str() {
            "samples" => {
                let page = reader
                    .sample_page(&query.fields, start, end, query.limit, after, boundary)
                    .map_err(|_| Failure::InvalidRequest)?;
                more = page.next.is_some();
                HistoryResult::HistoryPoints(
                    page.rows
                        .into_iter()
                        .map(|row| HistoryPoint {
                            elapsed_ms: row.key.elapsed_ms,
                            sequence: row.key.sequence,
                            values: query.fields.iter().cloned().zip(row.values).collect(),
                        })
                        .collect(),
                )
            }
            "connectionContexts" => {
                let mut rows: Vec<_> = reader
                    .connection_contexts(boundary)
                    .map_err(|_| Failure::Unavailable)?
                    .into_iter()
                    .filter(|e| {
                        e.elapsed_ms >= start
                            && e.elapsed_ms <= end
                            && after.is_none_or(|a| {
                                (e.elapsed_ms, e.sequence) > (a.elapsed_ms, a.sequence)
                            })
                    })
                    .collect();
                more = rows.len() > query.limit;
                rows.truncate(query.limit);
                HistoryResult::ConnectionContexts(rows)
            }
            "markers" => {
                let mut rows: Vec<_> = reader
                    .ride_markers(boundary)
                    .map_err(|_| Failure::InvalidRequest)?
                    .into_iter()
                    .filter(|m| {
                        m.elapsed_ms >= start
                            && m.elapsed_ms <= end
                            && after.is_none_or(|a| {
                                (m.elapsed_ms, m.sequence) > (a.elapsed_ms, a.sequence)
                            })
                    })
                    .collect();
                more = rows.len() > query.limit;
                rows.truncate(query.limit);
                HistoryResult::RideMarkers(rows)
            }
            "events" => {
                let page = reader
                    .event_page(start, end, query.limit, after, boundary)
                    .map_err(|_| Failure::InvalidRequest)?;
                more = page.next.is_some();
                HistoryResult::HistoryEvents(
                    page.rows
                        .into_iter()
                        .map(|row| HistoryEvent {
                            event_id: row.key.sequence.to_string(),
                            elapsed_ms: row.key.elapsed_ms,
                            source_sample_sequence: row.source_sample_sequence,
                            kind: row.kind,
                            details_json: row.details_json,
                        })
                        .collect(),
                )
            }
            "series" => {
                let policy = match query.gap_seconds {
                    Some(seconds) => {
                        float_view_application_runtime::history::GapPolicy::explicit_seconds(
                            seconds,
                        )
                        .map_err(|_| Failure::InvalidRequest)?
                    }
                    None => float_view_application_runtime::history::GapPolicy::Automatic,
                };
                let result = reader
                    .reduce_series(&query.fields, start, end, query.buckets, policy, boundary)
                    .map_err(|_| Failure::InvalidRequest)?;
                HistoryResult::SeriesColumns(
                    result
                        .columns
                        .into_iter()
                        .map(|column| {
                            (
                                column.field,
                                SeriesColumn {
                                    elapsed_ms: column.elapsed_ms,
                                    sequence: column.sequences,
                                    values: column.values.into_iter().map(Some).collect(),
                                    generations: column.generations,
                                },
                            )
                        })
                        .collect(),
                )
            }
            "locations" => {
                let after = query
                    .location_cursor
                    .as_deref()
                    .map(|raw| crate::dispatch::decode_cursor(raw, 512))
                    .transpose()?
                    .map(|value| {
                        value["locationSequence"]
                            .as_u64()
                            .ok_or(Failure::InvalidRequest)
                    })
                    .transpose()?;
                let page = reader
                    .location_page(start, end, query.limit, after, boundary)
                    .map_err(|_| Failure::InvalidRequest)?;
                let next_cursor = page
                    .next_location_sequence
                    .map(|sequence| {
                        crate::dispatch::cursor(&serde_json::json!({
                         "sessionId": metadata.summary.session_id,
                         "startElapsedMs": start,
                         "endElapsedMs": end,
                         "historyRevision": boundary.history_revision,
                         "locationSequence": sequence
                        }))
                    })
                    .transpose()?;
                let points = page
                    .rows
                    .into_iter()
                    .map(|row| {
                        Ok(LocationPoint {
                            location_sequence: row.location_sequence,
                            elapsed_ms: row.elapsed_ms,
                            latitude: row.latitude,
                            longitude: row.longitude,
                            accuracy_m: row.accuracy_m,
                            observed_at: time::OffsetDateTime::from_unix_timestamp_nanos(
                                i128::from(row.observed_at_epoch_ms) * 1_000_000,
                            )
                            .map_err(|_| Failure::InvalidRequest)?
                            .format(&time::format_description::well_known::Rfc3339)
                            .map_err(|_| Failure::Internal)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Failure>>()?;
                HistoryResult::Locations(LocationPage {
                    points,
                    gaps: page
                        .gaps
                        .into_iter()
                        .map(|gap| LocationGap {
                            start_elapsed_ms: gap.start_elapsed_ms,
                            end_elapsed_ms: gap.end_elapsed_ms,
                            reason: "noFix".into(),
                        })
                        .collect(),
                    next_cursor,
                    snapshot: metadata.boundary.clone(),
                })
            }
            _ => return Err(Failure::InvalidRequest),
        };
        Ok((metadata, value, more))
    })
    .await
}
