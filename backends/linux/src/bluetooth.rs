//! BlueZ execution only; shared Rust owns discovery eligibility and all write policy.
use crate::runtime::Ingress;
use bluer::{
    Adapter, Address, Device, Session, Uuid,
    gatt::{
        WriteOp,
        remote::{Characteristic, CharacteristicWriteRequest},
    },
};
use float_view_backend_service::{
    ble::{Profile, Progress, WriteMode},
    os::{Effect, Fact, LinkState, Permission, Radio},
    permissions::{Grant, Observation, Platform},
};
use futures_util::StreamExt;
use std::{
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, watch},
    task::JoinHandle,
};
const SERVICE: Uuid = Uuid::from_u128(0x6e400001_b5a3_f393_e0a9_e50e24dcca9e);
const WRITE: Uuid = Uuid::from_u128(0x6e400002_b5a3_f393_e0a9_e50e24dcca9e);
const NOTIFY: Uuid = Uuid::from_u128(0x6e400003_b5a3_f393_e0a9_e50e24dcca9e);
const DEADLINE: Duration = Duration::from_secs(8);
const MAX_DEVICES: usize = 256;
struct Connection {
    generation: u64,
    device: Device,
    write: Option<Characteristic>,
    notify: Option<Characteristic>,
    reader: Option<JoinHandle<()>>,
}
struct Bluetooth {
    session: Option<Session>,
    adapter: Option<Adapter>,
    connection: Option<Connection>,
    scan: Option<JoinHandle<()>>,
    input: Ingress,
    denied: Arc<AtomicBool>,
}
fn grant(access: Permission) -> Grant {
    Grant {
        access,
        requestable: false,
        rationale: false,
    }
}
fn observation(radio: Radio, access: Permission) -> Observation {
    let mut o = Observation::unknown(Platform::Linux);
    o.bluetooth_scan = grant(access);
    o.bluetooth_connect = grant(access);
    o.bluetooth_supported = (radio != Radio::Unknown).then_some(radio != Radio::Unsupported);
    o.bluetooth_service = Some(radio == Radio::PoweredOn);
    o.location_services = Some(false);
    o
}
impl Bluetooth {
    fn status(&self, radio: Radio, access: Permission) {
        let _ = self.input.cleanup(Fact::Permissions {
            observation: observation(radio, access),
        });
        let _ = self.input.cleanup(Fact::Bluetooth {
            radio,
            scan: access,
            connect: access,
        });
    }
    async fn refresh(&mut self) {
        let result = tokio::time::timeout(Duration::from_secs(2), async {
            if self.session.is_none() {
                self.session = Some(Session::new().await?);
            }
            let session = self.session.as_ref().unwrap();
            let mut names = session.adapter_names().await?;
            names.sort();
            names.truncate(16);
            if let Some(selected) = &self.adapter
                && !names.iter().any(|n| n == selected.name())
            {
                self.disconnect().await;
                self.adapter = None;
            }
            if self.adapter.is_none() {
                let mut first = None;
                for name in names {
                    let a = self.session.as_ref().unwrap().adapter(&name)?;
                    if first.is_none() {
                        first = Some(a.clone());
                    }
                    if a.is_powered().await? {
                        self.adapter = Some(a);
                        break;
                    }
                }
                if self.adapter.is_none() {
                    self.adapter = first;
                }
            }
            match &self.adapter {
                Some(adapter) => Ok::<_, bluer::Error>(if adapter.is_powered().await? {
                    Radio::PoweredOn
                } else {
                    Radio::PoweredOff
                }),
                None => Ok(Radio::Unsupported),
            }
        })
        .await;
        match result {
            Ok(Ok(radio)) => {
                if radio != Radio::PoweredOn {
                    self.stop_scan();
                    self.disconnect().await;
                }
                if radio != Radio::PoweredOn {
                    self.denied.store(false, Ordering::Relaxed);
                }
                if self.denied.load(Ordering::Relaxed) {
                    self.status(Radio::Unauthorized, Permission::Denied);
                } else {
                    self.status(radio, Permission::NotRequired);
                }
                if let Some(c) = &self.connection
                    && !matches!(
                        tokio::time::timeout(Duration::from_secs(1), c.device.is_connected()).await,
                        Ok(Ok(true))
                    )
                {
                    self.disconnect().await;
                }
            }
            _ => {
                self.stop_scan();
                self.disconnect().await;
                self.adapter = None;
                self.session = None;
                self.denied.store(false, Ordering::Relaxed);
                self.status(Radio::Unknown, Permission::Unknown);
            }
        }
    }
    fn stop_scan(&mut self) {
        if let Some(task) = self.scan.take() {
            task.abort();
        }
    }
    async fn disconnect(&mut self) {
        if let Some(mut c) = self.connection.take() {
            if let Some(task) = c.reader.take() {
                task.abort();
            }
            let _ = self.input.cleanup(Fact::Link {
                generation: c.generation,
                state: LinkState::Disconnected,
            });
            let _ = tokio::time::timeout(Duration::from_secs(1), c.device.disconnect()).await;
        }
    }
    async fn effect(&mut self, effect: Effect) {
        match effect {
            Effect::Scan { scan_id } => {
                self.stop_scan();
                if let Some(adapter) = self.adapter.clone() {
                    let input = self.input.clone();
                    let denied = self.denied.clone();
                    self.scan = Some(tokio::spawn(async move {
                        let result = tokio::time::timeout(
                            Duration::from_secs(3),
                            adapter.discover_devices(),
                        )
                        .await;
                        let events = match result {
                            Ok(Ok(events)) => events,
                            error => {
                                let (radio, access) = scan_failure(
                                    error
                                        .as_ref()
                                        .ok()
                                        .and_then(|v| v.as_ref().err())
                                        .map(|e| &e.kind),
                                );
                                denied.store(access == Permission::Denied, Ordering::Relaxed);
                                let _ = input.cleanup(Fact::Bluetooth {
                                    radio,
                                    scan: access,
                                    connect: access,
                                });
                                let _ = input.cleanup(Fact::Permissions {
                                    observation: observation(radio, access),
                                });
                                return;
                            }
                        };
                        tokio::pin!(events);
                        let mut interval = tokio::time::interval(Duration::from_secs(2));
                        loop {
                            tokio::select! {
                                event = events.next() => match event {
                                    Some(bluer::AdapterEvent::DeviceAdded(address)) => advertise(&adapter, address, scan_id, &input).await,
                                    None => break,
                                    _ => {}
                                },
                                _ = interval.tick() => {
                                    if let Ok(Ok(addresses)) = tokio::time::timeout(Duration::from_secs(1), adapter.device_addresses()).await {
                                        for address in addresses.into_iter().take(MAX_DEVICES) { advertise(&adapter, address, scan_id, &input).await; }
                                    }
                                }
                            }
                        }
                    }));
                }
            }
            Effect::StopScan => self.stop_scan(),
            Effect::Connect {
                generation,
                device_id,
            } => {
                self.disconnect().await;
                let Some((adapter_name, address)) = parse_device(&device_id) else {
                    self.fail(generation);
                    return;
                };
                let Some(adapter) = &self.adapter else {
                    self.fail(generation);
                    return;
                };
                if adapter.name() != adapter_name {
                    self.fail(generation);
                    return;
                }
                let Ok(device) = adapter.device(address) else {
                    self.fail(generation);
                    return;
                };
                self.connection = Some(Connection {
                    generation,
                    device: device.clone(),
                    write: None,
                    notify: None,
                    reader: None,
                });
                let input = self.input.clone();
                let result = tokio::time::timeout(DEADLINE, async {
                    connect_association(adapter, &device, address).await?;
                    let _ = input.send(Fact::Link {
                        generation,
                        state: LinkState::DiscoveringServices,
                    });
                    let mut write = None;
                    let mut notify = None;
                    for service in device.services().await?.into_iter().take(64) {
                        if service.uuid().await? == SERVICE {
                            for c in service.characteristics().await?.into_iter().take(64) {
                                let uuid = c.uuid().await?;
                                if uuid == WRITE {
                                    write = Some(c.clone());
                                }
                                if uuid == NOTIFY {
                                    notify = Some(c);
                                }
                            }
                        }
                    }
                    let mut profile = Profile {
                        service: write.is_some() && notify.is_some(),
                        notify: false,
                        with_response: 0,
                        without_response: 0,
                    };
                    if let Some(w) = &write {
                        let f = w.flags().await?;
                        profile.with_response = if f.write { 20 } else { 0 };
                        profile.without_response = if f.write_without_response { 20 } else { 0 };
                    }
                    if let Some(n) = &notify {
                        profile.notify = n.flags().await?.notify;
                    }
                    Ok::<_, bluer::Error>((write, notify, profile))
                })
                .await;
                match result {
                    Ok(Ok((write, notify, profile))) => {
                        if let Some(c) = &mut self.connection {
                            c.write = write;
                            c.notify = notify;
                        }
                        if !self.input.send(Fact::Profile {
                            generation,
                            operation: generation,
                            profile,
                        }) {
                            self.disconnect().await;
                        }
                    }
                    _ => {
                        self.fail(generation);
                        self.disconnect().await;
                    }
                }
            }
            Effect::Disconnect { generation } => {
                if self
                    .connection
                    .as_ref()
                    .is_some_and(|c| c.generation == generation)
                {
                    self.disconnect().await;
                }
            }
            Effect::EnableNotifications {
                generation,
                operation,
                ..
            } => {
                let Some(c) = self
                    .connection
                    .as_mut()
                    .filter(|c| c.generation == generation)
                else {
                    return;
                };
                let Some(notify) = &c.notify else {
                    return;
                };
                let result = tokio::time::timeout(DEADLINE, notify.notify_io()).await;
                match result {
                    Ok(Ok(mut reader)) if reader.mtu() <= 512 => {
                        let input = self.input.clone();
                        if !input.send(Fact::Notifications {
                            generation,
                            operation,
                            success: true,
                        }) {
                            self.disconnect().await;
                            return;
                        }
                        c.reader = Some(tokio::spawn(async move {
                            let mut bytes = [0; 512];
                            loop {
                                match reader.read(&mut bytes).await {
                                    Ok(0) | Err(_) => break,
                                    Ok(n) => {
                                        if !input.send(Fact::Bytes {
                                            generation,
                                            bytes: bytes[..n].to_vec(),
                                        }) {
                                            break;
                                        }
                                    }
                                }
                            }
                            let _ = input.cleanup(Fact::Link {
                                generation,
                                state: LinkState::Failed,
                            });
                        }));
                    }
                    _ => {
                        let _ = self.input.cleanup(Fact::Notifications {
                            generation,
                            operation,
                            success: false,
                        });
                    }
                }
            }
            Effect::Write {
                generation,
                operation,
                mode,
                bytes,
            } => {
                let Some(c) = self
                    .connection
                    .as_ref()
                    .filter(|c| c.generation == generation)
                else {
                    return;
                };
                let Some(write) = &c.write else {
                    return;
                };
                let req = CharacteristicWriteRequest {
                    op_type: match mode {
                        WriteMode::WithResponse => WriteOp::Request,
                        WriteMode::WithoutResponse => WriteOp::Command,
                    },
                    ..Default::default()
                };
                let result = tokio::time::timeout(DEADLINE, write.write_ext(&bytes, &req)).await;
                if matches!(result, Ok(Ok(()))) {
                    // D-Bus completion means remote response for Request, local acceptance for Command.
                    // Command completion permits the next bounded submission, never claims an ACK.
                    let sent = self.input.send(Fact::WriteProgress {
                        generation,
                        operation,
                        progress: Progress::Submitted,
                    });
                    let progressed = self.input.send(Fact::WriteProgress {
                        generation,
                        operation,
                        progress: match mode {
                            WriteMode::WithResponse => Progress::Acknowledged,
                            WriteMode::WithoutResponse => Progress::Available,
                        },
                    });
                    if !sent || !progressed {
                        self.disconnect().await;
                    }
                } else {
                    let _ = self.input.cleanup(Fact::WriteProgress {
                        generation,
                        operation,
                        progress: Progress::Failed,
                    });
                    self.disconnect().await;
                }
            }
            _ => {}
        }
    }
    fn fail(&self, generation: u64) {
        let _ = self.input.cleanup(Fact::Link {
            generation,
            state: LinkState::Failed,
        });
    }
}
// BlueZ may discard an unpaired Device object when the adapter loses power.
// Re-resolve only the saved address, within the caller's setup deadline.
async fn connect_association(
    adapter: &Adapter,
    device: &Device,
    address: Address,
) -> bluer::Result<()> {
    if adapter.device_addresses().await?.contains(&address) {
        return device.connect().await;
    }
    let events = adapter.discover_devices().await?;
    tokio::pin!(events);
    while let Some(event) = events.next().await {
        if matches!(event, bluer::AdapterEvent::DeviceAdded(found) if found == address) {
            break;
        }
    }
    // Keep this scoped discovery alive until BlueZ has accepted the connection.
    device.connect().await
}

fn scan_failure(kind: Option<&bluer::ErrorKind>) -> (Radio, Permission) {
    match kind {
        Some(bluer::ErrorKind::NotAuthorized | bluer::ErrorKind::NotPermitted) => {
            (Radio::Unauthorized, Permission::Denied)
        }
        _ => (Radio::Unknown, Permission::Unknown),
    }
}
fn parse_device(id: &str) -> Option<(&str, Address)> {
    let (adapter, address) = id.split_once('/')?;
    if !adapter.starts_with("hci")
        || !adapter[3..].bytes().all(|b| b.is_ascii_digit())
        || adapter.len() <= 3
        || adapter.len() > 16
    {
        return None;
    }
    Some((adapter, Address::from_str(address).ok()?))
}
async fn advertise(adapter: &Adapter, address: Address, scan_id: u64, input: &Ingress) {
    let _ = tokio::time::timeout(Duration::from_millis(500), async {
        let d = adapter.device(address)?;
        let Some(rssi) = d.rssi().await? else {
            return Ok::<_, bluer::Error>(());
        };
        let uart = d.uuids().await?.unwrap_or_default().contains(&SERVICE);
        let name = d.name().await?.map(|s| s.chars().take(120).collect());
        let _ = input.send(Fact::Discovered {
            scan_id,
            device_id: format!("{}/{address}", adapter.name()),
            name,
            rssi: Some(i32::from(rssi)),
            advertises_uart: uart,
        });
        Ok(())
    })
    .await;
}
pub async fn run(
    mut effects: mpsc::Receiver<Effect>,
    input: Ingress,
    mut stop: watch::Receiver<bool>,
) {
    let mut ble = Bluetooth {
        session: None,
        adapter: None,
        connection: None,
        scan: None,
        input,
        denied: Arc::new(AtomicBool::new(false)),
    };
    let mut refresh = tokio::time::interval(Duration::from_secs(2));
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased;
            _ = stop.changed() => break,
            Some(effect) = effects.recv() => ble.effect(effect).await,
            _ = refresh.tick() => ble.refresh().await,
        }
    }
    ble.stop_scan();
    ble.disconnect().await;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_associations_do_not_resolve_by_display_name() {
        assert!(parse_device("hci0/AA:BB:CC:DD:EE:FF").is_some());
        for value in [
            "board",
            "../AA:BB:CC:DD:EE:FF",
            "hci0/not-an-address",
            "hciX/AA:BB:CC:DD:EE:FF",
            "hci/AA:BB:CC:DD:EE:FF",
        ] {
            assert!(parse_device(value).is_none());
        }
    }
    #[test]
    fn scan_timeout_and_operational_failure_do_not_fabricate_denial() {
        for kind in [
            None,
            Some(&bluer::ErrorKind::Failed),
            Some(&bluer::ErrorKind::NotReady),
        ] {
            assert_eq!(scan_failure(kind), (Radio::Unknown, Permission::Unknown));
        }
        assert_eq!(
            scan_failure(Some(&bluer::ErrorKind::NotAuthorized)),
            (Radio::Unauthorized, Permission::Denied)
        );
    }
    #[test]
    fn missing_radio_and_denial_are_distinct() {
        let absent = observation(Radio::Unsupported, Permission::NotRequired);
        assert_eq!(absent.bluetooth_supported, Some(false));
        let denied = observation(Radio::Unauthorized, Permission::Denied);
        assert_eq!(denied.bluetooth_scan.access, Permission::Denied);
        assert!(denied.rows(None).iter().all(|row| row.action.is_none()));
    }
}
