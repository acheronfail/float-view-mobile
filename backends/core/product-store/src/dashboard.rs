//! Closed additive dashboard metadata; no layout or telemetry policy lives here.
use crate::{Error, Result};
use serde_json::Value;
use std::collections::HashSet;

fn keys(v: &Value, allowed: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.keys().all(|k| allowed.contains(&k.as_str())))
}
fn one(v: &Value, allowed: &[&str]) -> bool {
    v.as_str().is_some_and(|s| allowed.contains(&s))
}
fn domain(field: &str) -> Option<(f64, f64)> {
    if field.ends_with("TempC") {
        Some((-100., 300.))
    } else if field.ends_with("CurrentA") {
        Some((-2000., 2000.))
    } else if field.ends_with("VoltageV") || field.ends_with("Adc1V") || field.ends_with("Adc2V") {
        Some((0., 1000.))
    } else if [
        "dutyCycle",
        "driveLoad",
        "estimatedBatteryLevel",
        "batteryLevel",
        "bmsStateOfCharge",
        "bmsStateOfHealth",
        "bmsHumidityPercent",
    ]
    .contains(&field)
    {
        Some((0., 100.))
    } else if field == "speedKph" {
        Some((0., 500.))
    } else if field.ends_with("Deg") {
        Some((-360., 360.))
    } else if field == "powerW" {
        Some((-100000., 100000.))
    } else {
        None
    }
}
fn valid_size(size: &Value) -> bool {
    one(size, &["compact", "medium", "large"])
        || (size.is_object()
            && keys(size, &["columns", "rows"])
            && ["columns", "rows"].iter().all(|key| {
                size[key]
                    .as_f64()
                    .is_some_and(|value| (1.0..=4.0).contains(&value) && value.fract() == 0.0)
            }))
}
pub(crate) fn validate(value: &Value) -> Result<()> {
    if value.is_null() {
        return Ok(());
    }
    let valid = || -> Option<()> {
        if !keys(value, &["version", "widgets"])
            || value["version"].as_u64()? != 1
            || serde_json::to_vec(value).ok()?.len() > 8192
        {
            return None;
        }
        let widgets = value["widgets"].as_array()?;
        if widgets.len() > 24 {
            return None;
        }
        let mut ids = HashSet::new();
        for w in widgets {
            let id = w["id"].as_str()?;
            if id.is_empty()
                || id.len() > 64
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b))
                || !ids.insert(id)
                || !valid_size(&w["size"])
            {
                return None;
            }
            match w["kind"].as_str()? {
                "visual"
                    if keys(w, &["id", "size", "kind", "visual"])
                        && one(
                            &w["visual"],
                            &["speedGauge", "battery", "temperatures", "footpads"],
                        ) => {}
                "control"
                    if keys(w, &["id", "size", "kind", "control"])
                        && one(&w["control"], &["headlights", "leds"]) => {}
                "reading" => {
                    let field = w["field"].as_str()?;
                    if !keys(w, &["id", "size", "kind", "field", "presentation", "range"])
                        || !crate::settings::LIVE_WIDGET_FIELDS.contains(&field)
                        || ["empty", "headlights", "leds"].contains(&field)
                        || !one(&w["presentation"], &["number", "bar", "gauge", "trend"])
                    {
                        return None;
                    }
                    let bounds = domain(field);
                    if one(&w["presentation"], &["bar", "gauge"]) && bounds.is_none() {
                        return None;
                    }
                    if let Some(r) = w.get("range") {
                        let (low, high) = bounds?;
                        let min = r["min"].as_f64()?;
                        let max = r["max"].as_f64()?;
                        if !keys(r, &["min", "max"])
                            || !min.is_finite()
                            || !max.is_finite()
                            || min >= max
                            || min < low
                            || max > high
                        {
                            return None;
                        }
                    }
                }
                _ => return None,
            }
        }
        Some(())
    };
    valid().ok_or(Error::InvalidInput)
}
