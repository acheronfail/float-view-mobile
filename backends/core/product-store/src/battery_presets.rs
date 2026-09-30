//! Service-owned pack choices, independent of the pinned discharge curves.
//! Provenance and nominal-capacity derivations: ../BATTERY_PRESETS.md.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::LazyLock;
pub const CATALOG_VERSION: &str = "packs-2026-09-30-v1";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preset {
    pub id: String,
    pub label: String,
    pub manufacturer: String,
    pub cell_label: String,
    pub series_count: u8,
    pub parallel_count: u8,
    pub battery_cell_profile_id: String,
    pub nominal_capacity_ah: Option<f64>,
}
pub static PRESETS: LazyLock<Vec<Preset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("battery-presets.json")).expect("checked-in battery presets")
});
pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}
impl Preset {
    pub fn matches(&self, values: &Map<String, Value>) -> bool {
        values.get("seriesCount").and_then(Value::as_f64) == Some(f64::from(self.series_count))
            && values.get("parallelCount").and_then(Value::as_f64)
                == Some(f64::from(self.parallel_count))
            && values.get("batteryCellProfileId").and_then(Value::as_str)
                == Some(self.battery_cell_profile_id.as_str())
    }
}
/// Patch commands expand defaults; imported complete records never call this.
pub(crate) fn apply(
    values: &mut Map<String, Value>,
    patch: Map<String, Value>,
) -> crate::Result<()> {
    use crate::Error;
    const FIELDS: &[&str] = &[
        "seriesCount",
        "parallelCount",
        "batteryCellProfileId",
        "batteryCapacityAh",
        "batteryMinimumCellVoltageV",
    ];
    if let Some(id) = patch.get("batteryPresetId").filter(|v| !v.is_null()) {
        let p = id.as_str().and_then(preset).ok_or(Error::InvalidInput)?;
        if FIELDS.iter().any(|key| patch.contains_key(*key)) {
            return Err(Error::InvalidInput);
        }
        values.extend(
            serde_json::json!({
                "seriesCount": p.series_count, "parallelCount": p.parallel_count,
                "batteryCellProfileId": p.battery_cell_profile_id,
                "batteryCapacityAh": p.nominal_capacity_ah, "batteryMinimumCellVoltageV": null
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        values.extend(patch);
    } else {
        let counts_changed = ["seriesCount", "parallelCount"].iter().any(|key| {
            patch
                .get(*key)
                .is_some_and(|new| new.as_f64() != values.get(*key).and_then(Value::as_f64))
        });
        if counts_changed && !patch.contains_key("batteryCapacityAh") {
            values.insert("batteryCapacityAh".into(), Value::Null);
        }
        values.extend(patch);
        if let Some(p) = values
            .get("batteryPresetId")
            .and_then(Value::as_str)
            .and_then(preset)
            && !p.matches(values)
        {
            values.insert("batteryPresetId".into(), Value::Null);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_variant_is_bounded_unique_and_has_independent_nominal_arithmetic() {
        let mut ids = std::collections::HashSet::new();
        let cell_ah = [
            ("LR2170LA / DG40", 4.),
            ("Molicel P42A", 4.2),
            ("Samsung 50S", 5.),
            ("Reliance RS50", 5.),
            ("Molicel P28A", 2.8),
            ("Molicel P30B", 3.),
            ("Molicel P45B", 4.5),
            ("VTC6 / 30Q", 3.),
            ("Murata VTC5D", 2.8),
            ("Molicel P50B", 5.),
            ("Molicel M35A", 3.5),
            ("Sanyo 20700B", 4.),
            ("Sanyo GA", 3.5),
        ];
        assert_eq!(PRESETS.len(), 52);
        assert!(CATALOG_VERSION.len() <= 64);
        for p in PRESETS.iter() {
            assert!(ids.insert(&p.id));
            assert!(!p.id.is_empty() && p.id.len() <= 64);
            for text in [&p.label, &p.manufacturer, &p.cell_label] {
                assert!(!text.is_empty() && text.len() <= 128);
            }
            assert!((1..=36).contains(&p.series_count));
            assert!((1..=20).contains(&p.parallel_count));
            assert!(crate::battery_catalog::profile(&p.battery_cell_profile_id).is_some());
            let ah = cell_ah
                .iter()
                .find(|(cell, _)| *cell == p.cell_label)
                .unwrap()
                .1;
            assert!(
                (p.nominal_capacity_ah.unwrap() - ah * f64::from(p.parallel_count)).abs() < 1e-12
            );
        }
        let xr = PRESETS
            .iter()
            .find(|p| p.label.starts_with("XR ("))
            .unwrap();
        let pint = PRESETS
            .iter()
            .find(|p| p.label.starts_with("Pint X ("))
            .unwrap();
        assert_ne!(xr.id, pint.id);
        assert_eq!(xr.nominal_capacity_ah, pint.nominal_capacity_ah);
    }
}
