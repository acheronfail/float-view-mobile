//! One process owner and bounded OS execution, independent of the GTK view.
use float_view_backend_service::{
    Process,
    os::{Effect, Envelope, Fact},
};
use float_view_loopback_server::{backend::Backend, protocol::Platform, server::Server};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
use tokio::sync::{mpsc as channel, watch};

pub const FACT_CAPACITY: usize = 256;
pub const CLEANUP_CAPACITY: usize = 16;
pub const MAX_TIMERS: usize = 256;
#[derive(Clone)]
pub struct Ingress {
    pub ordinary: channel::Sender<Fact>,
    pub cleanup: channel::Sender<Fact>,
}
impl Ingress {
    pub fn send(&self, fact: Fact) -> bool {
        self.ordinary.try_send(fact).is_ok()
    }
    pub fn cleanup(&self, fact: Fact) -> bool {
        self.cleanup.try_send(fact).is_ok()
    }
}
pub struct Host {
    pub process: Arc<Process>,
    pub server: Server,
    pub ingress: Ingress,
    pub ui: mpsc::Receiver<Effect>,
    stop: watch::Sender<bool>,
    finished: mpsc::Receiver<()>,
}
pub fn data_root(xdg: Option<&str>, home: Option<&str>) -> Result<PathBuf, &'static str> {
    if let Some(path) = xdg
        .filter(|s| !s.is_empty())
        .map(Path::new)
        .filter(|p| p.is_absolute())
    {
        return Ok(path.join("float-view"));
    }
    home.map(Path::new)
        .filter(|p| p.is_absolute())
        .map(|p| p.join(".local/share/float-view"))
        .ok_or("Set an absolute HOME or XDG_DATA_HOME")
}
pub fn monotonic_ms() -> u64 {
    let time = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    (time.tv_sec as u64)
        .saturating_mul(1000)
        .saturating_add(time.tv_nsec as u64 / 1_000_000)
}
fn epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
impl Host {
    pub fn open(root: &Path, html: &[u8], software: bool) -> Result<Self, &'static str> {
        Self::open_on(root, html, software, Platform::Linux.port())
    }
    pub(crate) fn open_on(
        root: &Path,
        html: &[u8],
        software: bool,
        port: u16,
    ) -> Result<Self, &'static str> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(root)
            .map_err(|_| "Cannot create private data directory")?;
        let metadata =
            std::fs::symlink_metadata(root).map_err(|_| "Cannot inspect data directory")?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            return Err("Data directory must be a private directory (mode 0700)");
        }
        let process = Arc::new(Process::open(root).map_err(
            |_| "Cannot open data directory: check ownership, existing instance and storage health",
        )?);
        let server = Server::with_backend(
            Platform::Linux,
            html,
            Arc::new(Backend::with_service(process.service.clone())),
        )
        .map_err(|_| "Cannot initialize embedded frontend")?;
        server
            .start(port)
            .map_err(|_| "Cannot bind 127.0.0.1:11013; close the other instance")?;
        let (tx, facts) = channel::channel(FACT_CAPACITY);
        let (priority, cleanup) = channel::channel(CLEANUP_CAPACITY);
        let ingress = Ingress {
            ordinary: tx,
            cleanup: priority,
        };
        let (ui_tx, ui) = mpsc::sync_channel(8);
        let (stop, stopped) = watch::channel(false);
        let (done, finished) = mpsc::sync_channel(1);
        let owner = process.clone();
        let storage_root = root.to_owned();
        let input = ingress.clone();
        std::thread::Builder::new()
            .name("float-view-linux-os".into())
            .spawn(move || {
                if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    runtime.block_on(run(
                        owner,
                        input,
                        facts,
                        cleanup,
                        ui_tx,
                        stopped,
                        (software, storage_root),
                    ));
                    runtime.shutdown_timeout(Duration::from_secs(1));
                }
                let _ = done.send(());
            })
            .map_err(|_| "Cannot start OS executor")?;
        Ok(Self {
            process,
            server,
            ingress,
            ui,
            stop,
            finished,
        })
    }
    pub fn shutdown(&self) -> Result<(), &'static str> {
        let _ = self.stop.send(true);
        match self.finished.recv_timeout(Duration::from_secs(10)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => return Err("OS shutdown timed out"),
        }
        self.server
            .stop()
            .map_err(|_| "Listener shutdown timed out")?;
        self.process
            .shutdown()
            .map_err(|_| "Storage shutdown incomplete")
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}
struct Publisher {
    service: Arc<Mutex<float_view_backend_service::BackendService>>,
    lifetime: u64,
    sequence: u64,
}
impl Publisher {
    fn send(&mut self, fact: Fact) {
        self.sequence += 1;
        if let Ok(mut service) = self.service.lock()
            && service
                .apply_os(Envelope {
                    lifetime: self.lifetime,
                    sequence: self.sequence,
                    monotonic_ms: monotonic_ms(),
                    epoch_ms: epoch_ms(),
                    fact,
                })
                .is_err()
        {
            eprintln!("Linux OS fact rejected");
        }
    }
}
async fn run(
    process: Arc<Process>,
    input: Ingress,
    mut facts: channel::Receiver<Fact>,
    mut cleanup: channel::Receiver<Fact>,
    ui: mpsc::SyncSender<Effect>,
    mut stop: watch::Receiver<bool>,
    configuration: (bool, PathBuf),
) {
    let (software, storage_root) = configuration;
    let lifetime = process.service.lock().expect("service").lifetime();
    let mut publisher = Publisher {
        service: process.service.clone(),
        lifetime,
        sequence: 0,
    };
    let (ble_tx, ble_rx) = channel::channel(16);
    #[cfg(feature = "software-bluetooth")]
    let ble = if software {
        tokio::spawn(crate::software::run(ble_rx, input, stop.clone()))
    } else {
        tokio::spawn(crate::bluetooth::run(ble_rx, input, stop.clone()))
    };
    #[cfg(not(feature = "software-bluetooth"))]
    let ble = {
        assert!(!software);
        tokio::spawn(crate::bluetooth::run(ble_rx, input, stop.clone()))
    };
    let mut pending_state = None;
    let mut storage_checked = 0;
    let mut timers: BTreeMap<u64, (u64, bool)> = BTreeMap::new();
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased;
            _ = stop.changed() => break,
            Some(fact) = cleanup.recv() => publisher.send(fact),
            Some(fact) = facts.recv() => publisher.send(fact),
            _ = tick.tick() => {
                publisher.send(Fact::Clock {});
                let now = monotonic_ms();
                if now.saturating_sub(storage_checked) >= 5_000 {
                    let available_bytes = rustix::fs::statvfs(&storage_root).map(|v| v.f_bavail.saturating_mul(v.f_frsize)).unwrap_or(0);
                    publisher.send(Fact::Storage { available_bytes });
                    storage_checked = now;
                }
                let due: Vec<_> = timers.iter().filter(|(_, (at, _))| *at <= now).map(|(id, _)| *id).collect();
                for id in due { timers.remove(&id); publisher.send(Fact::Timer { id }); }
            }
        }
        let effects = process.service.lock().expect("service").drain_os().effects;
        for effect in effects {
            match effect {
                Effect::ArmTimer {
                    id,
                    delay_ms,
                    connection,
                } => {
                    if timers.len() < MAX_TIMERS || timers.contains_key(&id) {
                        timers.insert(id, (monotonic_ms().saturating_add(delay_ms), connection));
                    } else {
                        publisher.send(Fact::StopRequested {});
                    }
                }
                Effect::CancelTimer { id } => {
                    timers.remove(&id);
                }
                Effect::CancelConnectionTimers => timers.retain(|_, (_, connection)| !*connection),
                Effect::StartLocation { .. } | Effect::StopLocation => {}
                Effect::RecoverPermission { id, .. } => {
                    publisher.send(Fact::UiCompleted { id, success: false })
                }
                Effect::PresentFile { id, .. } => {
                    if ui.try_send(effect).is_err() {
                        publisher.send(Fact::PresentationStarted {
                            id,
                            accepted: false,
                        });
                    }
                }
                Effect::HostState { .. } => {
                    pending_state = Some(effect);
                }
                _ => {
                    if ble_tx.try_send(effect).is_err() {
                        publisher.send(Fact::StopRequested {});
                    }
                }
            }
        }
        if let Some(state) = pending_state.take()
            && let Err(mpsc::TrySendError::Full(state)) = ui.try_send(state)
        {
            pending_state = Some(state);
        }
    }
    timers.clear();
    publisher.send(Fact::StopRequested {});
    let _ = tokio::time::timeout(Duration::from_secs(9), ble).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn storage_owner_and_port_conflicts_preserve_existing_data() {
        let root =
            std::env::temp_dir().join(format!("float-view-linux-owner-{}", std::process::id()));
        let second_root = root.with_extension("second");
        let host = Host::open_on(&root, b"html", false, 0).unwrap();
        std::fs::write(root.join("sentinel"), b"preserve").unwrap();
        assert!(Host::open_on(&root, b"html", false, 0).is_err());
        assert!(Host::open_on(&second_root, b"html", false, host.server.port()).is_err());
        assert_eq!(std::fs::read(root.join("sentinel")).unwrap(), b"preserve");
        host.shutdown().unwrap();
        drop(host);
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(second_root).unwrap();
    }

    #[test]
    fn inaccessible_and_corrupt_roots_fail_without_reset() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root =
            std::env::temp_dir().join(format!("float-view-linux-corrupt-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Host::open_on(&root, b"html", false, 0).is_err());
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let alias = root.with_extension("alias");
        symlink(&root, &alias).unwrap();
        assert!(Host::open_on(&alias, b"html", false, 0).is_err());
        let host = Host::open_on(&root, b"html", false, 0).unwrap();
        host.shutdown().unwrap();
        drop(host);
        let catalog = root.join("catalog.sqlite");
        std::fs::write(&catalog, b"corrupt development data").unwrap();
        assert!(Host::open_on(&root, b"html", false, 0).is_err());
        assert_eq!(std::fs::read(catalog).unwrap(), b"corrupt development data");
        std::fs::remove_file(alias).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shutdown_contention_is_bounded_and_retry_retains_storage_owner() {
        let root = std::env::temp_dir().join(format!(
            "float-view-linux-contention-{}",
            std::process::id()
        ));
        let host = Host::open_on(&root, b"html", false, 0).unwrap();
        let owner = host.process.service.clone();
        let guard = owner.lock().unwrap();
        assert!(host.ingress.send(Fact::Clock {}));
        let start = std::time::Instant::now();
        assert_eq!(host.shutdown(), Err("OS shutdown timed out"));
        assert!(start.elapsed() < Duration::from_secs(12));
        assert!(Host::open_on(&root, b"html", false, 0).is_err());
        drop(guard);
        host.shutdown().unwrap();
        drop(owner);
        drop(host);
        let reopened = Host::open_on(&root, b"html", false, 0).unwrap();
        reopened.shutdown().unwrap();
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn roots_are_absolute_and_xdg_has_priority() {
        assert_eq!(
            data_root(Some("/tmp/data"), Some("/home/test")).unwrap(),
            Path::new("/tmp/data/float-view")
        );
        assert_eq!(
            data_root(Some("relative"), Some("/home/test")).unwrap(),
            Path::new("/home/test/.local/share/float-view")
        );
        assert!(data_root(None, Some("relative")).is_err());
    }
    #[test]
    fn cleanup_has_independent_admission() {
        let (tx, _rx) = channel::channel(1);
        let (priority, _priority_rx) = channel::channel(1);
        let input = Ingress {
            ordinary: tx,
            cleanup: priority,
        };
        assert!(input.send(Fact::Clock {}));
        assert!(!input.send(Fact::Clock {}));
        assert!(input.cleanup(Fact::StopRequested {}));
    }
}
