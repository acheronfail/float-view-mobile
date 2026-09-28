mod support;
use float_view_loopback_server::{
    host,
    limits::*,
    protocol::{Envelope, Platform},
    server::{Server, ServerError},
    transfer::{Direction, TransferError, Transfers},
};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

fn http(server: &Server, path: &str, headers: &str, body: &[u8]) -> Vec<u8> {
    let mut socket = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(
        socket,
        "{} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n{headers}\r\n",
        if body.is_empty() { "GET" } else { "POST" },
        server.port()
    )
    .unwrap();
    socket.write_all(body).unwrap();
    let mut response = vec![];
    if let Err(error) = socket.read_to_end(&mut response) {
        assert!(
            error.kind() == std::io::ErrorKind::ConnectionReset && !response.is_empty(),
            "{error}"
        );
    }
    response
}
fn status(response: &[u8], code: &str) {
    assert!(
        response.starts_with(format!("HTTP/1.1 {code}").as_bytes()),
        "{}",
        String::from_utf8_lossy(response)
    );
}

#[test]
fn listener_lifecycle_preserves_assets_token_and_rejects_failed_bind() {
    let html = b"<!doctype html>\n<body>\xF0\x9F\x9A\xB2</body>";
    let server = Server::new(Platform::Macos, html).unwrap();
    let token = server.token().to_owned();
    let port = server.start(0).unwrap();
    assert_eq!(server.start(port), Err(ServerError::AlreadyRunning));
    let second = Server::new(Platform::Macos, html).unwrap();
    assert_eq!(second.start(port), Err(ServerError::Bind));
    let response = http(&server, "/", "", b"");
    status(&response, "200");
    assert!(response.ends_with(html));
    status(&http(&server, "/api/bootstrap", "", b""), "503");
    server
        .backend
        .publish(float_view_loopback_server::protocol::Publication {
            generation: server
                .backend
                .generation
                .load(std::sync::atomic::Ordering::Acquire),
            sequence: 1,
            catalog_changed: false,
            facts: support::facts(),
        })
        .unwrap();
    let response = http(&server, "/api/bootstrap", "", b"");
    status(&response, "200");
    let offset = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    let bootstrap: serde_json::Value = serde_json::from_slice(&response[offset..]).unwrap();
    assert_eq!(bootstrap["sessionToken"], token);
    status(&http(&server, "/missing", "", b""), "404");
    server.stop().unwrap();
    server.stop().unwrap();
    second.start(port).unwrap();
    second.stop().unwrap();
    server.start(port).unwrap();
    assert_eq!(server.token(), token);
    status(&http(&server, "/api/bootstrap", "", b""), "200");
    server.stop().unwrap();
}

#[test]
fn authentication_precedes_body_and_host_admission() {
    let server = Server::new(Platform::Android, b"html").unwrap();
    server.start(0).unwrap();
    let generation = server
        .backend
        .generation
        .load(std::sync::atomic::Ordering::Acquire);
    for route in [
        "/api/session-data",
        "/api/session-import",
        "/api/session-export",
    ] {
        for headers in [
            String::new(),
            format!(
                "Origin: http://evil.example\r\nX-Float-View-Token: {}\r\n",
                server.token()
            ),
            format!(
                "Origin: http://127.0.0.1:{}\r\nX-Float-View-Token: wrong\r\n",
                server.port()
            ),
            format!(
                "Origin: http://127.0.0.1:{}\r\nOrigin: http://127.0.0.1:{}\r\nX-Float-View-Token: {}\r\n",
                server.port(),
                server.port(),
                server.token()
            ),
        ] {
            status(
                &http(
                    &server,
                    route,
                    &format!("{headers}Content-Length: 2\r\n"),
                    b"{}",
                ),
                "403",
            );
        }
    }
    assert!(
        server
            .backend
            .exchange
            .drain(generation, Duration::ZERO)
            .unwrap()
            .is_empty()
    );
    let headers = format!(
        "Origin: http://127.0.0.1:{}\r\nX-Float-View-Token: {}\r\nContent-Length: {}\r\n",
        server.port(),
        server.token(),
        MAX_SESSION_DATA_REQUEST_BYTES + 1
    );
    status(&http(&server, "/api/session-data", &headers, b"x"), "413");
    assert!(
        server
            .backend
            .exchange
            .drain(generation, Duration::ZERO)
            .unwrap()
            .is_empty()
    );
    server.stop().unwrap();
}

#[test]
fn host_envelopes_are_closed_versioned_and_bounded() {
    for invalid in [
        r#"{}"#,
        r#"{"version":2,"action":{"command":"stop"}}"#,
        r#"{"version":1,"extra":0,"action":{"command":"stop"}}"#,
        r#"{"version":1,"action":{"command":"drain","parameters":{"generation":1,"wait_ms":251}}}"#,
        r#"{"version":1,"action":{"command":"start","parameters":{"platform":"ios","port":11010}}}"#,
    ] {
        assert!(Envelope::decode(invalid.as_bytes()).is_err());
    }
    assert!(Envelope::decode(&vec![b' '; MAX_HOST_ENVELOPE_BYTES + 1]).is_err());
    let handle = host::create(1, b"html").unwrap();
    assert_eq!(host::command(handle,br#"{"version":1,"action":{"command":"start","parameters":{"platform":"macos","port":11010}}}"#),Err(host::Status::InvalidEnvelope));
    assert_eq!(host::destroy(handle), host::Status::Ok);
    assert_eq!(host::destroy(handle), host::Status::Ok);
    assert_eq!(
        host::command(handle, br#"{"version":1,"action":{"command":"stop"}}"#),
        Err(host::Status::InvalidHandle)
    );
    assert_eq!(host::destroy(0), host::Status::InvalidHandle);
}

#[test]
fn transfers_bound_memory_and_reject_replays_foreign_generations_and_directions() {
    let mut pool = Transfers::default();
    let a = pool
        .create(1, Direction::Import, MAX_TRANSFER_CHUNK_BYTES as u64 + 1)
        .unwrap();
    let b = pool.create(1, Direction::Export, 100).unwrap();
    assert_eq!(
        pool.create(1, Direction::Import, 1),
        Err(TransferError::Full)
    );
    assert_eq!(
        pool.put(2, a, Direction::Import, 0, b"x"),
        Err(TransferError::Stale)
    );
    assert_eq!(
        pool.put(1, a, Direction::Export, 0, b"x"),
        Err(TransferError::Stale)
    );
    assert_eq!(
        pool.put(
            1,
            a,
            Direction::Import,
            0,
            &vec![0; MAX_TRANSFER_CHUNK_BYTES + 1]
        ),
        Err(TransferError::InvalidChunk)
    );
    pool.put(
        1,
        a,
        Direction::Import,
        0,
        &vec![42; MAX_TRANSFER_CHUNK_BYTES],
    )
    .unwrap();
    assert_eq!(
        pool.put(
            1,
            a,
            Direction::Import,
            MAX_TRANSFER_CHUNK_BYTES as u64,
            b"x"
        ),
        Err(TransferError::Busy)
    );
    assert_eq!(
        pool.take(1, a, Direction::Import).unwrap().len(),
        MAX_TRANSFER_CHUNK_BYTES
    );
    assert_eq!(
        pool.put(1, a, Direction::Import, 0, b"x"),
        Err(TransferError::InvalidChunk)
    );
    assert_eq!(
        pool.put(
            1,
            a,
            Direction::Import,
            MAX_TRANSFER_CHUNK_BYTES as u64,
            b"xx"
        ),
        Err(TransferError::TooLarge)
    );
    pool.put(
        1,
        a,
        Direction::Import,
        MAX_TRANSFER_CHUNK_BYTES as u64,
        b"x",
    )
    .unwrap();
    assert_eq!(pool.clear(), vec![a, b]);
    assert_eq!(
        pool.take(1, a, Direction::Import),
        Err(TransferError::Stale)
    );
    assert!(pool.create(2, Direction::Import, 1).unwrap() > b);
}

#[test]
fn large_typed_results_are_framed_without_opening_the_domain_schema() {
    use float_view_loopback_server::{
        completion::{Completions, Fragment},
        exchange::OperationId,
    };
    let mut completions = Completions::default();
    let id = OperationId {
        generation: 1,
        sequence: 1,
    };
    assert!(
        completions
            .append(Fragment {
                id,
                offset: 1,
                text: "x".into(),
                last: false
            })
            .is_err()
    );
    let result = r#"{"status":"completed","value":{"kind":"imported","facts":"large result"}}"#;
    let middle = 32;
    assert!(
        completions
            .append(Fragment {
                id,
                offset: 0,
                text: result[..middle].into(),
                last: false
            })
            .unwrap()
            .is_none()
    );
    assert!(
        completions
            .append(Fragment {
                id,
                offset: 0,
                text: "replay".into(),
                last: false
            })
            .is_err()
    );
    assert!(
        completions
            .append(Fragment {
                id,
                offset: middle,
                text: result[middle..].into(),
                last: true
            })
            .unwrap()
            .is_some()
    );
    assert!(
        completions
            .append(Fragment {
                id,
                offset: 0,
                text: r#"{"status":"completed","value":{"frontendResponse":{}}}"#.into(),
                last: true
            })
            .is_err()
    );
    for sequence in 1..=2 {
        assert!(
            completions
                .append(Fragment {
                    id: OperationId {
                        generation: 1,
                        sequence
                    },
                    offset: 0,
                    text: "{".into(),
                    last: false
                })
                .unwrap()
                .is_none()
        );
    }
    assert!(
        completions
            .append(Fragment {
                id: OperationId {
                    generation: 1,
                    sequence: 3
                },
                offset: 0,
                text: "{".into(),
                last: false
            })
            .is_err()
    );
    completions.clear();
    assert!(
        completions
            .append(Fragment {
                id,
                offset: 0,
                text: result.into(),
                last: true
            })
            .unwrap()
            .is_some()
    );
}

#[test]
fn connection_admission_rejects_seventeenth_and_recovers() {
    let server = Server::new(Platform::Macos, b"html").unwrap();
    server.start(0).unwrap();
    let mut sockets = Vec::new();
    for _ in 0..MAX_CONNECTIONS {
        let mut socket = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        socket
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = [0; 512];
        let n = socket.read(&mut response).unwrap();
        status(&response[..n], "200");
        sockets.push(socket);
    }
    let mut excess = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
    excess
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let _ = excess.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let mut byte = [0];
    let rejected = excess.read(&mut byte);
    assert!(
        matches!(rejected, Ok(0))
            || matches!(rejected, Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset)
    );
    drop(sockets);
    server.stop().unwrap();
    server.start(0).unwrap();
    status(&http(&server, "/", "", b""), "200");
    server.stop().unwrap();
}

#[test]
fn malformed_history_cursor_is_rejected_before_catalog_lookup() {
    use base64::Engine;
    let server = Server::new(Platform::Macos, b"html").unwrap();
    server.start(0).unwrap();
    let generation = server
        .backend
        .generation
        .load(std::sync::atomic::Ordering::Acquire);
    let cursor = base64::engine::general_purpose::STANDARD.encode(br#"{"sessionId":"foreign","startElapsedMs":0,"endElapsedMs":1000,"historyRevision":1,"locationSequence":1}"#);
    let body = serde_json::to_vec(&serde_json::json!({"kind":"locations","sessionId":"owned","snapshot":{"historyRevision":1,"committedSampleSequence":1,"committedEventSequence":1,"committedElapsedMs":1000,"committedLocationSequence":1,"committedLocationElapsedMs":1000},"range":{"startElapsedMs":0,"endElapsedMs":1000},"limit":10,"cursor":cursor})).unwrap();
    let headers = format!(
        "Origin: http://127.0.0.1:{}\r\nX-Float-View-Token: {}\r\nContent-Length: {}\r\n",
        server.port(),
        server.token(),
        body.len()
    );
    status(&http(&server, "/api/session-data", &headers, &body), "400");
    assert!(
        server
            .backend
            .exchange
            .drain(generation, Duration::ZERO)
            .unwrap()
            .is_empty()
    );
    server.stop().unwrap();
}

#[test]
fn oversized_host_result_returns_one_correlated_error() {
    use float_view_loopback_server::{
        backend::Backend,
        facts::Completion,
        protocol::{CompletionRecord, Outcome},
    };
    use std::sync::Arc;
    let backend = Arc::new(Backend::default());
    let generation = backend.start().unwrap();
    let host = backend.clone();
    let worker = std::thread::spawn(move || {
        let work = host
            .exchange
            .drain(generation, MAX_HOST_WAIT)
            .unwrap()
            .remove(0);
        let tune = serde_json::from_value(serde_json::json!({
            "configurationId":"tune","revision":1,"name":"test","description":null,
            "identity":{"packageName":"refloat","packageVersion":"1","schemaFingerprint":"a".repeat(64),"wireSignature":1},
            "valuesJson":serde_json::json!({"text":{"type":"string","value":"a".repeat(MAX_MESSAGE_BYTES)}}).to_string(),
            "createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"
        })).unwrap();
        host.complete(CompletionRecord {
            id: work.id,
            outcome: Outcome::Completed(Box::new(Completion::Tune(tune))),
        })
        .unwrap();
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let reply = runtime.block_on(backend.request(
        1,
        serde_json::json!({"id":73,"method":"refloat.tune.get","params":{"tuneId":"tune"}}),
    ));
    assert_eq!(reply["id"], 73);
    assert_eq!(reply["error"]["code"], "unavailable");
    assert!(reply.get("result").is_none());
    worker.join().unwrap();
    backend.stop();
}
