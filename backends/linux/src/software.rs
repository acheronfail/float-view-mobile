//! Explicit test-only OS adapter. All simulation and assertions stay in conformance/.
use crate::runtime::Ingress;
use base64::{Engine, engine::general_purpose::STANDARD};
use float_view_backend_service::{
    os::{Effect, Fact, Permission, Radio},
    permissions::{Grant, Observation, Platform},
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
struct Relay {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    token: String,
    run: String,
    session: String,
    operation: u64,
    generation: u64,
    native_generation: u64,
    event_sequence: u64,
    scan_id: u64,
    device: String,
}
impl Relay {
    fn new() -> Option<Self> {
        let env = |key| std::env::var(key).ok();
        let endpoint = reqwest::Url::parse(&env("FLOAT_VIEW_FIXTURE_ENDPOINT")?).ok()?;
        if endpoint.scheme() != "http"
            || endpoint.host_str() != Some("127.0.0.1")
            || endpoint.port().is_none()
            || endpoint.path() != "/"
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
        {
            return None;
        }
        let identity =
            |key| env(key).filter(|v| v.len() == 48 && v.bytes().all(|b| b.is_ascii_hexdigit()));
        Some(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(4))
                .build()
                .ok()?,
            endpoint,
            token: identity("FLOAT_VIEW_FIXTURE_TOKEN")?,
            run: identity("FLOAT_VIEW_FIXTURE_RUN")?,
            session: identity("FLOAT_VIEW_FIXTURE_SESSION")?,
            operation: 0,
            generation: 0,
            native_generation: 0,
            event_sequence: 0,
            scan_id: 0,
            device: String::new(),
        })
    }
    async fn command(&mut self, command: &str, extra: Value, input: &Ingress) -> Result<(), ()> {
        self.operation += 1;
        let mut body = json!({"run": self.run, "session":self.session, "operation":self.operation, "generation":self.generation, "command":command});
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().ok_or(())?.clone());
        let mut response = self
            .client
            .post(self.endpoint.clone())
            .bearer_auth(&self.token)
            .header("X-Fixture-Session", &self.session)
            .json(&body)
            .send()
            .await
            .map_err(|_| ())?
            .error_for_status()
            .map_err(|_| ())?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
            if bytes.len() + chunk.len() > 65_536 {
                return Err(());
            }
            bytes.extend_from_slice(&chunk);
        }
        let reply: Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
        if reply["run"] != self.run
            || reply["session"] != self.session
            || reply["operation"] != self.operation
            || reply["generation"] != self.generation
        {
            return Err(());
        }
        let events = reply["events"].as_array().ok_or(())?;
        if events.len() > 128 {
            return Err(());
        }
        for e in events {
            let sequence = e["sequence"].as_u64().ok_or(())?;
            if sequence <= self.event_sequence || e["generation"] != self.generation {
                return Err(());
            }
            self.event_sequence = sequence;
            let g = self.native_generation;
            let fact = match e["event"].as_str().ok_or(())? {
                "discovered" => Fact::Discovered {
                    scan_id: self.scan_id,
                    device_id: e["device"].as_str().ok_or(())?.into(),
                    name: e["name"].as_str().map(str::to_owned),
                    rssi: e["rssi"].as_i64().map(|v| v as i32),
                    advertises_uart: e["uart"].as_bool().ok_or(())?,
                },
                "profile" => Fact::Profile {
                    generation: g,
                    operation: g,
                    profile: serde_json::from_value(e["profile"].clone()).map_err(|_| ())?,
                },
                "notifications" => Fact::Notifications {
                    generation: g,
                    operation: e["transport_operation"].as_u64().ok_or(())?,
                    success: e["success"].as_bool().ok_or(())?,
                },
                "writeProgress" => Fact::WriteProgress {
                    generation: g,
                    operation: e["transport_operation"].as_u64().ok_or(())?,
                    progress: serde_json::from_value(e["progress"].clone()).map_err(|_| ())?,
                },
                "link" => Fact::Link {
                    generation: g,
                    state: serde_json::from_value(e["state"].clone()).map_err(|_| ())?,
                },
                "bytes" => {
                    let encoded = e["bytes"].as_str().ok_or(())?;
                    if encoded.len() > 10_924 {
                        return Err(());
                    }
                    let bytes = STANDARD.decode(encoded).map_err(|_| ())?;
                    if bytes.len() > 8192 {
                        return Err(());
                    }
                    Fact::Bytes {
                        generation: g,
                        bytes,
                    }
                }
                _ => return Err(()),
            };
            if !input.send(fact) {
                return Err(());
            }
        }
        Ok(())
    }
    async fn effect(&mut self, effect: Effect, input: &Ingress) -> Result<(), ()> {
        let (command, mut fields) = match effect {
            Effect::Scan { scan_id } => {
                self.scan_id = scan_id;
                ("scan", json!({}))
            }
            Effect::StopScan => ("stopScan", json!({})),
            Effect::Connect {
                generation,
                device_id,
            } => {
                self.generation += 1;
                self.native_generation = generation;
                self.device = device_id;
                ("connect", json!({}))
            }
            Effect::Disconnect { .. } => ("disconnect", json!({})),
            Effect::EnableNotifications { operation, .. } => (
                "enableNotifications",
                json!({"transport_operation":operation}),
            ),
            Effect::Write {
                operation,
                mode,
                bytes,
                ..
            } => (
                "write",
                json!({"transport_operation":operation, "response":mode == float_view_backend_service::ble::WriteMode::WithResponse, "bytes":STANDARD.encode(bytes)}),
            ),
            _ => return Ok(()),
        };
        if !self.device.is_empty() {
            fields["device"] = json!(self.device);
        }
        self.command(command, fields, input).await
    }
}
pub async fn run(
    mut effects: mpsc::Receiver<Effect>,
    input: Ingress,
    mut stop: watch::Receiver<bool>,
) {
    let Some(mut relay) = Relay::new() else {
        eprintln!("Invalid explicit software fixture configuration");
        return;
    };
    let mut observation = Observation::unknown(Platform::Linux);
    observation.bluetooth_scan = Grant {
        access: Permission::NotRequired,
        requestable: false,
        rationale: false,
    };
    observation.bluetooth_connect = observation.bluetooth_scan.clone();
    observation.bluetooth_service = Some(true);
    observation.bluetooth_supported = Some(true);
    observation.location_services = Some(false);
    let _ = input.send(Fact::Permissions { observation });
    let _ = input.send(Fact::Bluetooth {
        radio: Radio::PoweredOn,
        scan: Permission::NotRequired,
        connect: Permission::NotRequired,
    });
    let mut poll = tokio::time::interval(Duration::from_millis(20));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let result = tokio::select! { biased;
            _ = stop.changed() => break,
            Some(effect) = effects.recv() => relay.effect(effect, &input).await,
            _ = poll.tick() => relay.command("poll", json!({}), &input).await,
        };
        if result.is_err() {
            let _ = input.cleanup(Fact::Bluetooth {
                radio: Radio::Unsupported,
                scan: Permission::NotRequired,
                connect: Permission::NotRequired,
            });
            break;
        }
    }
}
