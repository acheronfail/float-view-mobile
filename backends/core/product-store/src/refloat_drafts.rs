use crate::{Error, ProductStore, Result, TuneIdentity, TuneValue, json};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_BYTES: usize = 60_000;
const MAX_DRAFTS: usize = 128;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefloatEditorDraft {
    pub version: u8,
    pub identity: TuneIdentity,
    pub source: String,
    pub values: BTreeMap<String, TuneValue>,
    pub inputs: BTreeMap<String, String>,
    pub position: RefloatEditorPosition,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefloatEditorPosition {
    pub section_key: String,
    pub search: String,
    pub controls_expanded: bool,
    pub scroll_top: f64,
}
impl RefloatEditorDraft {
    pub fn validate(&self) -> Result<()> {
        self.identity.validate()?;
        fn value(v: &TuneValue) -> bool {
            match v {
                TuneValue::Double(n) => n.is_finite(),
                TuneValue::Integer(n) => n.unsigned_abs() <= 9_007_199_254_740_991,
                TuneValue::String(s) => s.len() <= 4096,
                TuneValue::Boolean(_) => true,
            }
        }
        if self.version != 1
            || !matches!(self.source.as_str(), "current" | "default")
            || self.values.is_empty()
            || self.values.len() > 512
            || self
                .values
                .iter()
                .any(|(name, v)| name.is_empty() || name.len() > 128 || !value(v))
            || self.inputs.len() > 512
            || self
                .inputs
                .iter()
                .any(|(name, s)| !self.values.contains_key(name) || s.len() > 128)
            || self.position.section_key.len() > 1024
            || self.position.search.len() > 512
            || !self.position.scroll_top.is_finite()
            || !(0.0..=10_000_000.0).contains(&self.position.scroll_top)
        {
            return Err(Error::InvalidInput);
        }
        json(self, MAX_BYTES)?;
        Ok(())
    }
}
fn key(key: &str) -> Result<()> {
    if key.len() != 24
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
impl ProductStore {
    pub fn clear_refloat_editor_drafts(&mut self) -> Result<()> {
        self.catalog
            .execute("DELETE FROM refloat_editor_draft", [])?;
        Ok(())
    }
    pub fn delete_refloat_editor_draft(&mut self, controller: &str) -> Result<()> {
        key(controller)?;
        self.catalog.execute(
            "DELETE FROM refloat_editor_draft WHERE controller_key=?",
            [controller],
        )?;
        Ok(())
    }

    pub fn refloat_editor_draft(&self, controller: &str) -> Result<Option<RefloatEditorDraft>> {
        key(controller)?;
        let text: Option<String> = self
            .catalog
            .query_row(
                "SELECT json FROM refloat_editor_draft WHERE controller_key=?",
                [controller],
                |r| r.get(0),
            )
            .optional()?;
        text.map(|text| {
            if text.len() > MAX_BYTES {
                return Err(Error::Corrupt);
            }
            let draft: RefloatEditorDraft =
                serde_json::from_str(&text).map_err(|_| Error::Corrupt)?;
            draft.validate().map_err(|_| Error::Corrupt)?;
            Ok(draft)
        })
        .transpose()
    }
    pub fn set_refloat_editor_draft(
        &mut self,
        controller: &str,
        draft: &RefloatEditorDraft,
        at_ms: i64,
    ) -> Result<()> {
        key(controller)?;
        draft.validate()?;
        crate::boards::timestamp(at_ms)?;
        let tx = self.catalog.transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM refloat_editor_draft WHERE controller_key=?)",
            [controller],
            |r| r.get(0),
        )?;
        let count: usize = tx.query_row("SELECT count(*) FROM refloat_editor_draft", [], |r| {
            r.get(0)
        })?;
        // Preserve existing work when capacity is reached; never silently evict edits.
        if !exists && count >= MAX_DRAFTS {
            return Err(Error::Capacity);
        }
        tx.execute("INSERT INTO refloat_editor_draft(controller_key,json,updated_at_ms) VALUES (?,?,?) ON CONFLICT(controller_key) DO UPDATE SET json=excluded.json,updated_at_ms=excluded.updated_at_ms", params![controller, json(draft, MAX_BYTES)?, at_ms])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn draft() -> RefloatEditorDraft {
        serde_json::from_value(json!({
            "version":1, "identity":{"packageName":"refloat", "packageVersion":"1.3.0",
                "schemaFingerprint":"a".repeat(64),"wireSignature":123},
            "source":"current", "values":{"gain":{"type":"double","value":1.2}}, "inputs":{"gain":""},
            "position":{"sectionKey":"Ride", "search":"gain", "controlsExpanded":false,"scrollTop":123}
        })).unwrap()
    }

    #[test]
    fn durable_drafts_are_controller_scoped_atomic_and_bounded() {
        let root = std::env::temp_dir().join(format!("float-view-drafts-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let first = "01".repeat(12);
        let saved = draft();
        {
            let mut store = ProductStore::open(&root).unwrap();
            store.set_refloat_editor_draft(&first, &saved, 1).unwrap();
            assert!(
                store
                    .refloat_editor_draft(&"02".repeat(12))
                    .unwrap()
                    .is_none()
            );
            let mut bad = saved.clone();
            bad.position.search = "é".repeat(257);
            assert_eq!(
                store.set_refloat_editor_draft(&first, &bad, 2),
                Err(Error::InvalidInput)
            );
            assert_eq!(
                store.refloat_editor_draft(&first).unwrap(),
                Some(saved.clone())
            );
        }
        {
            let mut store = ProductStore::open(&root).unwrap();
            assert_eq!(
                store.refloat_editor_draft(&first).unwrap(),
                Some(saved.clone())
            );
            for i in 2..=MAX_DRAFTS {
                store
                    .set_refloat_editor_draft(&format!("{i:024x}"), &saved, i as i64)
                    .unwrap();
            }
            assert_eq!(
                store.set_refloat_editor_draft(&format!("{:024x}", MAX_DRAFTS + 1), &saved, 200),
                Err(Error::Capacity)
            );
            let mut replacement = saved.clone();
            replacement.source = "default".into();
            replacement
                .values
                .insert("gain".into(), TuneValue::Double(1.0));
            replacement.inputs.clear();
            store
                .set_refloat_editor_draft(&first, &replacement, 201)
                .unwrap();
            assert_eq!(
                store.refloat_editor_draft(&first).unwrap(),
                Some(replacement)
            );
            store
                .catalog
                .execute(
                    "UPDATE refloat_editor_draft SET json='{}' WHERE controller_key=?",
                    [&first],
                )
                .unwrap();
            assert_eq!(store.refloat_editor_draft(&first), Err(Error::Corrupt));
            let text: String = store
                .catalog
                .query_row(
                    "SELECT json FROM refloat_editor_draft WHERE controller_key=?",
                    [&first],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(text, "{}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
