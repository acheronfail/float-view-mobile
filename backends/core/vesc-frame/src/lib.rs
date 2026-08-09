//! Safe bounded VESC framing implementation.
//!
//! The shared iOS/macOS backend selects this crate for CRC, encoding, exact
//! decoding, and stream-boundary interpretation through a narrow Apple bridge.
//! Android retains its independent Kotlin implementation. VESC payload
//! protocols and application-owned stream lifetime remain outside this crate.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Largest payload accepted by the shipped Float View framing implementations.
pub const MAX_PAYLOAD_BYTES: usize = 4_096;

/// Largest aggregate fragment buffer retained by the stream decoder.
pub const MAX_STREAM_BYTES: usize = MAX_PAYLOAD_BYTES * 2;

const SHORT_FRAME_HEADER: u8 = 2;
const LONG_FRAME_HEADER: u8 = 3;
const FRAME_FOOTER: u8 = 3;
const FRAME_TRAILER_BYTES: usize = 3;

/// Bounded framing failures shared by exact and streaming decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    /// The frame structure, length, header, or footer is invalid.
    InvalidFrame,
    /// The payload does not match the frame's CRC-16.
    InvalidCrc,
    /// A payload or aggregate stream exceeds its product limit.
    PayloadTooLarge,
}

/// One stateless interpretation of a bounded VESC notification byte region.
///
/// Offsets and lengths always refer to the borrowed input supplied to
/// [`scan_next`]. Callers may mechanically consume the reported prefix without
/// reproducing VESC header or length rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamScan {
    /// No complete candidate is present. Disposable leading bytes may be
    /// removed while the remaining suffix is retained for a later fragment.
    NeedMore {
        /// Number of disposable leading bytes.
        consumed: usize,
    },
    /// One complete candidate frame is available for exact decoding.
    Frame {
        /// Start of the complete frame after any leading noise.
        frame_offset: usize,
        /// Exact byte length of the complete candidate frame.
        frame_length: usize,
        /// Leading noise plus the complete candidate frame.
        consumed: usize,
    },
    /// A long-frame header declared a payload beyond the product bound.
    PayloadTooLarge {
        /// Leading noise plus one offending header byte, guaranteeing progress.
        consumed: usize,
    },
    /// The borrowed aggregate input exceeded the stream bound.
    StreamTooLarge {
        /// The complete oversized aggregate, which callers must discard.
        consumed: usize,
    },
}

impl Display for FrameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidFrame => "invalid VESC frame",
            Self::InvalidCrc => "invalid VESC frame CRC",
            Self::PayloadTooLarge => "VESC payload or stream exceeds its bound",
        })
    }
}

impl Error for FrameError {}

/// Calculate the VESC CRC-16 using polynomial `0x1021` and initial value zero.
#[must_use]
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0_u16;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
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

/// Encode one payload into the bounded VESC short or long frame form.
pub fn encode(payload: &[u8]) -> Result<Vec<u8>, FrameError> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(FrameError::PayloadTooLarge);
    }

    let header_bytes: usize = if payload.len() <= usize::from(u8::MAX) {
        2
    } else {
        3
    };
    let frame_bytes = header_bytes
        .checked_add(payload.len())
        .and_then(|length| length.checked_add(FRAME_TRAILER_BYTES))
        .ok_or(FrameError::PayloadTooLarge)?;
    let mut frame = Vec::with_capacity(frame_bytes);

    if header_bytes == 2 {
        frame.push(SHORT_FRAME_HEADER);
        frame.push(payload.len() as u8);
    } else {
        frame.push(LONG_FRAME_HEADER);
        frame.push((payload.len() >> 8) as u8);
        frame.push(payload.len() as u8);
    }
    frame.extend_from_slice(payload);
    let checksum = crc16(payload);
    frame.extend_from_slice(&checksum.to_be_bytes());
    frame.push(FRAME_FOOTER);
    Ok(frame)
}

/// Decode exactly one complete VESC frame, rejecting incomplete or trailing data.
pub fn decode(frame: &[u8]) -> Result<Vec<u8>, FrameError> {
    let (header_bytes, payload_bytes): (usize, usize) = match frame.first().copied() {
        Some(SHORT_FRAME_HEADER) if frame.len() >= 5 => (2, usize::from(frame[1])),
        Some(LONG_FRAME_HEADER) if frame.len() >= 6 => {
            (3, usize::from(u16::from_be_bytes([frame[1], frame[2]])))
        }
        Some(SHORT_FRAME_HEADER | LONG_FRAME_HEADER) | None => {
            return Err(FrameError::InvalidFrame);
        }
        Some(_) => return Err(FrameError::InvalidFrame),
    };

    if payload_bytes > MAX_PAYLOAD_BYTES {
        return Err(FrameError::PayloadTooLarge);
    }
    let expected_bytes = header_bytes
        .checked_add(payload_bytes)
        .and_then(|length| length.checked_add(FRAME_TRAILER_BYTES))
        .ok_or(FrameError::PayloadTooLarge)?;
    if frame.len() != expected_bytes || frame.last().copied() != Some(FRAME_FOOTER) {
        return Err(FrameError::InvalidFrame);
    }

    let payload_end = header_bytes + payload_bytes;
    let payload = &frame[header_bytes..payload_end];
    let received = u16::from_be_bytes([frame[payload_end], frame[payload_end + 1]]);
    if crc16(payload) != received {
        return Err(FrameError::InvalidCrc);
    }
    Ok(payload.to_vec())
}

/// Locate the next complete candidate frame in one borrowed stream region.
///
/// This operation is stateless: it retains no pointer or bytes. It validates
/// framing boundaries only; callers must pass every [`StreamScan::Frame`] to
/// [`decode`] for footer and CRC validation.
#[must_use]
pub fn scan_next(bytes: &[u8]) -> StreamScan {
    if bytes.len() > MAX_STREAM_BYTES {
        return StreamScan::StreamTooLarge {
            consumed: bytes.len(),
        };
    }

    let Some(frame_offset) = bytes
        .iter()
        .position(|byte| matches!(*byte, SHORT_FRAME_HEADER | LONG_FRAME_HEADER))
    else {
        return StreamScan::NeedMore {
            consumed: bytes.len(),
        };
    };
    let frame = &bytes[frame_offset..];
    let header_bytes = if frame[0] == SHORT_FRAME_HEADER { 2 } else { 3 };
    if frame.len() < header_bytes {
        return StreamScan::NeedMore {
            consumed: frame_offset,
        };
    }

    let payload_bytes = if header_bytes == 2 {
        usize::from(frame[1])
    } else {
        usize::from(u16::from_be_bytes([frame[1], frame[2]]))
    };
    if payload_bytes > MAX_PAYLOAD_BYTES {
        return StreamScan::PayloadTooLarge {
            consumed: frame_offset + 1,
        };
    }
    let Some(frame_length) = header_bytes
        .checked_add(payload_bytes)
        .and_then(|length| length.checked_add(FRAME_TRAILER_BYTES))
    else {
        return StreamScan::PayloadTooLarge {
            consumed: frame_offset + 1,
        };
    };
    if frame.len() < frame_length {
        return StreamScan::NeedMore {
            consumed: frame_offset,
        };
    }

    StreamScan::Frame {
        frame_offset,
        frame_length,
        consumed: frame_offset + frame_length,
    }
}

/// Reassemble bounded VESC frames from an arbitrarily fragmented byte stream.
#[derive(Debug, Default)]
pub struct StreamDecoder {
    buffer: Vec<u8>,
}

impl StreamDecoder {
    /// Construct an empty stream decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Append one fragment and return every complete payload now available.
    ///
    /// Aggregate overflow clears all retained bytes. A complete malformed frame
    /// is consumed before its error is returned, matching the shipped decoders.
    pub fn append(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        let Some(aggregate_bytes) = self.buffer.len().checked_add(bytes.len()) else {
            self.buffer.clear();
            return Err(FrameError::PayloadTooLarge);
        };
        if aggregate_bytes > MAX_STREAM_BYTES {
            self.buffer.clear();
            return Err(FrameError::PayloadTooLarge);
        }
        self.buffer.extend_from_slice(bytes);

        let mut payloads = Vec::new();
        loop {
            match scan_next(&self.buffer) {
                StreamScan::NeedMore { consumed } => {
                    self.buffer.drain(..consumed);
                    return Ok(payloads);
                }
                StreamScan::Frame {
                    frame_offset,
                    frame_length,
                    consumed,
                } => {
                    let result = decode(&self.buffer[frame_offset..frame_offset + frame_length]);
                    self.buffer.drain(..consumed);
                    payloads.push(result?);
                }
                StreamScan::PayloadTooLarge { consumed } => {
                    self.buffer.drain(..consumed);
                    return Err(FrameError::PayloadTooLarge);
                }
                StreamScan::StreamTooLarge { .. } => {
                    self.buffer.clear();
                    return Err(FrameError::PayloadTooLarge);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAMING_FIXTURE: &str = include_str!("../../../fixtures/vesc/framing.json");

    fn fixture_frame(name: &str) -> Vec<u8> {
        let name_marker = format!("\"name\": \"{name}\"");
        let case = FRAMING_FIXTURE
            .split_once(&name_marker)
            .unwrap_or_else(|| panic!("missing fixture case {name}"))
            .1;
        let value = case
            .split_once("\"frameHex\": \"")
            .unwrap_or_else(|| panic!("missing frameHex for {name}"))
            .1
            .split_once('"')
            .unwrap_or_else(|| panic!("unterminated frameHex for {name}"))
            .0;
        hex_bytes(value)
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        assert!(value.len().is_multiple_of(2), "hex must have byte pairs");
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).expect("fixture hex is ASCII");
                u8::from_str_radix(text, 16).expect("fixture contains valid hex")
            })
            .collect()
    }

    #[test]
    fn source_derived_short_and_long_vectors_round_trip() {
        let short_frame = fixture_frame("short-packet");
        let short_payload = hex_bytes("00060654484f5233303000");
        assert_eq!(decode(&short_frame), Ok(short_payload.clone()));
        assert_eq!(encode(&short_payload), Ok(short_frame));

        let long_frame = fixture_frame("long-packet-256-bytes");
        let long_payload = decode(&long_frame).expect("long fixture must decode");
        assert_eq!(long_payload.len(), 256);
        assert_eq!(long_payload[0], 4);
        assert!(long_payload[1..].iter().all(|byte| *byte == 0));
        assert_eq!(encode(&long_payload), Ok(long_frame));
    }

    #[test]
    fn crc_and_payload_boundaries_match_product_behavior() {
        assert_eq!(crc16(&[]), 0);
        assert_eq!(crc16(&[0]), 0);
        assert_eq!(encode(&[]), Ok(vec![2, 0, 0, 0, 3]));

        let short = encode(&vec![0x5a; 255]).expect("255-byte payload must encode");
        assert_eq!(&short[..2], &[2, 255]);
        assert_eq!(decode(&short), Ok(vec![0x5a; 255]));

        let long = encode(&vec![0xa5; 256]).expect("256-byte payload must encode");
        assert_eq!(&long[..3], &[3, 1, 0]);
        assert_eq!(decode(&long), Ok(vec![0xa5; 256]));

        let maximum = vec![0x33; MAX_PAYLOAD_BYTES];
        assert_eq!(
            decode(&encode(&maximum).expect("maximum payload must encode")),
            Ok(maximum)
        );
        assert_eq!(
            encode(&vec![0; MAX_PAYLOAD_BYTES + 1]),
            Err(FrameError::PayloadTooLarge)
        );
    }

    #[test]
    fn exact_decode_rejects_every_malformed_class() {
        let valid = fixture_frame("short-packet");
        assert_eq!(decode(&[]), Err(FrameError::InvalidFrame));
        assert_eq!(decode(&[4, 0, 0, 0, 3]), Err(FrameError::InvalidFrame));
        assert_eq!(decode(&[2, 0, 0, 0]), Err(FrameError::InvalidFrame));

        let mut trailing = valid.clone();
        trailing.push(0);
        assert_eq!(decode(&trailing), Err(FrameError::InvalidFrame));

        let mut invalid_footer = valid.clone();
        *invalid_footer.last_mut().expect("frame has footer") = 4;
        assert_eq!(decode(&invalid_footer), Err(FrameError::InvalidFrame));
        assert_eq!(
            decode(&fixture_frame("corrupt-crc")),
            Err(FrameError::InvalidCrc)
        );
        assert_eq!(decode(&[2, 2, 0, 0, 3]), Err(FrameError::InvalidFrame));
        assert_eq!(
            decode(&[3, 0x10, 0x01, 0, 0, 3]),
            Err(FrameError::PayloadTooLarge)
        );
    }

    #[test]
    fn stateless_scanner_reports_exact_boundaries_and_incomplete_suffixes() {
        let short = fixture_frame("short-packet");
        assert_eq!(scan_next(&[]), StreamScan::NeedMore { consumed: 0 });
        assert_eq!(scan_next(&[2]), StreamScan::NeedMore { consumed: 0 });
        assert_eq!(scan_next(&[3, 0]), StreamScan::NeedMore { consumed: 0 });
        assert_eq!(scan_next(&[0xff, 0]), StreamScan::NeedMore { consumed: 2 });
        assert_eq!(
            scan_next(&[0xff, 0, 3, 0]),
            StreamScan::NeedMore { consumed: 2 }
        );

        let noisy = [vec![0xff, 0], short.clone(), short.clone()].concat();
        assert_eq!(
            scan_next(&noisy),
            StreamScan::Frame {
                frame_offset: 2,
                frame_length: short.len(),
                consumed: 2 + short.len(),
            }
        );
        assert_eq!(
            scan_next(&short[..short.len() - 1]),
            StreamScan::NeedMore { consumed: 0 }
        );
    }

    #[test]
    fn stateless_scanner_reports_malformed_progress_and_bounds() {
        assert_eq!(
            scan_next(&[0xff, 3, 0x10, 0x01]),
            StreamScan::PayloadTooLarge { consumed: 2 }
        );

        let corrupt = fixture_frame("corrupt-crc");
        assert_eq!(
            scan_next(&corrupt),
            StreamScan::Frame {
                frame_offset: 0,
                frame_length: corrupt.len(),
                consumed: corrupt.len(),
            }
        );
        let oversized = vec![0; MAX_STREAM_BYTES + 1];
        assert_eq!(
            scan_next(&oversized),
            StreamScan::StreamTooLarge {
                consumed: oversized.len(),
            }
        );

        let maximum = encode(&vec![0x33; MAX_PAYLOAD_BYTES]).unwrap();
        assert_eq!(
            scan_next(&maximum),
            StreamScan::Frame {
                frame_offset: 0,
                frame_length: maximum.len(),
                consumed: maximum.len(),
            }
        );
    }

    #[test]
    fn stream_accepts_one_byte_fragments_split_headers_and_fixture_chunks() {
        let short = fixture_frame("short-packet");
        let expected_short = decode(&short).expect("short fixture must decode");
        let mut one_byte = StreamDecoder::new();
        for byte in &short[..short.len() - 1] {
            assert!(
                one_byte
                    .append(&[*byte])
                    .expect("fragment must be valid")
                    .is_empty()
            );
        }
        assert_eq!(
            one_byte.append(&short[short.len() - 1..]),
            Ok(vec![expected_short])
        );

        let long = fixture_frame("long-packet-256-bytes");
        let expected_long = decode(&long).expect("long fixture must decode");
        let mut split_header = StreamDecoder::new();
        assert_eq!(split_header.append(&long[..1]), Ok(Vec::new()));
        assert_eq!(split_header.append(&long[1..2]), Ok(Vec::new()));
        assert_eq!(split_header.append(&long[2..]), Ok(vec![expected_long]));

        let mut fixture_chunks = StreamDecoder::new();
        let mut offset = 0;
        for chunk_bytes in [1, 2, 17, 100, 142] {
            let payloads = fixture_chunks
                .append(&long[offset..offset + chunk_bytes])
                .expect("fixture chunk must be valid");
            offset += chunk_bytes;
            if offset == long.len() {
                assert_eq!(payloads, vec![decode(&long).unwrap()]);
            } else {
                assert!(payloads.is_empty());
            }
        }
        assert_eq!(offset, long.len());
    }

    #[test]
    fn stream_discards_noise_and_emits_multiple_frames_in_order() {
        let short = fixture_frame("short-packet");
        let long = fixture_frame("long-packet-256-bytes");
        let expected = vec![
            decode(&short).unwrap(),
            decode(&long).unwrap(),
            decode(&short).unwrap(),
        ];
        let bytes = [vec![0xff, 0, 1], short.clone(), long, short].concat();
        assert_eq!(StreamDecoder::new().append(&bytes), Ok(expected));
    }

    #[test]
    fn stream_consumes_bad_frames_and_recovers_retained_input() {
        let valid = fixture_frame("short-packet");
        let expected = decode(&valid).unwrap();

        let mut bad_crc = StreamDecoder::new();
        let bytes = [fixture_frame("corrupt-crc"), valid.clone()].concat();
        assert_eq!(bad_crc.append(&bytes), Err(FrameError::InvalidCrc));
        assert_eq!(bad_crc.append(&[]), Ok(vec![expected.clone()]));

        let mut invalid_footer = valid.clone();
        *invalid_footer.last_mut().unwrap() = 4;
        let mut bad_footer = StreamDecoder::new();
        assert_eq!(
            bad_footer.append(&[invalid_footer, valid.clone()].concat()),
            Err(FrameError::InvalidFrame)
        );
        assert_eq!(bad_footer.append(&[]), Ok(vec![expected.clone()]));

        let mut oversized = StreamDecoder::new();
        assert_eq!(
            oversized.append(&[3, 0x10, 0x01]),
            Err(FrameError::PayloadTooLarge)
        );
        assert_eq!(oversized.append(&valid), Ok(vec![expected]));
    }

    #[test]
    fn aggregate_overflow_clears_state_before_recovery() {
        let valid = fixture_frame("short-packet");
        let expected = decode(&valid).unwrap();
        let mut decoder = StreamDecoder::new();
        assert_eq!(decoder.append(&[3, 0]), Ok(Vec::new()));
        assert_eq!(
            decoder.append(&vec![0; MAX_STREAM_BYTES]),
            Err(FrameError::PayloadTooLarge)
        );
        assert_eq!(decoder.append(&valid), Ok(vec![expected]));
    }
}
