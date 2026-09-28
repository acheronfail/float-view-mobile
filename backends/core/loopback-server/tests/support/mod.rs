use float_view_backend_service::BackendService;
use float_view_loopback_server::{
    backend::Backend,
    protocol::Platform,
    server::{Server, ServerError},
};
use std::{
    ops::Deref,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct TestServer {
    server: Option<Server>,
    root: PathBuf,
}
impl Deref for TestServer {
    type Target = Server;
    fn deref(&self) -> &Server {
        self.server.as_ref().unwrap()
    }
}
impl Drop for TestServer {
    fn drop(&mut self) {
        drop(self.server.take());
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
pub fn server(platform: Platform, html: &[u8]) -> Result<TestServer, ServerError> {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "float-view-transport-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let service = Arc::new(Mutex::new(BackendService::open(&root).unwrap()));
    let server = Server::with_backend(platform, html, Arc::new(Backend::with_service(service)))?;
    Ok(TestServer {
        server: Some(server),
        root,
    })
}
