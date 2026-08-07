use std::collections::VecDeque;

pub const MAX_PAYLOAD_BYTES: usize = 4096;
const MAX_STREAM_BYTES: usize = MAX_PAYLOAD_BYTES * 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidFrame,
    InvalidCrc,
    PayloadTooLarge,
    UnsupportedCommand(u8),
    TruncatedPayload,
    InvalidUtf8,
}

#[must_use]
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0_u16;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Encodes one bounded VESC payload.
///
/// # Errors
///
/// Rejects payloads larger than the protocol bound.
pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::PayloadTooLarge);
    }
    let mut frame = Vec::with_capacity(payload.len() + 6);
    if payload.len() <= u8::MAX.into() {
        frame.push(2);
        frame.push(u8::try_from(payload.len()).map_err(|_| ProtocolError::PayloadTooLarge)?);
    } else {
        let length = u16::try_from(payload.len()).map_err(|_| ProtocolError::PayloadTooLarge)?;
        frame.push(3);
        frame.extend_from_slice(&length.to_be_bytes());
    }
    frame.extend_from_slice(payload);
    frame.extend_from_slice(&crc16(payload).to_be_bytes());
    frame.push(3);
    Ok(frame)
}

/// Decodes exactly one bounded VESC frame.
///
/// # Errors
///
/// Rejects malformed headers/footers, lengths, and CRC values.
pub fn decode_frame(frame: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let (header, payload_len) = match frame.first() {
        Some(2) if frame.len() >= 5 => (2, usize::from(frame[1])),
        Some(3) if frame.len() >= 6 => (3, usize::from(u16::from_be_bytes([frame[1], frame[2]]))),
        _ => return Err(ProtocolError::InvalidFrame),
    };
    if payload_len > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::PayloadTooLarge);
    }
    let expected = header + payload_len + 3;
    if frame.len() != expected || frame.last() != Some(&3) {
        return Err(ProtocolError::InvalidFrame);
    }
    let payload = &frame[header..header + payload_len];
    let received =
        u16::from_be_bytes([frame[header + payload_len], frame[header + payload_len + 1]]);
    if crc16(payload) != received {
        return Err(ProtocolError::InvalidCrc);
    }
    Ok(payload.to_vec())
}

#[derive(Default)]
pub struct FrameStreamDecoder {
    buffer: VecDeque<u8>,
}

impl FrameStreamDecoder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    /// Appends one transport chunk and returns all complete payloads.
    ///
    /// # Errors
    ///
    /// Clears the buffer and rejects aggregate or declared oversized frames,
    /// malformed frames, and invalid CRCs.
    pub fn append(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, ProtocolError> {
        if self.buffer.len().saturating_add(bytes.len()) > MAX_STREAM_BYTES {
            self.buffer.clear();
            return Err(ProtocolError::PayloadTooLarge);
        }
        self.buffer.extend(bytes);
        let mut payloads = Vec::new();
        loop {
            while matches!(self.buffer.front(), Some(value) if *value != 2 && *value != 3) {
                self.buffer.pop_front();
            }
            let Some(first) = self.buffer.front().copied() else {
                return Ok(payloads);
            };
            let header = if first == 2 { 2 } else { 3 };
            if self.buffer.len() < header {
                return Ok(payloads);
            }
            let payload_len = if header == 2 {
                usize::from(self.buffer[1])
            } else {
                usize::from(u16::from_be_bytes([self.buffer[1], self.buffer[2]]))
            };
            if payload_len > MAX_PAYLOAD_BYTES {
                self.buffer.pop_front();
                return Err(ProtocolError::PayloadTooLarge);
            }
            let frame_len = header + payload_len + 3;
            if self.buffer.len() < frame_len {
                return Ok(payloads);
            }
            let frame: Vec<u8> = self.buffer.drain(..frame_len).collect();
            payloads.push(decode_frame(&frame)?);
        }
    }
}
