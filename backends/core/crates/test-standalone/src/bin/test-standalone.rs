use app_backend::{
    BackendRuntime, BackendRuntimeConfig, TransportMode, cleanup_standalone_storage,
    standalone_resources, standalone_storage_root,
};
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    let root = standalone_storage_root();
    cleanup_standalone_storage(&root);
    let runtime = BackendRuntime::start(BackendRuntimeConfig {
        generation: 1,
        requested_port: 0,
        storage_root: root.clone(),
        platform: "macos".into(),
        launch_token: "standalone-token".into(),
        resources: standalone_resources(),
        application: None,
        transport: TransportMode::Software,
        platform_commands: None,
    })
    .await
    .expect("start standalone production runtime");
    println!(
        "FLOAT_VIEW_RUST_WS_URL=ws://{}/api/ws?token={}",
        runtime.address(),
        runtime.launch_token()
    );
    let mut input = tokio::io::stdin();
    let mut byte = [0_u8; 1];
    let _ = input.read(&mut byte).await;
    runtime
        .stop()
        .await
        .expect("stop standalone production runtime");
    cleanup_standalone_storage(&root);
}
