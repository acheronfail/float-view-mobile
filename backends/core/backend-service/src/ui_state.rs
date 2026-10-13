//! A single process-lifetime presentation checkpoint. No product effects.
use crate::{BackendService, operations::Failure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Checkpoint {
    version: u8,
    route: Route,
    live: Option<Live>,
    surface: Option<Surface>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Route {
    Home,
    Sessions,
    Settings,
    Live,
    Bms,
    BoardHistory {
        #[serde(rename = "savedDeviceId")]
        saved_device_id: String,
    },
    Session {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "savedDeviceId")]
        saved_device_id: Option<String>,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Live {
    device_id: String,
    mode: Mode,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Mode {
    Normal,
    Detailed,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Surface {
    LiveActions,
    LiveCharts,
    BmsDetails,
    RefloatConfiguration,
    RefloatTunes,
    BoardActions,
    BoardConfiguration {
        #[serde(rename = "savedDeviceId")]
        saved_device_id: String,
        #[serde(rename = "batteryOnly")]
        battery_only: bool,
    },
}
fn id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 512
}
impl Checkpoint {
    fn validate(&self) -> bool {
        if self.version != 1 {
            return false;
        }
        let valid_route = match &self.route {
            Route::BoardHistory { saved_device_id } => id(saved_device_id),
            Route::Session {
                session_id,
                saved_device_id,
            } => id(session_id) && saved_device_id.as_ref().is_none_or(|s| id(s)),
            _ => true,
        };
        let live_valid = match (&self.route, &self.live) {
            (Route::Live, Some(live)) => id(&live.device_id),
            (Route::Live, None) => true,
            (_, None) => true,
            _ => false,
        };
        let surface_valid = match (&self.route, &self.surface) {
            (_, None) => true,
            (
                Route::Live,
                Some(Surface::BoardConfiguration {
                    saved_device_id, ..
                }),
            ) => self.live.is_some() && id(saved_device_id),
            (Route::Live, Some(surface)) => {
                self.live.is_some() && !matches!(surface, Surface::BoardActions)
            }
            (Route::BoardHistory { .. }, Some(Surface::BoardActions)) => true,
            (
                Route::BoardHistory {
                    saved_device_id: route_id,
                },
                Some(Surface::BoardConfiguration {
                    saved_device_id,
                    battery_only,
                }),
            ) => id(saved_device_id) && saved_device_id == route_id && !battery_only,
            _ => false,
        };
        valid_route && live_valid && surface_valid
    }
}
impl BackendService {
    pub(crate) fn clear_connection_checkpoint(&mut self, device_id: &str) {
        if self
            .ui_checkpoint
            .as_ref()
            .and_then(|c| c.live.as_ref())
            .is_some_and(|live| live.device_id == device_id)
        {
            self.ui_checkpoint = None;
        }
    }

    pub fn ui_checkpoint(&self) -> &Option<Checkpoint> {
        &self.ui_checkpoint
    }
    pub fn set_ui_checkpoint(&mut self, value: serde_json::Value) -> Result<(), Failure> {
        if value.is_null() {
            self.ui_checkpoint = None;
            return Ok(());
        }
        let bytes = serde_json::to_vec(&value).map_err(|_| Failure::InvalidRequest)?;
        if bytes.len() > 4096
            || !["version", "route", "live", "surface"]
                .iter()
                .all(|k| value.get(k).is_some())
        {
            return Err(Failure::InvalidRequest);
        }
        if value["route"]["kind"] == "session" && value["route"].get("savedDeviceId").is_none() {
            return Err(Failure::InvalidRequest);
        }
        let checkpoint: Checkpoint =
            serde_json::from_value(value).map_err(|_| Failure::InvalidRequest)?;
        if !checkpoint.validate() {
            return Err(Failure::InvalidRequest);
        }
        self.ui_checkpoint = Some(checkpoint);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn checkpoint_is_atomic_ephemeral_and_has_no_product_events() {
        let root = std::env::temp_dir().join(format!("float-view-ui-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let good = json!({"version":1,"route":{"kind":"settings"},"live":null,"surface":null});
        {
            let mut service = BackendService::open(&root).unwrap();
            let subscriber = service.subscribe();
            let revision = service.service_revision();
            let catalog = service.catalog_revision();
            assert!(service.ui_checkpoint().is_none());
            service.set_ui_checkpoint(good.clone()).unwrap();
            let mut bad = good.clone();
            bad["route"] = json!({"kind":"session","sessionId":"\u{1}".repeat(512),"savedDeviceId":"\u{1}".repeat(512)});
            assert!(matches!(
                service.set_ui_checkpoint(bad),
                Err(Failure::InvalidRequest)
            ));
            assert_eq!(serde_json::to_value(service.ui_checkpoint()).unwrap(), good);
            assert_eq!(service.service_revision(), revision);
            assert_eq!(service.catalog_revision(), catalog);
            assert!(!subscriber.has_changed().unwrap());
            service.set_ui_checkpoint(serde_json::Value::Null).unwrap();
            assert!(service.ui_checkpoint().is_none());
            service.set_ui_checkpoint(good).unwrap();
        }
        assert!(
            BackendService::open(&root)
                .unwrap()
                .ui_checkpoint()
                .is_none()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn checkpoint_validation_is_closed_and_bounded() {
        let good = json!({"version":1,"route":{"kind":"live"},"live":{"deviceId":"board","mode":"detailed"},"surface":{"kind":"liveCharts"}});
        let parse = |v| serde_json::from_value::<Checkpoint>(v).is_ok_and(|c| c.validate());
        assert!(parse(good.clone()));
        let mut bad = good.clone();
        bad["version"] = json!(2);
        assert!(!parse(bad));
        let mut bad = good.clone();
        bad["draft"] = json!({});
        assert!(!parse(bad));
        let mut bad = good.clone();
        bad["route"] = json!({"kind":"settings"});
        assert!(!parse(bad));
        let mut bad = good.clone();
        bad["live"]["deviceId"] = json!("é".repeat(257));
        assert!(!parse(bad));
        let mut bad = good;
        bad["surface"] = json!({"kind":"shutdown"});
        assert!(!parse(bad));
    }
}
