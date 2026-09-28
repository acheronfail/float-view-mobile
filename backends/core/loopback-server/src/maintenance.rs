//! Bounded catalog-page maintenance orchestration; durable jobs remain host-owned.
use crate::{
    backend::Backend,
    dispatch::Params,
    facts::{Completion, Maintenance},
    projection,
    protocol::{Failure, Operation},
};
use serde_json::{Value, json};
impl Backend {
    pub(crate) async fn prune(
        &self,
        client: u64,
        p: Params<'_>,
        preview: bool,
    ) -> Result<Value, Failure> {
        let scope = p.text("scope")?;
        if scope != "one" && scope != "all" {
            return Err(Failure::InvalidRequest);
        }
        let session_id = p.nullable("sessionId")?;
        if scope == "one" && session_id.is_none() {
            return Err(Failure::InvalidRequest);
        }
        let resolution = p.number("targetResolutionSeconds")?;
        if !(1..=300).contains(&resolution) {
            return Err(Failure::InvalidRequest);
        }
        if self.service.is_some() {
            return self
                .prune_service(
                    session_id.as_deref().filter(|_| scope == "one"),
                    resolution,
                    preview,
                )
                .await;
        }
        let kind = if preview { "prune_preview" } else { "prune" };
        let previous = match self.call(client, Operation::GetMaintenance).await? {
            Completion::Maintenance(job) => job,
            _ => return Err(Failure::Internal),
        };
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| Failure::Internal)?;
        let mut job = previous
            .filter(|j| {
                j.state == "pending"
                    && j.kind == kind
                    && j.scope == scope
                    && j.session_id == session_id
                    && j.target_resolution_seconds == resolution
            })
            .unwrap_or(Maintenance {
                operation_id: random.iter().map(|b| format!("{b:02x}")).collect(),
                kind: kind.into(),
                scope: scope.clone(),
                state: "pending".into(),
                session_id: session_id.clone(),
                target_resolution_seconds: resolution,
                upper_session_id: None,
                last_session_id: None,
                completed_sessions: 0,
                skipped_sessions: 0,
                retained_samples: 0,
                reclaimable_bytes: 0,
                error: None,
            });
        let mut resume = job.last_session_id.clone();
        let mut cursor = None;
        let mut recorded = 0_u64;
        let mut stored = 0_u64;
        let mut reclaimable = 0_u64;
        let mut skipped = true;
        let mut reason = None;
        self.call(client, Operation::SaveMaintenance { job: job.clone() })
            .await?;
        loop {
            let (ids, next) = if scope == "one" {
                (
                    vec![session_id.clone().ok_or(Failure::InvalidRequest)?],
                    None,
                )
            } else {
                let Completion::Sessions(page) = self
                    .call(
                        client,
                        Operation::ListSessions {
                            history_id: None,
                            limit: 100,
                            cursor: cursor.clone(),
                            completed_only: true,
                        },
                    )
                    .await?
                else {
                    return Err(Failure::Internal);
                };
                if page.items.len() > 100
                    || page.next_cursor.is_some() && page.next_cursor == cursor
                {
                    return Err(Failure::Internal);
                }
                (
                    page.items.into_iter().map(|s| s.session_id).collect(),
                    page.next_cursor,
                )
            };
            for id in ids {
                if let Some(last) = &resume {
                    if last == &id {
                        resume = None;
                    }
                    continue;
                }
                if let Completion::Maintenance(Some(current)) =
                    self.call(client, Operation::GetMaintenance).await?
                {
                    if current.operation_id != job.operation_id {
                        return Err(Failure::Conflict);
                    }
                    if current.state == "cancel_requested" {
                        job.state = "cancelled".into();
                        break;
                    }
                }
                let Completion::PrunePreview(result) = self
                    .call(
                        client,
                        Operation::PruneSession {
                            id: id.clone(),
                            resolution,
                            preview,
                        },
                    )
                    .await?
                else {
                    return Err(Failure::Internal);
                };
                recorded = recorded
                    .checked_add(result.recorded_sample_count)
                    .ok_or(Failure::Internal)?;
                stored = stored
                    .checked_add(result.estimated_stored_sample_count)
                    .ok_or(Failure::Internal)?;
                reclaimable = reclaimable
                    .checked_add(result.estimated_reclaimable_bytes.unwrap_or(0))
                    .ok_or(Failure::Internal)?;
                skipped &= result.skipped;
                if result.skip_reason.is_some() {
                    reason = result.skip_reason;
                }
                job.last_session_id = Some(id.clone());
                job.upper_session_id = Some(id);
                job.completed_sessions = job
                    .completed_sessions
                    .checked_add(1)
                    .ok_or(Failure::Internal)?;
                job.skipped_sessions = job
                    .skipped_sessions
                    .checked_add(u64::from(result.skipped))
                    .ok_or(Failure::Internal)?;
                job.retained_samples = stored;
                job.reclaimable_bytes = reclaimable;
                self.call(client, Operation::SaveMaintenance { job: job.clone() })
                    .await?;
            }
            if job.state == "cancelled" || next.is_none() {
                break;
            }
            cursor = next;
        }
        if job.state != "cancelled" {
            job.state = "completed".into();
        }
        self.call(client, Operation::SaveMaintenance { job: job.clone() })
            .await?;
        let operation = projection::maintenance(&job);
        if preview {
            Ok(
                json!({"operation":operation,"recordedSampleCount":recorded,"estimatedStoredSampleCount":stored,"estimatedReclaimableBytes":reclaimable,"skipped":skipped,"skipReason":reason}),
            )
        } else {
            Ok(operation)
        }
    }
}
