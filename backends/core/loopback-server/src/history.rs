//! Historical requests are validated before catalog/storage operations.
use crate::{
    backend::Backend,
    dispatch::{Params, cursor, decode_cursor},
    facts::{Completion, HistoryResult},
    projection,
    protocol::{Failure, HistoryQuery, Operation, PublishedBoundary, Range},
};
use serde_json::{Value, json};
impl Backend {
    pub(crate) async fn history(
        &self,
        client: u64,
        request: Value,
        permit: std::sync::Arc<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<Value, Failure> {
        let p = Params(request.as_object().ok_or(Failure::InvalidRequest)?);
        p.only(&[
            "kind",
            "sessionId",
            "snapshot",
            "range",
            "fields",
            "buckets",
            "gapPolicy",
            "mode",
            "limit",
            "cursor",
        ])?;
        let kind = p.text("kind")?;
        if !["series", "samples", "events", "locations", "markers"].contains(&kind.as_str()) {
            return Err(Failure::InvalidRequest);
        }
        let id = p.text("sessionId")?;
        let snapshot: PublishedBoundary = serde_json::from_value(request["snapshot"].clone())
            .map_err(|_| Failure::InvalidRequest)?;
        let range: Range = serde_json::from_value(request["range"].clone())
            .map_err(|_| Failure::InvalidRequest)?;
        let max = 9_007_199_254_740_991;
        if range.start_elapsed_ms > range.end_elapsed_ms
            || range.end_elapsed_ms > max
            || [
                snapshot.history_revision,
                snapshot.committed_sample_sequence,
                snapshot.committed_event_sequence,
                snapshot.committed_elapsed_ms,
                snapshot.committed_location_sequence,
                snapshot.committed_location_elapsed_ms,
            ]
            .iter()
            .any(|n| *n > max)
        {
            return Err(Failure::InvalidRequest);
        }
        let fields = if kind == "series" || kind == "samples" {
            let fields: Vec<String> = serde_json::from_value(request["fields"].clone())
                .map_err(|_| Failure::InvalidRequest)?;
            if fields.is_empty()
                || fields.len() > 16
                || fields.iter().any(|f| f.is_empty() || f.len() > 64)
                || fields
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != fields.len()
            {
                return Err(Failure::InvalidRequest);
            }
            fields
        } else {
            Vec::new()
        };
        let (mut buckets, mut gap_seconds, mut limit) = (0, None, 0);
        if kind == "series" {
            buckets = p.number("buckets")? as usize;
            if !(1..=1024).contains(&buckets) {
                return Err(Failure::InvalidRequest);
            }
            let policy = request["gapPolicy"]
                .as_object()
                .ok_or(Failure::InvalidRequest)?;
            let gap = policy
                .get("thresholdSeconds")
                .ok_or(Failure::InvalidRequest)?;
            if !gap.is_null() {
                let gap = gap
                    .as_f64()
                    .filter(|v| (0.1..=3600.0).contains(v))
                    .ok_or(Failure::InvalidRequest)?;
                gap_seconds = Some(gap);
            }
        } else {
            limit = p.number("limit")? as usize;
            let maximum = if kind == "markers" {
                256
            } else if kind == "events" {
                500
            } else {
                2000
            };
            if !(1..=maximum).contains(&limit) {
                return Err(Failure::InvalidRequest);
            }
            if kind == "samples" && p.text("mode")? != "window" {
                return Err(Failure::InvalidRequest);
            }
        }
        let raw_cursor = p.nullable("cursor")?;
        if raw_cursor.as_ref().is_some_and(|c| c.len() > 512) {
            return Err(Failure::InvalidRequest);
        }
        let mut after_elapsed_ms = None;
        let mut after_sequence = None;
        if kind != "locations"
            && let Some(raw) = &raw_cursor
        {
            let c = decode_cursor(raw, 512)?;
            if (kind == "markers" && c["k"] != kind)
                || c["v"] != 1
                || c["s"] != id
                || c["b"] != request["snapshot"]
                || c["r"] != request["range"]
            {
                return Err(Failure::InvalidRequest);
            }
            after_elapsed_ms = Some(c["e"].as_u64().ok_or(Failure::InvalidRequest)?);
            after_sequence = Some(c["q"].as_u64().ok_or(Failure::InvalidRequest)?);
        }
        if kind == "locations"
            && let Some(raw) = &raw_cursor
        {
            let c = decode_cursor(raw, 512)?;
            if c["sessionId"] != id
                || c["startElapsedMs"] != range.start_elapsed_ms
                || c["endElapsedMs"] != range.end_elapsed_ms
                || c["historyRevision"] != snapshot.history_revision
                || c["locationSequence"]
                    .as_u64()
                    .is_none_or(|n| n > snapshot.committed_location_sequence)
            {
                return Err(Failure::InvalidRequest);
            }
        }
        let Completion::Session(source) = self
            .call(client, Operation::GetSession { id: id.clone() })
            .await?
        else {
            return Err(Failure::Internal);
        };
        if projection::boundary(&source.boundary) != request["snapshot"] {
            return Err(Failure::InvalidRequest);
        }
        let query = HistoryQuery {
            kind: kind.clone(),
            snapshot,
            range,
            fields: fields.clone(),
            limit,
            buckets,
            gap_seconds,
            after_elapsed_ms,
            after_sequence,
            location_cursor: if kind == "locations" {
                raw_cursor
            } else {
                None
            },
        };
        let (metadata, result, more) =
            crate::reading::read(source, self.service_lease(&id)?, query, permit).await?;
        let mut response = json!({"kind":kind,"sessionId":id,"snapshot":request["snapshot"],"range":request["range"]});
        let make_cursor = |elapsed: u64, sequence: u64| -> Result<String, Failure> {
            let encoded = cursor(
                &json!({"v":1,"k":kind,"s":id,"b":request["snapshot"],"r":request["range"],"e":elapsed,"q":sequence}),
            )?;
            if encoded.len() > 512 {
                return Err(Failure::Internal);
            }
            Ok(encoded)
        };
        match (kind.as_str(), result) {
            ("series", HistoryResult::SeriesColumns(columns)) => {
                let mut count = 0;
                for field in &fields {
                    let column = columns.get(field).ok_or(Failure::Internal)?;
                    let n = column.values.len();
                    count += n;
                    if column.elapsed_ms.len() != n
                        || column.sequence.len() != n
                        || column.generations.len() != n
                    {
                        return Err(Failure::Internal);
                    }
                }
                if count > 65536 || columns.len() != fields.len() {
                    return Err(Failure::Internal);
                }
                response["columns"] = projection::value(&columns);
                response["gapPolicy"] = request["gapPolicy"].clone();
                response["fields"] = projection::descriptors(&fields, &metadata.extension_fields);
            }
            ("samples", HistoryResult::HistoryPoints(mut points)) => {
                points.truncate(limit);
                response["mode"] = json!("window");
                response["fields"] = projection::descriptors(&fields, &metadata.extension_fields);
                response["elapsedMs"] =
                    json!(points.iter().map(|p| p.elapsed_ms).collect::<Vec<_>>());
                response["sequence"] = json!(points.iter().map(|p| p.sequence).collect::<Vec<_>>());
                response["values"] = json!(
                    fields
                        .iter()
                        .map(|f| (
                            f,
                            points
                                .iter()
                                .map(|p| p.values.get(f).copied().flatten())
                                .collect::<Vec<_>>()
                        ))
                        .collect::<std::collections::BTreeMap<_, _>>()
                );
                response["nextCursor"] = if more {
                    let last = points.last().ok_or(Failure::Internal)?;
                    json!(make_cursor(last.elapsed_ms, last.sequence)?)
                } else {
                    Value::Null
                };
            }
            ("markers", HistoryResult::RideMarkers(markers)) => {
                response["nextCursor"] = if more {
                    let last = markers.last().ok_or(Failure::Internal)?;
                    json!(make_cursor(last.elapsed_ms, last.sequence)?)
                } else {
                    Value::Null
                };
                response["markers"] = projection::value(&markers);
            }
            ("events", HistoryResult::HistoryEvents(mut events)) => {
                events.truncate(limit);
                let sequence = |e: &crate::facts::HistoryEvent| {
                    e.event_id.parse::<u64>().map_err(|_| Failure::Internal)
                };
                response["page"] = json!({"elapsedMs":events.iter().map(|e|e.elapsed_ms).collect::<Vec<_>>(),"sequence":events.iter().map(sequence).collect::<Result<Vec<_>,_>>()?,"sourceSampleSequence":events.iter().map(|e|e.source_sample_sequence.unwrap_or(0)).collect::<Vec<_>>(),"kinds":events.iter().map(|e|&e.kind).collect::<Vec<_>>(),"values":events.iter().map(|e|serde_json::from_str::<Value>(&e.details_json).ok().and_then(|v|v["value"].as_str().map(str::to_owned)).unwrap_or_default()).collect::<Vec<_>>()});
                response["nextCursor"] = if more {
                    let last = events.last().ok_or(Failure::Internal)?;
                    json!(make_cursor(last.elapsed_ms, sequence(last)?)?)
                } else {
                    Value::Null
                };
            }
            ("locations", HistoryResult::Locations(page)) => {
                if page.points.len() > limit
                    || page.gaps.len() > 2000
                    || page.next_cursor.as_ref().is_some_and(|c| c.len() > 512)
                {
                    return Err(Failure::Internal);
                }
                response["points"] = projection::value(&page.points);
                response["gaps"] = projection::value(&page.gaps);
                response["nextCursor"] = json!(page.next_cursor);
                response["locationBoundary"] = json!({"lastSequence":page.snapshot.committed_location_sequence,"lastElapsedMs":page.snapshot.committed_location_elapsed_ms});
            }
            _ => return Err(Failure::Internal),
        }
        Ok(response)
    }
}
