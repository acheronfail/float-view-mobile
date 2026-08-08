//! Deterministic software platform, Bluetooth, and VESC adapters for core tests.

use std::collections::VecDeque;

use host_ports::{PlatformCommand, PlatformEvent};
use protocol_vesc::{VescCommand, decode_frame, encode_frame};

const DEVICE_ID: &str = "software-vesc";
const BMS_CAN_ID: u8 = 42;
const MAX_ADAPTER_EVENTS: usize = 64;

#[derive(Default)]
pub struct SoftwarePlatform {
    commands: VecDeque<PlatformCommand>,
    events: VecDeque<PlatformEvent>,
}

impl SoftwarePlatform {
    pub fn push_command(&mut self, command: PlatformCommand) {
        self.commands.push_back(command);
    }

    pub fn pop_command(&mut self) -> Option<PlatformCommand> {
        self.commands.pop_front()
    }

    pub fn push_event(&mut self, event: PlatformEvent) {
        self.events.push_back(event);
    }

    pub fn pop_event(&mut self) -> Option<PlatformEvent> {
        self.events.pop_front()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftwareConnectionState {
    Connecting,
    Ready,
    Disconnected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SoftwareBluetoothEvent {
    PermissionGranted,
    DeviceFound {
        device_id: &'static str,
        name: &'static str,
        rssi: i16,
    },
    ConnectionChanged {
        device_id: &'static str,
        state: SoftwareConnectionState,
    },
    Notification {
        device_id: &'static str,
        bytes: Vec<u8>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftwareAdapterError {
    UnknownDevice,
    NotConnected,
    QueueFull,
}

/// Queue-driven deterministic stand-in for host Bluetooth. It deliberately
/// models transport events only and does not impersonate a physical BLE test.
#[derive(Default)]
pub struct SoftwareBluetoothAdapter {
    peripheral: SoftwareVescPeripheral,
    events: VecDeque<SoftwareBluetoothEvent>,
    connected: bool,
}

impl SoftwareBluetoothAdapter {
    /// # Errors
    ///
    /// Returns `QueueFull` when the deterministic event bound is reached.
    pub fn request_permission(&mut self) -> Result<(), SoftwareAdapterError> {
        self.push(SoftwareBluetoothEvent::PermissionGranted)
    }

    /// # Errors
    ///
    /// Returns `QueueFull` when the deterministic event bound is reached.
    pub fn start_discovery(&mut self) -> Result<(), SoftwareAdapterError> {
        self.push(SoftwareBluetoothEvent::DeviceFound {
            device_id: DEVICE_ID,
            name: "Software VESC",
            rssi: -42,
        })
    }

    /// # Errors
    ///
    /// Rejects unknown devices and a full event queue.
    pub fn connect(&mut self, device_id: &str) -> Result<(), SoftwareAdapterError> {
        if device_id != DEVICE_ID {
            return Err(SoftwareAdapterError::UnknownDevice);
        }
        self.push(SoftwareBluetoothEvent::ConnectionChanged {
            device_id: DEVICE_ID,
            state: SoftwareConnectionState::Connecting,
        })?;
        self.connected = true;
        self.push(SoftwareBluetoothEvent::ConnectionChanged {
            device_id: DEVICE_ID,
            state: SoftwareConnectionState::Ready,
        })
    }

    /// # Errors
    ///
    /// Rejects unknown/disconnected devices and a full event queue.
    pub fn write(&mut self, device_id: &str, bytes: &[u8]) -> Result<(), SoftwareAdapterError> {
        if device_id != DEVICE_ID {
            return Err(SoftwareAdapterError::UnknownDevice);
        }
        if !self.connected {
            return Err(SoftwareAdapterError::NotConnected);
        }
        if let Some(bytes) = self.peripheral.response(bytes) {
            self.push(SoftwareBluetoothEvent::Notification {
                device_id: DEVICE_ID,
                bytes,
            })?;
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Rejects unknown devices and a full event queue.
    pub fn disconnect(&mut self, device_id: &str) -> Result<(), SoftwareAdapterError> {
        if device_id != DEVICE_ID {
            return Err(SoftwareAdapterError::UnknownDevice);
        }
        self.connected = false;
        self.push(SoftwareBluetoothEvent::ConnectionChanged {
            device_id: DEVICE_ID,
            state: SoftwareConnectionState::Disconnected,
        })
    }

    pub fn pop_event(&mut self) -> Option<SoftwareBluetoothEvent> {
        self.events.pop_front()
    }

    fn push(&mut self, event: SoftwareBluetoothEvent) -> Result<(), SoftwareAdapterError> {
        if self.events.len() >= MAX_ADAPTER_EVENTS {
            return Err(SoftwareAdapterError::QueueFull);
        }
        self.events.push_back(event);
        Ok(())
    }
}

/// Minimal deterministic direct controller used by Apple, Android, and Rust
/// software-host conformance. Request and response bytes match the retained
/// Swift/Kotlin peripheral implementations.
pub struct SoftwareVescPeripheral {
    current_configuration: Vec<u8>,
}

impl Default for SoftwareVescPeripheral {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareVescPeripheral {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_configuration: CURRENT_CONFIGURATION.to_vec(),
        }
    }

    #[must_use]
    pub fn response(&mut self, frame: &[u8]) -> Option<Vec<u8>> {
        let request = decode_frame(frame).ok()?;
        let payload = match request.as_slice() {
            [command] if *command == VescCommand::GetFirmwareVersion as u8 => FIRMWARE.to_vec(),
            [command] if *command == VescCommand::PingCan as u8 => {
                vec![VescCommand::PingCan as u8, BMS_CAN_ID]
            }
            [forward, can_id, command]
                if *forward == VescCommand::ForwardCan as u8
                    && *can_id == BMS_CAN_ID
                    && *command == VescCommand::GetFirmwareVersion as u8 =>
            {
                BMS_FIRMWARE.to_vec()
            }
            [forward, can_id, command]
                if *forward == VescCommand::ForwardCan as u8
                    && *can_id == BMS_CAN_ID
                    && *command == VescCommand::BmsGetValues as u8 =>
            {
                BMS_VALUES.to_vec()
            }
            [36, 101, 0, 2, 0] => vec![36, 101, 0, 10],
            [36, 101, 10, 4] => TELEMETRY.to_vec(),
            _ => self.configuration_response(&request)?,
        };
        encode_frame(&payload).ok()
    }

    fn configuration_response(&mut self, request: &[u8]) -> Option<Vec<u8>> {
        match request.first().copied()? {
            92 => {
                if request.len() != 10 || request[1] != 0 {
                    return None;
                }
                let length = read_i32(request, 2)?;
                let offset = read_i32(request, 6)?;
                if !(1..=400).contains(&length) || offset >= CONFIGURATION_SCHEMA.len() {
                    return None;
                }
                let end = CONFIGURATION_SCHEMA.len().min(offset.checked_add(length)?);
                let mut response = vec![92, 0];
                response.extend_from_slice(
                    &u32::try_from(CONFIGURATION_SCHEMA.len())
                        .ok()?
                        .to_be_bytes(),
                );
                response.extend_from_slice(&u32::try_from(offset).ok()?.to_be_bytes());
                response.extend_from_slice(&CONFIGURATION_SCHEMA[offset..end]);
                Some(response)
            }
            93 if request == [93, 0] => {
                let mut response = vec![93, 0];
                response.extend_from_slice(&self.current_configuration);
                Some(response)
            }
            94 if request == [94, 0] => {
                let mut response = vec![94, 0];
                response.extend_from_slice(DEFAULT_CONFIGURATION);
                Some(response)
            }
            95 if request.len() > 2
                && request[1] == 0
                && request[2..] == *ACCEPTED_CONFIGURATION_WRITE =>
            {
                self.current_configuration = EXPECTED_CONFIGURATION_READBACK.to_vec();
                Some(vec![95])
            }
            _ => None,
        }
    }
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<usize> {
    let value = u32::from_be_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
    ]);
    usize::try_from(value).ok()
}

const CONFIGURATION_SCHEMA: &[u8] =
    include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.0.0.qcompress");
const CURRENT_CONFIGURATION: &[u8] =
    include_bytes!("../../../../fixtures/refloat-configuration/values/1.0.0-current.bin");
const DEFAULT_CONFIGURATION: &[u8] =
    include_bytes!("../../../../fixtures/refloat-configuration/values/1.0.0-default.bin");
const ACCEPTED_CONFIGURATION_WRITE: &[u8] =
    include_bytes!("../../../../fixtures/refloat-configuration/values/1.0.0-write.bin");
const EXPECTED_CONFIGURATION_READBACK: &[u8] =
    include_bytes!("../../../../fixtures/refloat-configuration/values/1.0.0-readback.bin");

const FIRMWARE: &[u8] = &[
    0, 6, 6, b'T', b'H', b'O', b'R', b'3', b'0', b'0', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 0, 0, 0, b'V', b'E', b'S', b'C', 0,
];
const BMS_FIRMWARE: &[u8] = &[
    0x00, 0x06, 0x00, 0x56, 0x42, 0x4d, 0x53, 0x00, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
    0x88, 0x99, 0xaa, 0xbb, 0x00, 0x00, 0x01, 0x01,
];
const BMS_VALUES: &[u8] = &[
    0x60, 0x03, 0x01, 0x0b, 0x00, 0x03, 0x41, 0x21, 0x40, 0xff, 0x43, 0xa1, 0x58, 0xff, 0x44, 0x51,
    0x20, 0x00, 0x00, 0x04, 0xe2, 0x00, 0x00, 0xf4, 0x24, 0x03, 0x10, 0x68, 0x10, 0x5e, 0x10, 0x54,
    0x01, 0x00, 0x01, 0x02, 0x09, 0xf6, 0x0a, 0x41, 0x0a, 0xbe, 0x09, 0xab, 0x14, 0x82, 0x0b, 0x6d,
    0x03, 0x6b, 0x03, 0xb6, 0x2a, 0x42, 0x80, 0x00, 0x00, 0x44, 0x80, 0x00, 0x00, 0x42, 0x00, 0x00,
    0x00, 0x44, 0x00, 0x00, 0x00, 0x27, 0x94, 0x01, 0x4f, 0x4b, 0x00,
];

const TELEMETRY: &[u8] = &[
    36, 101, 10, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 0, 0, 0, 0, 0,
    0, 0, 0, 128, 0, 0, 0, 0, 0, 60, 56, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Immutable list of every retained protocol vector family compared by the
/// Rust protocol tests and this end-to-end software adapter test.
pub const PARITY_FIXTURES: &[(&str, &str)] = &[
    ("vesc-framing", protocol_vesc::fixtures::FRAMING),
    ("vesc-firmware-6.06", protocol_vesc::fixtures::FIRMWARE_606),
    (
        "vesc-firmware-express-6.05",
        protocol_vesc::fixtures::FIRMWARE_EXPRESS_605,
    ),
    (
        "vesc-firmware-thor-6.02",
        protocol_vesc::fixtures::FIRMWARE_THOR_602,
    ),
    (
        "vesc-firmware-thor-6.05",
        protocol_vesc::fixtures::FIRMWARE_THOR_605,
    ),
    (
        "vesc-firmware-thor-6.06",
        protocol_vesc::fixtures::FIRMWARE_THOR_606,
    ),
    (
        "vesc-firmware-thor-7.00",
        protocol_vesc::fixtures::FIRMWARE_THOR_700,
    ),
    (
        "vesc-realtime",
        include_str!("../../../../fixtures/vesc/realtime-values.json"),
    ),
    (
        "vesc-realtime-fault",
        include_str!("../../../../fixtures/vesc/realtime-values-over-voltage.json"),
    ),
    (
        "vesc-gatt",
        include_str!("../../../../fixtures/vesc/gatt-profile.json"),
    ),
    ("refloat-1.0", protocol_refloat::fixtures::PROTOCOL_100),
    ("refloat-1.1", protocol_refloat::fixtures::PROTOCOL_110),
    ("refloat-1.2.3", protocol_refloat::fixtures::PROTOCOL_123),
    (
        "refloat-catalog",
        protocol_refloat::fixtures::TELEMETRY_CATALOG,
    ),
    (
        "refloat-lifecycle",
        protocol_refloat::fixtures::TELEMETRY_LIFECYCLE,
    ),
    (
        "refloat-controls",
        protocol_refloat::fixtures::PRODUCT_CONTROLS,
    ),
    (
        "refloat-detailed",
        include_str!("../../../../fixtures/refloat/detailed-source-provenance.json"),
    ),
    (
        "refloat-ride-states",
        include_str!("../../../../fixtures/refloat/ride-states-1.2.3.json"),
    ),
    (
        "refloat-rtdata-1.2",
        include_str!("../../../../fixtures/refloat/rtdata-1.2.json"),
    ),
    (
        "refloat-android-hardware",
        include_str!("../../../../fixtures/refloat/rtdata-1.2.3-android-hardware.json"),
    ),
    (
        "refloat-config",
        protocol_refloat::fixtures::CONFIGURATION_MANIFEST,
    ),
    (
        "refloat-config-routes",
        include_str!("../../../../fixtures/refloat-configuration/routes.json"),
    ),
    (
        "refloat-config-adversarial",
        include_str!("../../../../fixtures/refloat-configuration/adversarial.json"),
    ),
    ("bms", protocol_vesc_bms::fixtures::SOURCE_DERIVED),
];

#[cfg(test)]
mod tests {
    use domain_types::CanId;
    use protocol_refloat::{
        ConfigurationValue, decode_configuration_values, decode_legacy, decode_qt_compressed,
        encode_configuration_values,
    };
    use protocol_vesc::{ControllerRoute, ProtocolError, decode_firmware, route_command};
    use protocol_vesc_bms::{BmsConnection, decode_values, values_request};

    use super::*;

    #[test]
    fn software_bluetooth_and_vesc_cover_the_shared_protocol_path() {
        let mut bluetooth = SoftwareBluetoothAdapter::default();
        bluetooth.request_permission().expect("permission");
        bluetooth.start_discovery().expect("discovery");
        bluetooth.connect(DEVICE_ID).expect("connect");
        assert_eq!(bluetooth.events.len(), 4);

        let firmware_request =
            encode_frame(&[VescCommand::GetFirmwareVersion as u8]).expect("frame");
        bluetooth
            .write(DEVICE_ID, &firmware_request)
            .expect("write");
        let notification = bluetooth.events.back().expect("notification");
        let SoftwareBluetoothEvent::Notification { bytes, .. } = notification else {
            panic!("expected notification");
        };
        let firmware = decode_firmware(&decode_frame(bytes).expect("frame")).expect("firmware");
        assert_eq!(firmware.hardware_name.as_deref(), Some("THOR300"));
        assert_eq!(firmware.custom_configuration_count, 1);
    }

    #[test]
    fn software_peripheral_outputs_match_swift_and_kotlin_vectors() {
        let mut peripheral = SoftwareVescPeripheral::new();
        let legacy_request = encode_frame(&[36, 101, 10, 4]).expect("request");
        let legacy = peripheral.response(&legacy_request).expect("response");
        let telemetry = decode_legacy(&decode_frame(&legacy).expect("frame")).expect("telemetry");
        assert_eq!(telemetry.ride_state, domain_types::RideState::Riding);

        let ping = peripheral
            .response(&encode_frame(&[VescCommand::PingCan as u8]).expect("request"))
            .expect("ping");
        assert_eq!(decode_frame(&ping).expect("frame"), [62, 42]);

        let can = CanId::new(42).expect("CAN");
        let bms_identity_request = encode_frame(&route_command(
            &[VescCommand::GetFirmwareVersion as u8],
            ControllerRoute::Can(can),
        ))
        .expect("request");
        let bms_identity = peripheral
            .response(&bms_identity_request)
            .expect("identity");
        let identity =
            decode_firmware(&decode_frame(&bms_identity).expect("frame")).expect("identity");
        assert!(protocol_vesc_bms::supports_firmware(&identity));

        let bms_request = encode_frame(&values_request(BmsConnection::Can(can))).expect("request");
        let bms = peripheral.response(&bms_request).expect("BMS values");
        let snapshot =
            decode_values(&decode_frame(&bms).expect("frame"), Some(can)).expect("values");
        assert_eq!(snapshot.status.as_deref(), Some("OK"));
    }

    #[test]
    fn software_configuration_schema_write_and_readback_are_stateful() {
        let mut peripheral = SoftwareVescPeripheral::new();
        let schema_request =
            encode_frame(&protocol_refloat::schema_request(400, 0)).expect("request");
        let schema_response =
            decode_frame(&peripheral.response(&schema_request).expect("response")).expect("frame");
        assert_eq!(&schema_response[..10], &[92, 0, 0, 0, 87, 226, 0, 0, 0, 0]);
        assert_eq!(&schema_response[10..], &CONFIGURATION_SCHEMA[..400]);

        let schema = decode_qt_compressed(CONFIGURATION_SCHEMA).expect("schema");
        let mut write_values =
            decode_configuration_values(ACCEPTED_CONFIGURATION_WRITE, &schema).expect("write");
        assert_eq!(
            write_values.get("kp"),
            Some(&ConfigurationValue::Double(21.0))
        );
        let encoded = encode_configuration_values(&write_values, &schema).expect("encoded");
        assert_eq!(encoded, ACCEPTED_CONFIGURATION_WRITE);

        let write_request =
            encode_frame(&protocol_refloat::write_values_request(&encoded)).expect("request");
        assert_eq!(
            decode_frame(&peripheral.response(&write_request).expect("ack")).expect("frame"),
            [95]
        );
        let current_request =
            encode_frame(&protocol_refloat::current_values_request()).expect("request");
        let current =
            decode_frame(&peripheral.response(&current_request).expect("current")).expect("frame");
        assert_eq!(&current[..2], &[93, 0]);
        assert_eq!(&current[2..], EXPECTED_CONFIGURATION_READBACK);

        write_values.insert("kp".into(), ConfigurationValue::Double(22.0));
        let rejected = encode_configuration_values(&write_values, &schema).expect("encoded");
        assert!(
            peripheral
                .response(
                    &encode_frame(&protocol_refloat::write_values_request(&rejected))
                        .expect("frame")
                )
                .is_none()
        );
    }

    #[test]
    fn parity_fixture_inventory_is_valid_json_and_uniquely_named() {
        let mut names = std::collections::HashSet::new();
        for (name, fixture) in PARITY_FIXTURES {
            assert!(names.insert(*name));
            let value: serde_json::Value = serde_json::from_str(fixture).expect(name);
            assert_eq!(value["schemaVersion"], 1);
            assert_wire_vectors(&value);
        }
    }

    fn assert_wire_vectors(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(frame_hex) = object.get("frameHex").and_then(|item| item.as_str()) {
                    let frame = hex(frame_hex);
                    if object.get("validCrc").and_then(serde_json::Value::as_bool) == Some(false) {
                        assert_eq!(decode_frame(&frame), Err(ProtocolError::InvalidCrc));
                    } else {
                        let payload = decode_frame(&frame).expect("fixture frame");
                        assert_eq!(encode_frame(&payload).expect("re-encoded fixture"), frame);
                    }
                }
                if let Some(payload_hex) = object.get("payloadHex").and_then(|item| item.as_str()) {
                    assert!(!hex(payload_hex).is_empty());
                }
                for nested in object.values() {
                    assert_wire_vectors(nested);
                }
            }
            serde_json::Value::Array(items) => {
                for nested in items {
                    assert_wire_vectors(nested);
                }
            }
            _ => {}
        }
    }

    fn hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("hex"), 16).expect("hex")
            })
            .collect()
    }

    #[test]
    fn adapter_rejects_wrong_devices_disconnected_writes_and_malformed_frames() {
        let mut bluetooth = SoftwareBluetoothAdapter::default();
        assert_eq!(
            bluetooth.connect("other"),
            Err(SoftwareAdapterError::UnknownDevice)
        );
        assert_eq!(
            bluetooth.write(DEVICE_ID, &[1, 2, 3]),
            Err(SoftwareAdapterError::NotConnected)
        );
        bluetooth.connect(DEVICE_ID).expect("connect");
        assert_eq!(bluetooth.write(DEVICE_ID, &[1, 2, 3]), Ok(()));
        assert_eq!(SoftwareVescPeripheral::new().response(&[1, 2, 3]), None);
        assert_eq!(decode_frame(&[1, 2, 3]), Err(ProtocolError::InvalidFrame));
    }
}
