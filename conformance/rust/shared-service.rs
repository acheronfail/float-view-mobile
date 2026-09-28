//! Test-only OS relay. All portable behavior comes from the production constructor.
use float_view_backend_service::{Process, os::Envelope};
use float_view_loopback_server::{backend::Backend, protocol::Platform, server::Server};
use std::{
    io::{self, BufRead, Write},
    path::Path,
    sync::Arc,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args().nth(1).ok_or("root required")?;
    let process = Process::open(Path::new(&root))?;
    let backend = Arc::new(Backend::with_service(process.service.clone()));
    let server = Server::with_backend(
        Platform::Macos,
        b"<!doctype html><title>Direct service</title>",
        backend,
    )
    .map_err(|_| "server")?;
    let port = server.start(0).map_err(|_| "start")?;
    println!(
        "{}",
        serde_json::json!({"event":"ready","lifetime":process.service.lock().map_err(|_|"owner")?.lifetime(),"url":format!("ws://127.0.0.1:{port}/api/ws?token={}",server.token()),"origin":format!("http://127.0.0.1:{port}")})
    );
    io::stdout().flush()?;
    let mut input = io::stdin().lock();
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = (&mut input).take(1_048_578).read_until(b'\n', &mut line)?;
        if n == 0 {
            break;
        }
        if n > 1_048_577 || line.last() != Some(&b'\n') {
            return Err("bounded input".into());
        }
        let input: serde_json::Value = serde_json::from_slice(&line)?;
        let id = input["id"].as_u64().ok_or("id")?;
        let mut s = process.service.lock().map_err(|_| "owner")?;
        let result = if input["command"] == "fact" {
            let envelope: Envelope = serde_json::from_value(input["envelope"].clone())?;
            s.apply_os(envelope)
                .map(|_| serde_json::Value::Null)
                .map_err(|e| format!("{e:?}"))
        } else if input["command"] == "drain" {
            if !input["envelope"].is_null() {
                s.apply_os(serde_json::from_value(input["envelope"].clone())?)
                    .map_err(|_| "clock")?;
            }
            Ok(serde_json::to_value(s.drain_os())?)
        } else {
            Err("invalid command".into())
        };
        println!("{}", serde_json::json!({"id":id,"result":result}));
        io::stdout().flush()?;
    }
    server.stop().map_err(|_| "stop")?;
    process.shutdown().map_err(|_| "shutdown")?;
    Ok(())
}
use std::io::Read;
