//! VESC framing, command, routing, and firmware identity owner.

mod bytes;
mod command;
mod firmware;
mod frame;

pub use bytes::{ByteReader, ByteWriter};
pub use command::{
    ControllerRoute, ExpectedResponse, VescCommand, response_key, route_command, shutdown_command,
};
pub use firmware::{
    AccessMode, Operation, decode_firmware, firmware_access, permits, supports_shutdown,
};
pub use frame::{FrameStreamDecoder, ProtocolError, crc16, decode_frame, encode_frame};

pub mod fixtures {
    pub const FRAMING: &str = include_str!("../../../../fixtures/vesc/framing.json");
    pub const FIRMWARE_606: &str = include_str!("../../../../fixtures/vesc/firmware-6.06.json");
    pub const FIRMWARE_EXPRESS_605: &str =
        include_str!("../../../../fixtures/vesc/firmware-express-6.05.json");
    pub const FIRMWARE_THOR_602: &str =
        include_str!("../../../../fixtures/vesc/firmware-thor300-6.02.json");
    pub const FIRMWARE_THOR_605: &str =
        include_str!("../../../../fixtures/vesc/firmware-thor300-6.05.json");
    pub const FIRMWARE_THOR_606: &str =
        include_str!("../../../../fixtures/vesc/firmware-thor300-6.06.json");
    pub const FIRMWARE_THOR_700: &str =
        include_str!("../../../../fixtures/vesc/firmware-thor300-7.00.json");
    pub const REALTIME_VALUES: &str =
        include_str!("../../../../fixtures/vesc/realtime-values.json");
    pub const CONTROLLER_SHUTDOWN: &str =
        include_str!("../../../../fixtures/vesc/controller-shutdown.json");
    pub const CONTROLLER_DISCOVERY: &str =
        include_str!("../../../../fixtures/connection/controller-discovery.tsv");
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FramingFixture {
        cases: Vec<FramingCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FramingCase {
        name: String,
        frame_hex: String,
        chunk_sizes: Vec<usize>,
        valid_crc: bool,
        command_id: u8,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FirmwareFixture {
        frame_hex: String,
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }

    #[test]
    fn canonical_framing_vectors_match_stream_and_crc_behavior() {
        let fixture: FramingFixture = serde_json::from_str(fixtures::FRAMING).unwrap();
        for case in fixture.cases {
            let frame = hex_bytes(&case.frame_hex);
            let mut decoder = FrameStreamDecoder::new();
            let mut offset = 0;
            let mut result = Ok(Vec::new());
            for count in case.chunk_sizes {
                result = decoder.append(&frame[offset..offset + count]);
                offset += count;
                if result.is_err() {
                    break;
                }
            }
            if case.valid_crc {
                let payloads = result.unwrap();
                assert_eq!(payloads.len(), 1, "{}", case.name);
                assert_eq!(payloads[0][0], case.command_id, "{}", case.name);
                assert_eq!(encode_frame(&payloads[0]).unwrap(), frame, "{}", case.name);
            } else {
                assert_eq!(result, Err(ProtocolError::InvalidCrc), "{}", case.name);
            }
        }
    }

    #[test]
    fn stream_discards_noise_handles_multiple_frames_and_retains_truncation() {
        let frame = encode_frame(&[VescCommand::GetFirmwareVersion as u8, 6, 6, 0]).unwrap();
        let mut combined = vec![0xff, 0x01];
        combined.extend_from_slice(&frame);
        combined.extend_from_slice(&frame);
        let mut decoder = FrameStreamDecoder::new();
        assert_eq!(decoder.append(&combined).unwrap().len(), 2);

        let mut truncated = FrameStreamDecoder::new();
        assert!(
            truncated
                .append(&frame[..frame.len() - 1])
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            truncated.append(&frame[frame.len() - 1..]).unwrap().len(),
            1
        );
    }

    #[test]
    fn malformed_lengths_footer_and_bounds_are_rejected() {
        assert_eq!(decode_frame(&[]), Err(ProtocolError::InvalidFrame));
        assert_eq!(
            decode_frame(&[2, 1, 0, 0, 4]),
            Err(ProtocolError::InvalidFrame)
        );
        assert_eq!(
            FrameStreamDecoder::new().append(&[3, 0x10, 0x01]),
            Err(ProtocolError::PayloadTooLarge)
        );
        assert_eq!(
            encode_frame(&vec![0; frame::MAX_PAYLOAD_BYTES + 1]),
            Err(ProtocolError::PayloadTooLarge)
        );
    }

    #[test]
    fn readers_writers_commands_routes_and_attribution_are_bounded() {
        let mut writer = ByteWriter::new(8);
        writer.put_u8(4).unwrap();
        writer.put_u16(0x1234).unwrap();
        writer.put_i32(-2).unwrap();
        assert_eq!(
            writer.put_bytes(&[1, 2]),
            Err(ProtocolError::PayloadTooLarge)
        );
        let bytes = writer.finish();
        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.u8().unwrap(), 4);
        assert_eq!(reader.u16().unwrap(), 0x1234);
        assert_eq!(reader.i32().unwrap(), -2);
        assert_eq!(reader.u8(), Err(ProtocolError::TruncatedPayload));

        let direct = route_command(&[4], ControllerRoute::Direct);
        assert_eq!(direct, vec![4]);
        let can = route_command(
            &[4],
            ControllerRoute::Can(float_view_domain::CanId::new(42).unwrap()),
        );
        assert_eq!(can, vec![34, 42, 4]);

        let expected = ExpectedResponse::new(VescCommand::CustomAppData, Some(31));
        assert_eq!(response_key(&[36, 101, 31, 0]), Some(expected));
        assert_ne!(response_key(&[36, 101, 32]), Some(expected));
        assert_eq!(response_key(&[]), None);
    }

    #[test]
    fn exact_firmware_catalog_and_strict_identity_tails_match_vectors() {
        let cases = [
            (fixtures::FIRMWARE_THOR_602, 6, 2, None),
            (fixtures::FIRMWARE_THOR_605, 6, 5, Some(0)),
            (fixtures::FIRMWARE_THOR_606, 6, 6, Some(0)),
            (fixtures::FIRMWARE_THOR_700, 7, 0, Some(0)),
        ];
        for (json, major, minor, crc) in cases {
            let fixture: FirmwareFixture = serde_json::from_str(json).unwrap();
            let payload = decode_frame(&hex_bytes(&fixture.frame_hex)).unwrap();
            let firmware = decode_firmware(&payload).unwrap();
            assert_eq!((firmware.major, firmware.minor), (major, minor));
            assert_eq!(firmware.hardware_name.as_deref(), Some("THOR300"));
            assert_eq!(firmware.hardware_type, 0);
            assert_eq!(firmware.controller_uuid, Some([0; 12]));
            assert_eq!(firmware.hardware_configuration_crc, crc);
            assert_eq!(firmware_access(&firmware), Some(AccessMode::ReadWrite));
        }

        let fixture: FirmwareFixture =
            serde_json::from_str(fixtures::FIRMWARE_EXPRESS_605).unwrap();
        let express =
            decode_firmware(&decode_frame(&hex_bytes(&fixture.frame_hex)).unwrap()).unwrap();
        assert_eq!(express.hardware_type, 2);
        assert_eq!(express.hardware_name.as_deref(), Some("VESC Express T"));
        assert_eq!(firmware_access(&express), None);

        let fixture: FirmwareFixture = serde_json::from_str(fixtures::FIRMWARE_THOR_606).unwrap();
        let mut test_build =
            decode_firmware(&decode_frame(&hex_bytes(&fixture.frame_hex)).unwrap()).unwrap();
        test_build.test_version = 1;
        assert_eq!(firmware_access(&test_build), Some(AccessMode::ReadOnly));
        assert!(!permits(Operation::Write, &test_build));
        assert!(permits(Operation::Read, &test_build));
        assert!(!supports_shutdown(&test_build));
    }

    #[test]
    fn malformed_firmware_identity_is_rejected() {
        assert_eq!(decode_firmware(&[]), Err(ProtocolError::TruncatedPayload));
        assert_eq!(
            decode_firmware(&[4]),
            Err(ProtocolError::UnsupportedCommand(4))
        );
        let mut unterminated = vec![0, 6, 6];
        unterminated.extend_from_slice(&[b'A'; 65]);
        assert_eq!(
            decode_firmware(&unterminated),
            Err(ProtocolError::TruncatedPayload)
        );
    }
}
