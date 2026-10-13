//! Strict, bounded decoding of the controller data Float View consumes.
//!
//! This unselected safe candidate owns only the selected `COMM_GET_VALUES`
//! prefix, selective controller CAN ID, and `COMM_GET_MCCONF` current-limit subset. Native hosts retain
//! production request construction, routing, scheduling, and model mapping.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

/// VESC `COMM_GET_VALUES`.
pub const COMM_GET_VALUES: u8 = 4;
/// VESC `COMM_GET_MCCONF`.
pub const COMM_GET_MCCONF: u8 = 14;
/// VESC `COMM_GET_VALUES_SELECTIVE`, requesting only the controller's CAN ID
/// (field-mask bit 17, echoed as a big-endian u32 before the one-byte ID).
pub const CONTROLLER_CAN_ID_REQUEST: [u8; 5] = [50, 0, 2, 0, 0];
/// Bytes consumed from a complete standard realtime-values response.
pub const REALTIME_CONSUMED_BYTES: usize = 30;
/// Bytes consumed before optional motor-configuration fields.
pub const CURRENT_LIMITS_CONSUMED_BYTES: usize = 25;

const BATTERY_SERIES_CELL_OFFSETS: [(u32, usize); 5] = [
    (0x2E43_A161, 462), // 6.02
    (0x3F82_9CF7, 452), // 6.05
    (0x2EFD_0142, 458), // 6.06
    (0x57AD_8F53, 463), // 7.00
    (0x9307_410D, 463), // 7.01
];

/// Stable application-facing classification of the raw VESC fault byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultKind {
    /// VESC fault code zero.
    None,
    /// `FAULT_CODE_OVER_VOLTAGE`.
    OverVoltage,
    /// Any other raw fault code.
    Unknown,
}

impl FaultKind {
    const fn from_code(code: u8) -> Self {
        match code {
            0 => Self::None,
            1 => Self::OverVoltage,
            _ => Self::Unknown,
        }
    }
}

/// The fixed realtime subset currently consumed by Float View.
#[derive(Clone, Copy, PartialEq)]
pub struct RealtimeValues {
    pub controller_temp_c: f64,
    pub motor_temp_c: f64,
    pub motor_current_a: f64,
    pub battery_current_a: f64,
    pub duty_cycle: f64,
    pub erpm: i32,
    pub pack_voltage_v: f64,
    pub fault_code: u8,
    pub fault: FaultKind,
}

impl Debug for RealtimeValues {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("RealtimeValues([redacted])")
    }
}

/// The bounded current-limit subset currently consumed from motor config.
#[derive(Clone, Copy, PartialEq)]
pub struct CurrentLimits {
    pub motor_current_max_a: f64,
    pub motor_current_min_a: f64,
    pub battery_current_max_a: f64,
    pub battery_current_min_a: f64,
    pub battery_series_cells: Option<u8>,
}

impl Debug for CurrentLimits {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("CurrentLimits([redacted])")
    }
}

/// Closed failures matching the selected native decoders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerDataError {
    /// The payload contains a command other than the requested decoder owns.
    UnsupportedCommand(u8),
    /// The payload is truncated or contains an invalid consumed numeric value.
    TruncatedPayload,
}

impl Display for ControllerDataError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCommand(command) => {
                write!(formatter, "unsupported VESC command {command}")
            }
            Self::TruncatedPayload => formatter.write_str("truncated VESC controller data"),
        }
    }
}

impl Error for ControllerDataError {}

/// Decode the exact echoed field mask and controller ID. No telemetry is requested.
pub fn decode_controller_can_id(payload: &[u8]) -> Result<u8, ControllerDataError> {
    match payload {
        [50, 0, 2, 0, 0, can_id @ 0..=253] => Ok(*can_id),
        [command, ..] if *command != CONTROLLER_CAN_ID_REQUEST[0] => {
            Err(ControllerDataError::UnsupportedCommand(*command))
        }
        _ => Err(ControllerDataError::TruncatedPayload),
    }
}

/// Decode the selected prefix of one standard `COMM_GET_VALUES` response.
///
/// VESC appends fields after the fault byte. They remain deliberately opaque
/// because Float View does not consume them through this response.
pub fn decode_realtime_values(payload: &[u8]) -> Result<RealtimeValues, ControllerDataError> {
    let mut reader = ByteReader::new(payload);
    let command = reader.u8()?;
    if command != COMM_GET_VALUES {
        return Err(ControllerDataError::UnsupportedCommand(command));
    }

    let controller_temp_c = reader.i16_scaled(10.0)?;
    let motor_temp_c = reader.i16_scaled(10.0)?;
    let motor_current_a = reader.i32_scaled(100.0)?;
    let battery_current_a = reader.i32_scaled(100.0)?;
    reader.skip(8)?;
    let duty_cycle = reader.i16_scaled(1_000.0)?;
    let erpm = reader.i32()?;
    let pack_voltage_v = reader.i16_scaled(10.0)?;
    let fault_code = reader.u8()?;

    Ok(RealtimeValues {
        controller_temp_c,
        motor_temp_c,
        motor_current_a,
        battery_current_a,
        duty_cycle,
        erpm,
        pack_voltage_v,
        fault_code,
        fault: FaultKind::from_code(fault_code),
    })
}

/// Decode the current-limit subset of one `COMM_GET_MCCONF` response.
pub fn decode_current_limits(payload: &[u8]) -> Result<CurrentLimits, ControllerDataError> {
    let mut reader = ByteReader::new(payload);
    let command = reader.u8()?;
    if command != COMM_GET_MCCONF {
        return Err(ControllerDataError::UnsupportedCommand(command));
    }

    let signature = reader.u32()?;
    reader.skip(4)?;
    let limits = CurrentLimits {
        motor_current_max_a: reader.float32_auto()?,
        motor_current_min_a: reader.float32_auto()?,
        battery_current_max_a: reader.float32_auto()?,
        battery_current_min_a: reader.float32_auto()?,
        battery_series_cells: BATTERY_SERIES_CELL_OFFSETS
            .iter()
            .find_map(|(candidate, offset)| (*candidate == signature).then_some(*offset))
            .and_then(|offset| payload.get(offset).copied())
            .filter(|value| *value > 0),
    };

    let values = [
        limits.motor_current_max_a,
        limits.motor_current_min_a,
        limits.battery_current_max_a,
        limits.battery_current_min_a,
    ];
    if values
        .into_iter()
        .any(|value| !value.is_finite() || value.abs() <= 0.0)
    {
        return Err(ControllerDataError::TruncatedPayload);
    }

    Ok(limits)
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn u8(&mut self) -> Result<u8, ControllerDataError> {
        let value = self
            .bytes
            .get(self.offset)
            .copied()
            .ok_or(ControllerDataError::TruncatedPayload)?;
        self.offset += 1;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, ControllerDataError> {
        let mut value = 0_u32;
        for _ in 0..4 {
            value = (value << 8) | u32::from(self.u8()?);
        }
        Ok(value)
    }

    fn i16_scaled(&mut self, scale: f64) -> Result<f64, ControllerDataError> {
        let raw = i16::from_be_bytes([self.u8()?, self.u8()?]);
        Ok(f64::from(raw) / scale)
    }

    fn i32(&mut self) -> Result<i32, ControllerDataError> {
        Ok(i32::from_be_bytes([
            self.u8()?,
            self.u8()?,
            self.u8()?,
            self.u8()?,
        ]))
    }

    fn i32_scaled(&mut self, scale: f64) -> Result<f64, ControllerDataError> {
        Ok(f64::from(self.i32()?) / scale)
    }

    fn float32_auto(&mut self) -> Result<f64, ControllerDataError> {
        let raw = self.u32()?;
        let exponent_bits = ((raw >> 23) & 0xFF) as i32;
        let significand_bits = raw & 0x7F_FFFF;
        if exponent_bits == 0 && significand_bits == 0 {
            return Ok(0.0);
        }

        let significand = f64::from(significand_bits) / 16_777_216.0 + 0.5;
        let value = significand * 2_f64.powi(exponent_bits - 126);
        let signed = if raw & 0x8000_0000 == 0 {
            value
        } else {
            -value
        };
        if !signed.is_finite() || signed.abs() > f64::from(f32::MAX) {
            return Err(ControllerDataError::TruncatedPayload);
        }
        Ok(signed)
    }

    fn skip(&mut self, count: usize) -> Result<(), ControllerDataError> {
        self.offset = self
            .offset
            .checked_add(count)
            .filter(|offset| *offset <= self.bytes.len())
            .ok_or(ControllerDataError::TruncatedPayload)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NORMAL_FIXTURE: &str = include_str!("../../../fixtures/vesc/realtime-values.json");
    const OVER_VOLTAGE_FIXTURE: &str =
        include_str!("../../../fixtures/vesc/realtime-values-over-voltage.json");
    const LIMITS: [[u8; 4]; 4] = [
        0x42C8_0000_u32.to_be_bytes(),
        0xC2C8_0000_u32.to_be_bytes(),
        0x4248_0000_u32.to_be_bytes(),
        0xC1A0_0000_u32.to_be_bytes(),
    ];

    #[test]
    fn controller_id_read_is_exact_bounded_and_rejects_other_masks() {
        for can_id in 0..=253 {
            assert_eq!(
                decode_controller_can_id(&[50, 0, 2, 0, 0, can_id]),
                Ok(can_id)
            );
        }
        let response = [50, 0, 2, 0, 0, 42];
        for extent in 0..response.len() {
            assert!(decode_controller_can_id(&response[..extent]).is_err());
        }
        for payload in [
            vec![4, 0, 2, 0, 0, 42],
            vec![50, 0, 0, 0, 0, 42],
            vec![50, 0, 2, 0, 0, 254],
            vec![50, 0, 2, 0, 0, 255],
            vec![50, 0, 2, 0, 0, 42, 0],
        ] {
            assert!(decode_controller_can_id(&payload).is_err());
        }
    }

    #[test]
    fn committed_realtime_fixtures_match_native_values() {
        let normal = decode_realtime_values(&fixture_payload(NORMAL_FIXTURE)).unwrap();
        assert_eq!(normal.controller_temp_c, 25.0);
        assert_eq!(normal.motor_temp_c, 23.0);
        assert_eq!(normal.motor_current_a, 12.34);
        assert_eq!(normal.battery_current_a, -5.0);
        assert_eq!(normal.duty_cycle, 0.5);
        assert_eq!(normal.erpm, 10_000);
        assert_eq!(normal.pack_voltage_v, 51.6);
        assert_eq!(normal.fault_code, 0);
        assert_eq!(normal.fault, FaultKind::None);

        let faulted = decode_realtime_values(&fixture_payload(OVER_VOLTAGE_FIXTURE)).unwrap();
        assert_eq!(faulted.fault_code, 1);
        assert_eq!(faulted.fault, FaultKind::OverVoltage);
    }

    #[test]
    fn realtime_consumes_exact_prefix_and_permits_tail() {
        let complete = fixture_payload(NORMAL_FIXTURE);
        let expected = decode_realtime_values(&complete).unwrap();
        assert_eq!(
            decode_realtime_values(&complete[..REALTIME_CONSUMED_BYTES]),
            Ok(expected)
        );
        let mut tailed = complete[..REALTIME_CONSUMED_BYTES].to_vec();
        tailed.extend_from_slice(&[0xAA; 64]);
        assert_eq!(decode_realtime_values(&tailed), Ok(expected));
    }

    #[test]
    fn realtime_preserves_signed_boundaries_scaling_and_fault_classes() {
        let mut payload = fixture_payload(NORMAL_FIXTURE);
        payload[1..3].copy_from_slice(&i16::MIN.to_be_bytes());
        payload[3..5].copy_from_slice(&i16::MAX.to_be_bytes());
        payload[5..9].copy_from_slice(&i32::MIN.to_be_bytes());
        payload[9..13].copy_from_slice(&i32::MAX.to_be_bytes());
        payload[21..23].copy_from_slice(&i16::MIN.to_be_bytes());
        payload[23..27].copy_from_slice(&i32::MIN.to_be_bytes());
        payload[27..29].copy_from_slice(&i16::MAX.to_be_bytes());
        payload[29] = 255;
        let decoded = decode_realtime_values(&payload).unwrap();
        assert_eq!(decoded.controller_temp_c, f64::from(i16::MIN) / 10.0);
        assert_eq!(decoded.motor_temp_c, f64::from(i16::MAX) / 10.0);
        assert_eq!(decoded.motor_current_a, f64::from(i32::MIN) / 100.0);
        assert_eq!(decoded.battery_current_a, f64::from(i32::MAX) / 100.0);
        assert_eq!(decoded.duty_cycle, f64::from(i16::MIN) / 1_000.0);
        assert_eq!(decoded.erpm, i32::MIN);
        assert_eq!(decoded.pack_voltage_v, f64::from(i16::MAX) / 10.0);
        assert_eq!(decoded.fault, FaultKind::Unknown);
    }

    #[test]
    fn realtime_rejects_every_consumed_truncation_and_wrong_command() {
        let payload = fixture_payload(NORMAL_FIXTURE);
        for length in 0..REALTIME_CONSUMED_BYTES {
            assert_eq!(
                decode_realtime_values(&payload[..length]),
                Err(ControllerDataError::TruncatedPayload),
                "accepted length {length}"
            );
        }
        let mut wrong = payload;
        wrong[0] = COMM_GET_MCCONF;
        assert_eq!(
            decode_realtime_values(&wrong),
            Err(ControllerDataError::UnsupportedCommand(COMM_GET_MCCONF))
        );
    }

    #[test]
    fn every_supported_signature_uses_its_exact_cell_offset() {
        for (signature, offset) in BATTERY_SERIES_CELL_OFFSETS {
            let payload = motor_configuration(signature, Some((offset, 18)));
            assert_eq!(
                decode_current_limits(&payload),
                Ok(CurrentLimits {
                    motor_current_max_a: 100.0,
                    motor_current_min_a: -100.0,
                    battery_current_max_a: 50.0,
                    battery_current_min_a: -20.0,
                    battery_series_cells: Some(18),
                }),
                "signature {signature:08x} offset {offset}"
            );
        }
    }

    #[test]
    fn cell_count_is_optional_for_unknown_missing_and_zero_offsets() {
        let unknown = motor_configuration(0xDEAD_BEEF, Some((463, 18)));
        assert_eq!(
            decode_current_limits(&unknown)
                .unwrap()
                .battery_series_cells,
            None
        );

        let missing = motor_configuration(0x57AD_8F53, None);
        assert_eq!(missing.len(), CURRENT_LIMITS_CONSUMED_BYTES);
        assert_eq!(
            decode_current_limits(&missing)
                .unwrap()
                .battery_series_cells,
            None
        );

        let zero = motor_configuration(0x57AD_8F53, Some((463, 0)));
        assert_eq!(
            decode_current_limits(&zero).unwrap().battery_series_cells,
            None
        );
    }

    #[test]
    fn current_limits_preserve_auto_float_sign_and_reject_invalid_values() {
        let valid = motor_configuration(0x57AD_8F53, None);
        let decoded = decode_current_limits(&valid).unwrap();
        assert_eq!(decoded.motor_current_max_a, 100.0);
        assert_eq!(decoded.motor_current_min_a, -100.0);
        assert_eq!(decoded.battery_current_max_a, 50.0);
        assert_eq!(decoded.battery_current_min_a, -20.0);

        let mut zero = valid.clone();
        zero[9..13].copy_from_slice(&0_u32.to_be_bytes());
        assert_eq!(
            decode_current_limits(&zero),
            Err(ControllerDataError::TruncatedPayload)
        );

        let mut excessive = valid;
        excessive[9..13].copy_from_slice(&0x7FFF_FFFF_u32.to_be_bytes());
        assert_eq!(
            decode_current_limits(&excessive),
            Err(ControllerDataError::TruncatedPayload)
        );
    }

    #[test]
    fn current_limits_reject_every_consumed_truncation_and_wrong_command() {
        let payload = motor_configuration(0x57AD_8F53, None);
        for length in 0..CURRENT_LIMITS_CONSUMED_BYTES {
            assert_eq!(
                decode_current_limits(&payload[..length]),
                Err(ControllerDataError::TruncatedPayload),
                "accepted length {length}"
            );
        }
        let mut wrong = payload;
        wrong[0] = COMM_GET_VALUES;
        assert_eq!(
            decode_current_limits(&wrong),
            Err(ControllerDataError::UnsupportedCommand(COMM_GET_VALUES))
        );
    }

    #[test]
    fn output_and_diagnostics_are_deterministic_and_redacted() {
        let realtime_payload = fixture_payload(NORMAL_FIXTURE);
        let first = decode_realtime_values(&realtime_payload).unwrap();
        let second = decode_realtime_values(&realtime_payload).unwrap();
        assert_eq!(first, second);
        assert_eq!(format!("{first:?}"), "RealtimeValues([redacted])");

        let limits_payload = motor_configuration(0x57AD_8F53, Some((463, 18)));
        let limits = decode_current_limits(&limits_payload).unwrap();
        assert_eq!(format!("{limits:?}"), "CurrentLimits([redacted])");
        assert!(
            !ControllerDataError::TruncatedPayload
                .to_string()
                .contains("100")
        );
    }

    fn motor_configuration(signature: u32, cell: Option<(usize, u8)>) -> Vec<u8> {
        let length = cell
            .map_or(CURRENT_LIMITS_CONSUMED_BYTES, |(offset, _)| offset + 1)
            .max(CURRENT_LIMITS_CONSUMED_BYTES);
        let mut payload = vec![0_u8; length];
        payload[0] = COMM_GET_MCCONF;
        payload[1..5].copy_from_slice(&signature.to_be_bytes());
        for (index, value) in LIMITS.into_iter().enumerate() {
            let start = 9 + index * 4;
            payload[start..start + 4].copy_from_slice(&value);
        }
        if let Some((offset, value)) = cell {
            payload[offset] = value;
        }
        payload
    }

    fn fixture_payload(json: &str) -> Vec<u8> {
        let frame = hex_bytes(json_string(json, "frameHex"));
        let header = match frame.first().copied() {
            Some(2) => 2,
            Some(3) => 3,
            value => panic!("unexpected fixture frame header: {value:?}"),
        };
        let length = if header == 2 {
            usize::from(frame[1])
        } else {
            (usize::from(frame[1]) << 8) | usize::from(frame[2])
        };
        frame[header..header + length].to_vec()
    }

    fn json_string<'a>(json: &'a str, key: &str) -> &'a str {
        let prefix = format!("\"{key}\": \"");
        let start = json.find(&prefix).unwrap() + prefix.len();
        let suffix = &json[start..];
        &suffix[..suffix.find('"').unwrap()]
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }
}
