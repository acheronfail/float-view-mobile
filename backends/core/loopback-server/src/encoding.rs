//! Bounded response workers; dropping the request interrupts serialization/gzip.
use crate::limits::*;
use std::{
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
struct Cancellation(Arc<AtomicBool>);
impl Drop for Cancellation {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
struct Buffer {
    bytes: Vec<u8>,
    cancelled: Arc<AtomicBool>,
}
impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.bytes.len().saturating_add(bytes.len()) > MAX_SESSION_DATA_RESPONSE_BYTES {
            return Err(io::ErrorKind::FileTooLarge.into());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub async fn response(
    value: serde_json::Value,
    gzip: bool,
    permit: Arc<tokio::sync::OwnedSemaphorePermit>,
) -> Result<Vec<u8>, ()> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let _guard = Cancellation(cancelled.clone());
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut buffer = Buffer {
            bytes: Vec::new(),
            cancelled: cancelled.clone(),
        };
        serde_json::to_writer(&mut buffer, &value).map_err(|_| ())?;
        if !gzip {
            return Ok(buffer.bytes);
        }
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        for chunk in buffer.bytes.chunks(MAX_TRANSFER_CHUNK_BYTES) {
            if cancelled.load(Ordering::Acquire) {
                return Err(());
            }
            encoder.write_all(chunk).map_err(|_| ())?;
        }
        encoder.finish().map_err(|_| ())
    })
    .await
    .map_err(|_| ())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoded_boundary_precedes_compression_and_cancellation_is_sticky() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut buffer = Buffer {
            bytes: Vec::new(),
            cancelled: cancelled.clone(),
        };
        buffer
            .write_all(&vec![b'a'; MAX_SESSION_DATA_RESPONSE_BYTES])
            .unwrap();
        assert_eq!(
            buffer.write(b"b").unwrap_err().kind(),
            io::ErrorKind::FileTooLarge
        );
        cancelled.store(true, Ordering::Release);
        assert_eq!(
            buffer.write(b"").unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(buffer.bytes.len(), MAX_SESSION_DATA_RESPONSE_BYTES);
    }
    #[test]
    fn gzip_roundtrip_and_two_read_admission_recover() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(2)
            .build()
            .unwrap();
        runtime.block_on(async {
            let admission = Arc::new(tokio::sync::Semaphore::new(MAX_HISTORY_READS));
            let first = Arc::new(admission.clone().try_acquire_owned().unwrap());
            let second = Arc::new(admission.clone().try_acquire_owned().unwrap());
            assert!(admission.clone().try_acquire_owned().is_err());
            let value =
                serde_json::json!({"data":"a".repeat(MAX_SESSION_DATA_RESPONSE_BYTES - 11)});
            let compressed = response(value.clone(), true, first).await.unwrap();
            let mut decoded = Vec::new();
            std::io::Read::read_to_end(
                &mut flate2::read::GzDecoder::new(compressed.as_slice()),
                &mut decoded,
            )
            .unwrap();
            assert_eq!(decoded.len(), MAX_SESSION_DATA_RESPONSE_BYTES);
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&decoded).unwrap(),
                value
            );
            let oversized = serde_json::json!({"data":"a".repeat(MAX_SESSION_DATA_RESPONSE_BYTES)});
            assert!(response(oversized, true, second).await.is_err());
            assert_eq!(admission.available_permits(), MAX_HISTORY_READS);
        });
    }
}
