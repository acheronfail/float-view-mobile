//! Bounded catalog-page maintenance orchestration; durable jobs remain host-owned.
use crate::{backend::Backend, dispatch::Params, protocol::Failure};
use serde_json::Value;
impl Backend {
    pub(crate) async fn prune(
        &self,
        _client: u64,
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
        self.prune_service(
            session_id.as_deref().filter(|_| scope == "one"),
            resolution,
            preview,
        )
        .await
    }
}
