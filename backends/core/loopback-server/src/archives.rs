//! HTTP archive streaming through bounded caller-owned binary chunks.
use crate::{
    backend::Backend,
    dispatch::Params,
    facts::Completion,
    limits::*,
    protocol::Operation,
    server::{App, Client, json_response, request_body, status},
    transfer::Direction,
};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
struct Lease {
    backend: Arc<Backend>,
    id: u64,
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut cleanup) = self.backend.cleanup.lock()
            && !cleanup.contains(&self.id)
        {
            cleanup.push(self.id);
        }
        self.backend.changed.notify_waiters();
    }
}
fn lease(client: &Client, direction: Direction, maximum: u64) -> Result<Lease, StatusCode> {
    let id = client
        .backend
        .transfers
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .create(client.generation, direction, maximum)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Lease {
        backend: client.backend.clone(),
        id,
    })
}
pub(crate) async fn import(State(app): State<Arc<App>>, request: Request) -> Response {
    if request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|n| n > MAX_SESSION_IMPORT_BYTES as u64)
    {
        return status(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let client = match Client::new(&app.backend) {
        Ok(c) => c,
        Err(e) => return status(e),
    };
    let transfer = match lease(&client, Direction::Import, MAX_SESSION_IMPORT_BYTES as u64) {
        Ok(t) => t,
        Err(e) => return status(e),
    };
    if app
        .backend
        .call(
            client.id,
            Operation::BeginImport {
                transfer_id: transfer.id,
            },
        )
        .await
        .is_err()
    {
        return status(StatusCode::BAD_REQUEST);
    }
    let mut body = request.into_body().into_data_stream();
    let mut offset = 0_u64;
    while let Some(chunk) = body.next().await {
        let Ok(chunk) = chunk else {
            return status(StatusCode::BAD_REQUEST);
        };
        for bytes in chunk.chunks(MAX_TRANSFER_CHUNK_BYTES) {
            if offset + bytes.len() as u64 > MAX_SESSION_IMPORT_BYTES as u64 {
                return status(StatusCode::PAYLOAD_TOO_LARGE);
            }
            if app
                .backend
                .transfers
                .lock()
                .map_err(|_| ())
                .and_then(|mut t| {
                    t.put(
                        client.generation,
                        transfer.id,
                        Direction::Import,
                        offset,
                        bytes,
                    )
                    .map_err(|_| ())
                })
                .is_err()
            {
                return status(StatusCode::BAD_REQUEST);
            }
            if app
                .backend
                .call(
                    client.id,
                    Operation::WriteTransfer {
                        transfer_id: transfer.id,
                        offset,
                    },
                )
                .await
                .is_err()
            {
                return status(StatusCode::BAD_REQUEST);
            }
            offset += bytes.len() as u64;
        }
    }
    match app
        .backend
        .call(
            client.id,
            Operation::FinishImport {
                transfer_id: transfer.id,
            },
        )
        .await
    {
        Ok(Completion::Imported(id)) => json_response(json!({"sessionId":id})),
        _ => status(StatusCode::BAD_REQUEST),
    }
}
pub(crate) async fn export(State(app): State<Arc<App>>, request: Request) -> Response {
    let bytes = match request_body(request, MAX_SESSION_DATA_REQUEST_BYTES).await {
        Ok(b) => b,
        Err(e) => return status(e),
    };
    let request: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return status(StatusCode::BAD_REQUEST),
    };
    let Some(object) = request.as_object() else {
        return status(StatusCode::BAD_REQUEST);
    };
    let p = Params(object);
    let (Ok(id), Ok(format)) = (p.text("sessionId"), p.text("format")) else {
        return status(StatusCode::BAD_REQUEST);
    };
    if p.only(&["sessionId", "format"]).is_err()
        || format != "csv" && format != "float-view-archive"
    {
        return status(StatusCode::BAD_REQUEST);
    }
    let client = match Client::new(&app.backend) {
        Ok(c) => c,
        Err(e) => return status(e),
    };
    let transfer = match lease(&client, Direction::Export, u64::MAX) {
        Ok(t) => t,
        Err(e) => return status(e),
    };
    let Ok(Completion::Export {
        file_name,
        length,
        transfer_id,
        ..
    }) = app
        .backend
        .call(
            client.id,
            Operation::ExportSession {
                id,
                format: format.clone(),
                present: false,
                transfer_id: transfer.id,
            },
        )
        .await
    else {
        return status(StatusCode::BAD_REQUEST);
    };
    if transfer_id != transfer.id
        || file_name.len() > 255
        || file_name.contains(['\r', '\n', '"', '/', '\\'])
    {
        return status(StatusCode::BAD_REQUEST);
    }
    let stream = futures_util::stream::try_unfold(
        (client, transfer, 0_u64),
        move |(client, transfer, offset)| async move {
            if offset == length {
                return Ok(None);
            }
            let count = (length - offset).min(MAX_TRANSFER_CHUNK_BYTES as u64) as usize;
            client
                .backend
                .call(
                    client.id,
                    Operation::ReadTransfer {
                        transfer_id: transfer.id,
                        offset,
                        length: count,
                    },
                )
                .await
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::Interrupted))?;
            let bytes = client
                .backend
                .transfers
                .lock()
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::Interrupted))?
                .take(client.generation, transfer.id, Direction::Export)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::Interrupted))?;
            if bytes.len() != count {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }
            Ok(Some((bytes, (client, transfer, offset + count as u64))))
        },
    );
    (
        [
            (
                header::CONTENT_TYPE,
                if format == "csv" {
                    "text/csv".to_owned()
                } else {
                    "application/octet-stream".to_owned()
                },
            ),
            (header::CACHE_CONTROL, "no-store".into()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{file_name}\""),
            ),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}
