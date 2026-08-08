use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;
use std::time::Duration;

use app_coordinator::{
    ApplicationActor, DeterministicClock, DeterministicRandom, RuntimeGeneration,
};
use host_ports::PlatformCommand;
use infra_loopback_server::{
    ContractDispatcher, LoopbackServer, MemoryResources, ResourceProvider, ServerConfig,
    SessionHttpApi, StaticResource, StorageSessionHttpApi, start_server,
};
use infra_sqlite_storage::{CatalogStore, SessionStore};

use super::NativeBackend;
#[cfg(feature = "software-host")]
use super::software_host::SoftwareHost;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportMode {
    Physical,
    Software,
}

/// Inputs selected by either deterministic conformance adapters or a native host bootstrap.
pub struct BackendRuntimeConfig {
    pub generation: u64,
    pub requested_port: u16,
    pub storage_root: PathBuf,
    pub platform: String,
    pub launch_token: String,
    pub resources: Arc<dyn ResourceProvider>,
    pub application: Option<Arc<ApplicationActor>>,
    pub transport: TransportMode,
    pub platform_commands: Option<SyncSender<PlatformCommand>>,
}

/// One production composition shared by standalone conformance and native hosts.
pub struct BackendRuntime {
    application: Arc<ApplicationActor>,
    server: Option<LoopbackServer>,
    platform: String,
    resources: Arc<dyn ResourceProvider>,
    dispatcher: Arc<dyn ContractDispatcher>,
    background_pump: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
    session_http: Arc<dyn SessionHttpApi>,
    launch_token: String,
    requested_port: u16,
}

impl BackendRuntime {
    /// Opens durable stores, starts the actor/dispatcher, and binds authenticated loopback.
    ///
    /// # Errors
    /// Returns a bounded startup diagnostic if storage, actor, or loopback startup fails.
    pub async fn start(config: BackendRuntimeConfig) -> Result<Self, String> {
        let catalog = Arc::new(
            CatalogStore::open(&config.storage_root)
                .map_err(|error| format!("storage startup failed: {error:?}"))?,
        );
        let software_host = config.transport == TransportMode::Software;
        if software_host && !cfg!(feature = "software-host") {
            return Err("software host support is not compiled into this runtime".into());
        }
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let application = if let Some(application) = config.application {
            application
        } else {
            ApplicationActor::start(
                RuntimeGeneration(config.generation),
                DeterministicClock::new(0, 0, 1),
                DeterministicRandom::new(config.generation),
            )
            .map_err(|error| format!("application startup failed: {error:?}"))?
        };
        let dispatcher: Arc<dyn ContractDispatcher> = Arc::new(NativeBackend::new(
            Arc::clone(&application),
            Arc::clone(&catalog),
            Arc::clone(&sessions),
        )?);
        let session_http: Arc<dyn SessionHttpApi> =
            Arc::new(StorageSessionHttpApi::new(Arc::clone(&catalog)));
        let server = start_server(
            ServerConfig::new(
                "",
                &config.launch_token,
                &config.platform,
                Arc::clone(&config.resources),
                Arc::clone(&dispatcher),
            )
            .with_bind_port(config.requested_port)
            .with_session_http(Arc::clone(&session_http)),
        )
        .await
        .map_err(|error| format!("loopback startup failed: {error}"))?;
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = Arc::clone(&stopped);
        let worker_dispatcher = Arc::clone(&dispatcher);
        let worker_application = Arc::clone(&application);
        let platform_commands = config.platform_commands;
        let worker = std::thread::Builder::new()
            .name(
                if software_host {
                    "float-view-software-host"
                } else {
                    "float-view-native-pump"
                }
                .into(),
            )
            .spawn(move || {
                if !software_host {
                    let _ = worker_application.register_pump_thread(std::thread::current());
                }
                #[cfg(feature = "software-host")]
                let mut software = software_host.then(SoftwareHost::default);
                #[cfg(feature = "software-host")]
                if let Some(software) = software.as_mut() {
                    let _ = software.initialize(&worker_application);
                }
                while !worker_stopped.load(Ordering::Acquire) {
                    #[cfg(feature = "software-host")]
                    if let Some(software) = software.as_mut() {
                        software.pump(&worker_application, platform_commands.as_ref());
                    }
                    worker_dispatcher.pump();
                    std::thread::park_timeout(if software_host {
                        Duration::from_millis(10)
                    } else {
                        worker_dispatcher.next_pump_wait()
                    });
                }
            })
            .map_err(|error| format!("backend pump startup failed: {error}"))?;
        Ok(Self {
            application,
            server: Some(server),
            platform: config.platform,
            resources: config.resources,
            dispatcher,
            background_pump: Some((stopped, worker)),
            session_http,
            launch_token: config.launch_token,
            requested_port: config.requested_port,
        })
    }

    /// Returns the currently bound loopback address.
    ///
    /// # Panics
    /// Panics only while the loopback portion is suspended.
    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.server.as_ref().expect("running loopback").address
    }

    #[must_use]
    pub fn launch_token(&self) -> &str {
        &self.launch_token
    }

    #[must_use]
    pub fn application(&self) -> Arc<ApplicationActor> {
        Arc::clone(&self.application)
    }

    pub async fn suspend(&mut self) {
        if let Some(server) = self.server.take() {
            server.stop().await;
        }
    }

    /// # Errors
    /// Returns a bounded startup diagnostic when loopback cannot bind.
    pub async fn resume(&mut self) -> Result<SocketAddr, String> {
        if self.server.is_none() {
            self.server = Some(
                start_server(
                    ServerConfig::new(
                        "",
                        &self.launch_token,
                        &self.platform,
                        Arc::clone(&self.resources),
                        Arc::clone(&self.dispatcher),
                    )
                    .with_bind_port(self.requested_port)
                    .with_session_http(Arc::clone(&self.session_http)),
                )
                .await
                .map_err(|error| format!("loopback resume failed: {error}"))?,
            );
        }
        Ok(self.address())
    }

    /// # Errors
    /// Returns a bounded shutdown diagnostic if the application actor cannot stop cleanly.
    pub async fn stop(mut self) -> Result<(), String> {
        self.suspend().await;
        if let Some((stopped, worker)) = self.background_pump.take() {
            stopped.store(true, Ordering::Release);
            worker.thread().unpark();
            worker
                .join()
                .map_err(|_| "native pump shutdown failed".to_owned())?;
        }
        self.application
            .stop()
            .map_err(|error| format!("application shutdown failed: {error:?}"))
    }
}

#[must_use]
pub fn standalone_storage_root() -> PathBuf {
    std::env::temp_dir().join(format!("float-view-standalone-{}", std::process::id()))
}

#[must_use]
pub fn standalone_resources() -> Arc<dyn ResourceProvider> {
    resources_with_index(Arc::from(
        &b"<!doctype html><title>Float View Rust</title>"[..],
    ))
}

#[must_use]
pub fn resources_with_index(bytes: Arc<[u8]>) -> Arc<dyn ResourceProvider> {
    Arc::new(MemoryResources::new([(
        "index.html".into(),
        StaticResource {
            content_type: "text/html; charset=utf-8",
            bytes,
        },
    )]))
}

pub fn cleanup_standalone_storage(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
}
