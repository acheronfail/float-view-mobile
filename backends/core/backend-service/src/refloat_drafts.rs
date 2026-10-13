//! Editor checkpoints are product storage, independent of transport lifetimes.
use crate::{BackendService, facts::ConfigurationIdentity, operations::Failure, store_failure};
use float_view_product_store::RefloatEditorDraft;

impl BackendService {
    fn draft_controller(&self, device_id: &str) -> Result<String, Failure> {
        let active = self
            .policy
            .active
            .as_ref()
            .filter(|a| a.device.id == device_id && a.state == crate::os::LinkState::Ready)
            .ok_or(Failure::InvalidState)?;
        active
            .refloat_draft_key
            .clone()
            .ok_or(Failure::InvalidState)
    }
    pub(crate) fn clear_connection_editor_state(&mut self) -> Result<(), Failure> {
        let Some(active) = self.policy.active.as_ref() else {
            return Ok(());
        };
        if let Some(key) = &active.refloat_draft_key {
            self.store
                .delete_refloat_editor_draft(key)
                .map_err(store_failure)?;
        }
        self.clear_connection_checkpoint(&active.device.id.clone());
        Ok(())
    }
    pub fn refloat_editor_draft(
        &self,
        device_id: &str,
    ) -> Result<Option<RefloatEditorDraft>, Failure> {
        self.store
            .refloat_editor_draft(&self.draft_controller(device_id)?)
            .map_err(store_failure)
    }
    pub fn set_refloat_editor_draft(
        &mut self,
        device_id: &str,
        revision: u64,
        draft: RefloatEditorDraft,
    ) -> Result<(), Failure> {
        draft.validate().map_err(store_failure)?;
        let key = self.draft_controller(device_id)?;
        self.validate_configuration_source(
            &ConfigurationIdentity {
                package_name: draft.identity.package_name.clone(),
                package_version: draft.identity.package_version.clone(),
                schema_fingerprint: draft.identity.schema_fingerprint.clone(),
                wire_signature: draft.identity.wire_signature,
            },
            revision,
        )?;
        self.store
            .set_refloat_editor_draft(&key, &draft, self.policy.exchange.epoch)
            .map_err(store_failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn a_new_process_clears_editor_sessions_without_a_live_connection() {
        let root =
            std::env::temp_dir().join(format!("float-view-editor-lifetime-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let key = "07".repeat(12);
        {
            let mut service = BackendService::open(&root).unwrap();
            let draft = serde_json::from_value(json!({
                "version":1,"identity":{"packageName":"refloat","packageVersion":"1.3.0","schemaFingerprint":"a".repeat(64),"wireSignature":123},
                "source":"current","values":{"gain":{"type":"double","value":1.2}},"inputs":{},
                "position":{"sectionKey":"Ride","search":"gain","controlsExpanded":false,"scrollTop":123}
            })).unwrap();
            service
                .store
                .set_refloat_editor_draft(&key, &draft, 10)
                .unwrap();
        }
        {
            let service = BackendService::open(&root).unwrap();
            assert!(service.store.refloat_editor_draft(&key).unwrap().is_none());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
