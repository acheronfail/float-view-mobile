//! Strict, bounded decoding for VESC `COMM_BMS_GET_VALUES` payloads.
//!
//! This component is an unselected candidate. Native hosts retain production
//! BMS values decoding until separate bridge and cutover changes are verified.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::str;

/// VESC command byte for `COMM_BMS_GET_VALUES`.
pub const COMM_BMS_GET_VALUES: u8 = 96;

/// Maximum accepted number of cell voltage and balancing entries.
pub const MAX_CELLS: usize = 50;

/// Maximum accepted number of BMS temperature entries.
pub const MAX_TEMPERATURES: usize = 50;

/// Maximum accepted UTF-8 byte length before the extended status terminator.
pub const MAX_STATUS_BYTES: usize = 40;

/// Structurally decoded command-96 values without application observation time.
///
/// Collections use fixed-capacity storage so untrusted counts cannot influence
/// allocation. Decoded values and status text are intentionally redacted by
/// [`Debug`].
#[derive(Clone, PartialEq)]
pub struct BmsValues<'a> {
    pub total_voltage_v: f64,
    pub charge_voltage_v: f64,
    pub input_current_a: f64,
    pub input_ic_current_a: f64,
    pub session_amp_hours_ah: f64,
    pub session_watt_hours_wh: f64,
    cell_voltages_v: [f64; MAX_CELLS],
    balancing: [bool; MAX_CELLS],
    cell_count: usize,
    temperatures_c: [f64; MAX_TEMPERATURES],
    temperature_count: usize,
    pub ic_temperature_c: f64,
    pub humidity_sensor_temperature_c: f64,
    pub humidity_percent: f64,
    pub highest_cell_temperature_c: f64,
    pub state_of_charge: f64,
    pub state_of_health: f64,
    pub reported_can_id: u8,
    pub lifetime_charge_amp_hours_ah: f64,
    pub lifetime_charge_watt_hours_wh: f64,
    pub lifetime_discharge_amp_hours_ah: f64,
    pub lifetime_discharge_watt_hours_wh: f64,
    pub pressure_pa: f64,
    pub data_version: Option<u8>,
    pub status: Option<&'a str>,
}

impl BmsValues<'_> {
    /// Decoded cell voltages, bounded by [`MAX_CELLS`].
    #[must_use]
    pub fn cell_voltages_v(&self) -> &[f64] {
        &self.cell_voltages_v[..self.cell_count]
    }

    /// Decoded balancing flags, one for each cell voltage.
    #[must_use]
    pub fn balancing(&self) -> &[bool] {
        &self.balancing[..self.cell_count]
    }

    /// Decoded BMS temperatures, bounded by [`MAX_TEMPERATURES`].
    #[must_use]
    pub fn temperatures_c(&self) -> &[f64] {
        &self.temperatures_c[..self.temperature_count]
    }
}

impl Debug for BmsValues<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BmsValues")
            .field("values", &"[redacted]")
            .field("cell_count", &self.cell_count)
            .field("temperature_count", &self.temperature_count)
            .field("data_version", &self.data_version)
            .field("status", &self.status.map(|_| "[redacted]"))
            .finish()
    }
}

/// Closed failures matching the selected native command-96 decoders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BmsValuesError {
    /// The payload contains a command other than `COMM_BMS_GET_VALUES`.
    UnsupportedCommand(u8),
    /// The payload is truncated or violates the strict bounded layout.
    TruncatedPayload,
}

impl Display for BmsValuesError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCommand(command) => {
                write!(formatter, "unsupported VESC command {command}")
            }
            Self::TruncatedPayload => formatter.write_str("truncated VESC BMS values"),
        }
    }
}

impl Error for BmsValuesError {}

/// Decode exactly one strict `COMM_BMS_GET_VALUES` payload.
///
/// `expected_can_id` applies route attribution performed by the native request
/// scheduler. Observation time and application-state mapping remain outside
/// this structural decoder.
pub fn decode(
    payload: &[u8],
    expected_can_id: Option<u8>,
) -> Result<BmsValues<'_>, BmsValuesError> {
    let mut reader = ByteReader::new(payload);
    let command = reader.u8()?;
    if command != COMM_BMS_GET_VALUES {
        return Err(BmsValuesError::UnsupportedCommand(command));
    }

    let total_voltage_v = reader.i32(1_000_000.0)?;
    let charge_voltage_v = reader.i32(1_000_000.0)?;
    let input_current_a = reader.i32(1_000_000.0)?;
    let input_ic_current_a = reader.i32(1_000_000.0)?;
    let session_amp_hours_ah = reader.i32(1_000.0)?;
    let session_watt_hours_wh = reader.i32(1_000.0)?;

    let cell_count = usize::from(reader.u8()?);
    if cell_count > MAX_CELLS {
        return Err(BmsValuesError::TruncatedPayload);
    }
    let mut cell_voltages_v = [0.0; MAX_CELLS];
    for value in &mut cell_voltages_v[..cell_count] {
        *value = reader.i16(1_000.0)?;
    }
    let mut balancing = [false; MAX_CELLS];
    for value in &mut balancing[..cell_count] {
        *value = match reader.u8()? {
            0 => false,
            1 => true,
            _ => return Err(BmsValuesError::TruncatedPayload),
        };
    }

    let temperature_count = usize::from(reader.u8()?);
    if temperature_count > MAX_TEMPERATURES {
        return Err(BmsValuesError::TruncatedPayload);
    }
    let mut temperatures_c = [0.0; MAX_TEMPERATURES];
    for value in &mut temperatures_c[..temperature_count] {
        *value = reader.i16(100.0)?;
    }

    let ic_temperature_c = reader.i16(100.0)?;
    let humidity_sensor_temperature_c = reader.i16(100.0)?;
    let humidity_percent = reader.i16(100.0)?;
    let highest_cell_temperature_c = reader.i16(100.0)?;
    let state_of_charge = reader.i16(1_000.0)?;
    let state_of_health = reader.i16(1_000.0)?;
    let reported_can_id = reader.u8()?;
    if reported_can_id > 253 || expected_can_id.is_some_and(|value| value != reported_can_id) {
        return Err(BmsValuesError::TruncatedPayload);
    }

    let lifetime_charge_amp_hours_ah = reader.float32_auto()?;
    let lifetime_charge_watt_hours_wh = reader.float32_auto()?;
    let lifetime_discharge_amp_hours_ah = reader.float32_auto()?;
    let lifetime_discharge_watt_hours_wh = reader.float32_auto()?;
    let pressure_pa = reader.i16(0.1)?;

    let (data_version, status) = if reader.remaining() == 0 {
        (None, None)
    } else {
        let version = reader.u8()?;
        let status = reader.terminated_utf8(MAX_STATUS_BYTES)?;
        if reader.remaining() != 0 {
            return Err(BmsValuesError::TruncatedPayload);
        }
        (Some(version), Some(status))
    };

    let fixed_values = [
        total_voltage_v,
        charge_voltage_v,
        input_current_a,
        input_ic_current_a,
        session_amp_hours_ah,
        session_watt_hours_wh,
        ic_temperature_c,
        humidity_sensor_temperature_c,
        humidity_percent,
        highest_cell_temperature_c,
        state_of_charge,
        state_of_health,
        lifetime_charge_amp_hours_ah,
        lifetime_charge_watt_hours_wh,
        lifetime_discharge_amp_hours_ah,
        lifetime_discharge_watt_hours_wh,
        pressure_pa,
    ];
    if fixed_values.into_iter().any(|value| !value.is_finite())
        || cell_voltages_v[..cell_count]
            .iter()
            .chain(&temperatures_c[..temperature_count])
            .any(|value| !value.is_finite())
    {
        return Err(BmsValuesError::TruncatedPayload);
    }

    Ok(BmsValues {
        total_voltage_v,
        charge_voltage_v,
        input_current_a,
        input_ic_current_a,
        session_amp_hours_ah,
        session_watt_hours_wh,
        cell_voltages_v,
        balancing,
        cell_count,
        temperatures_c,
        temperature_count,
        ic_temperature_c,
        humidity_sensor_temperature_c,
        humidity_percent,
        highest_cell_temperature_c,
        state_of_charge,
        state_of_health,
        reported_can_id,
        lifetime_charge_amp_hours_ah,
        lifetime_charge_watt_hours_wh,
        lifetime_discharge_amp_hours_ah,
        lifetime_discharge_watt_hours_wh,
        pressure_pa,
        data_version,
        status,
    })
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn u8(&mut self) -> Result<u8, BmsValuesError> {
        let value = self
            .bytes
            .get(self.offset)
            .copied()
            .ok_or(BmsValuesError::TruncatedPayload)?;
        self.offset += 1;
        Ok(value)
    }

    fn i16(&mut self, scale: f64) -> Result<f64, BmsValuesError> {
        let bytes = [self.u8()?, self.u8()?];
        Ok(f64::from(i16::from_be_bytes(bytes)) / scale)
    }

    fn i32(&mut self, scale: f64) -> Result<f64, BmsValuesError> {
        let bytes = [self.u8()?, self.u8()?, self.u8()?, self.u8()?];
        Ok(f64::from(i32::from_be_bytes(bytes)) / scale)
    }

    fn float32_auto(&mut self) -> Result<f64, BmsValuesError> {
        let raw = u32::from_be_bytes([self.u8()?, self.u8()?, self.u8()?, self.u8()?]);
        let exponent = i32::from(((raw >> 23) & 0xff) as u8);
        let significand_bits = raw & 0x7f_ffff;
        if exponent == 0 && significand_bits == 0 {
            return Ok(0.0);
        }
        let significand = f64::from(significand_bits) / 16_777_216.0 + 0.5;
        let magnitude = significand * 2_f64.powi(exponent - 126);
        let value = if raw & 0x8000_0000 == 0 {
            magnitude
        } else {
            -magnitude
        };
        if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
            return Err(BmsValuesError::TruncatedPayload);
        }
        Ok(value)
    }

    fn terminated_utf8(&mut self, max_content_bytes: usize) -> Result<&'a str, BmsValuesError> {
        let remainder = &self.bytes[self.offset..];
        let search_end = remainder.len().min(max_content_bytes + 1);
        let end = remainder[..search_end]
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(BmsValuesError::TruncatedPayload)?;
        let value =
            str::from_utf8(&remainder[..end]).map_err(|_| BmsValuesError::TruncatedPayload)?;
        self.offset += end + 1;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    const BMS_FIXTURES: &str = include_str!("../../../fixtures/vesc-bms/source-derived.json");
    const REPORTED_CAN_OFFSET_WITHOUT_VARIABLE_VALUES: usize = 39;
    const FIRST_AUTO_FLOAT_OFFSET_WITHOUT_VARIABLE_VALUES: usize = 40;

    #[test]
    fn decodes_source_derived_legacy_and_extended_values_exactly() {
        let legacy_payload = fixture_payload("vbms-values-legacy-typical");
        let legacy = decode(&legacy_payload, Some(42)).expect("audited legacy values");
        assert_typical_values(&legacy);
        assert_eq!(legacy.data_version, None);
        assert_eq!(legacy.status, None);

        let extended_payload = fixture_payload("vbms-values-extended-typical");
        let extended = decode(&extended_payload, Some(42)).expect("audited extended values");
        assert_typical_values(&extended);
        assert_eq!(extended.data_version, Some(1));
        assert_eq!(extended.status, Some("OK"));
    }

    fn assert_typical_values(values: &BmsValues<'_>) {
        assert_eq!(values.total_voltage_v, 50.4);
        assert_eq!(values.charge_voltage_v, 54.6);
        assert_eq!(values.input_current_a, -12.345);
        assert_eq!(values.input_ic_current_a, -12.3);
        assert_eq!(values.session_amp_hours_ah, 1.25);
        assert_eq!(values.session_watt_hours_wh, 62.5);
        assert_eq!(values.cell_voltages_v(), [4.2, 4.19, 4.18]);
        assert_eq!(values.balancing(), [true, false, true]);
        assert_eq!(values.temperatures_c(), [25.5, 26.25]);
        assert_eq!(values.ic_temperature_c, 27.5);
        assert_eq!(values.humidity_sensor_temperature_c, 24.75);
        assert_eq!(values.humidity_percent, 52.5);
        assert_eq!(values.highest_cell_temperature_c, 29.25);
        assert_eq!(values.state_of_charge, 0.875);
        assert_eq!(values.state_of_health, 0.95);
        assert_eq!(values.reported_can_id, 42);
        assert_eq!(values.lifetime_charge_amp_hours_ah, 64.0);
        assert_eq!(values.lifetime_charge_watt_hours_wh, 1_024.0);
        assert_eq!(values.lifetime_discharge_amp_hours_ah, 32.0);
        assert_eq!(values.lifetime_discharge_watt_hours_wh, 512.0);
        assert!((values.pressure_pa - 101_320.0).abs() < 0.001);
    }

    #[test]
    fn accepts_exact_zero_and_maximum_collection_and_status_bounds() {
        let minimum_payload = bounded_payload(0, 0, None);
        let minimum = decode(&minimum_payload, Some(42)).expect("minimum bounded values");
        assert!(minimum.cell_voltages_v().is_empty());
        assert!(minimum.balancing().is_empty());
        assert!(minimum.temperatures_c().is_empty());
        assert_eq!(minimum.status, None);

        let maximum_payload = bounded_payload(50, 50, Some((255, &[b'A'; MAX_STATUS_BYTES])));
        let maximum = decode(&maximum_payload, Some(42)).expect("maximum bounded values");
        assert_eq!(maximum.cell_voltages_v().len(), MAX_CELLS);
        assert_eq!(maximum.balancing().len(), MAX_CELLS);
        assert_eq!(maximum.temperatures_c().len(), MAX_TEMPERATURES);
        assert_eq!(maximum.data_version, Some(255));
        assert_eq!(maximum.status.map(str::len), Some(MAX_STATUS_BYTES));
        assert!(size_of::<BmsValues<'_>>() < 2_048);
    }

    #[test]
    fn rejects_every_truncation_and_bounded_structural_failure() {
        let legacy = fixture_payload("vbms-values-legacy-typical");
        let extended = fixture_payload("vbms-values-extended-typical");
        for length in 0..legacy.len() {
            assert_eq!(
                decode(&legacy[..length], Some(42)),
                Err(BmsValuesError::TruncatedPayload),
                "accepted legacy prefix length {length}"
            );
        }
        for length in 0..extended.len() {
            if length == legacy.len() {
                let legacy_boundary = decode(&extended[..length], Some(42))
                    .expect("extended fixture includes one exact legacy payload prefix");
                assert_eq!(legacy_boundary.data_version, None);
                assert_eq!(legacy_boundary.status, None);
            } else {
                assert_eq!(
                    decode(&extended[..length], Some(42)),
                    Err(BmsValuesError::TruncatedPayload),
                    "accepted incomplete extended prefix length {length}"
                );
            }
        }

        let mut excessive_cells = legacy.clone();
        excessive_cells[25] = 51;
        let mut excessive_temperatures = legacy.clone();
        excessive_temperatures[35] = 51;
        let mut invalid_balancing = legacy.clone();
        invalid_balancing[32] = 2;
        let mut invalid_reported_can = legacy.clone();
        invalid_reported_can[52] = 254;
        let mut overflowing_auto_float = legacy.clone();
        let legacy_auto_float_offset = first_auto_float_offset(3, 2);
        overflowing_auto_float[legacy_auto_float_offset..legacy_auto_float_offset + 4]
            .copy_from_slice(&0x7f80_0000_u32.to_be_bytes());

        for malformed in [
            excessive_cells,
            excessive_temperatures,
            invalid_balancing,
            invalid_reported_can,
            overflowing_auto_float,
        ] {
            assert_eq!(
                decode(&malformed, Some(42)),
                Err(BmsValuesError::TruncatedPayload)
            );
        }
        assert_eq!(
            decode(&legacy, Some(41)),
            Err(BmsValuesError::TruncatedPayload)
        );
        assert_eq!(decode(&[], None), Err(BmsValuesError::TruncatedPayload));
        assert_eq!(
            decode(&[0], None),
            Err(BmsValuesError::UnsupportedCommand(0))
        );
        assert_eq!(
            decode(&[255], None),
            Err(BmsValuesError::UnsupportedCommand(255))
        );
    }

    #[test]
    fn validates_auto_float_status_utf8_tail_and_remainder_boundaries() {
        let mut zero = bounded_payload(0, 0, None);
        let zero_auto_float_offset = first_auto_float_offset(0, 0);
        zero[zero_auto_float_offset..zero_auto_float_offset + 4]
            .copy_from_slice(&0_u32.to_be_bytes());
        assert_eq!(
            decode(&zero, Some(42))
                .expect("zero auto float")
                .lifetime_charge_amp_hours_ah,
            0.0
        );

        let mut negative = zero.clone();
        negative[zero_auto_float_offset..zero_auto_float_offset + 4]
            .copy_from_slice(&0xc280_0000_u32.to_be_bytes());
        assert_eq!(
            decode(&negative, Some(42))
                .expect("negative auto float")
                .lifetime_charge_amp_hours_ah,
            -64.0
        );

        let mut maximum_finite = zero.clone();
        maximum_finite[zero_auto_float_offset..zero_auto_float_offset + 4]
            .copy_from_slice(&0x7f7f_ffff_u32.to_be_bytes());
        assert_eq!(
            decode(&maximum_finite, Some(42))
                .expect("maximum finite auto float")
                .lifetime_charge_amp_hours_ah,
            f64::from(f32::MAX)
        );

        let legacy = bounded_payload(0, 0, None);
        let malformed_tails = [
            [legacy.as_slice(), &[1]].concat(),
            [legacy.as_slice(), &[1, 0xff, 0]].concat(),
            [legacy.as_slice(), &[1, b'O', 0, 0]].concat(),
            [legacy.as_slice(), &[1], &[b'A'; MAX_STATUS_BYTES + 1], &[0]].concat(),
        ];
        for malformed in malformed_tails {
            assert_eq!(
                decode(&malformed, Some(42)),
                Err(BmsValuesError::TruncatedPayload)
            );
        }
    }

    #[test]
    fn diagnostics_redact_values_collections_and_status() {
        let payload = bounded_payload(1, 1, Some((7, b"SECRET-STATUS")));
        let values = decode(&payload, Some(42)).expect("private values");
        let diagnostic = format!("{values:?}");
        assert!(!diagnostic.contains("SECRET"));
        assert!(!diagnostic.contains("42"));
        assert!(!diagnostic.contains("101320"));
        assert_eq!(diagnostic.matches("[redacted]").count(), 2);
        assert_eq!(
            BmsValuesError::UnsupportedCommand(37).to_string(),
            "unsupported VESC command 37"
        );
    }

    fn bounded_payload(
        cells: usize,
        temperatures: usize,
        extended: Option<(u8, &[u8])>,
    ) -> Vec<u8> {
        assert!(cells <= MAX_CELLS);
        assert!(temperatures <= MAX_TEMPERATURES);
        let mut payload = vec![COMM_BMS_GET_VALUES];
        for value in [
            50_400_000_i32,
            54_600_000,
            -12_345_000,
            -12_300_000,
            1_250,
            62_500,
        ] {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        payload.push(cells as u8);
        for index in 0..cells {
            payload.extend_from_slice(&(4_000_i16 + index as i16).to_be_bytes());
        }
        payload.extend((0..cells).map(|index| (index % 2) as u8));
        payload.push(temperatures as u8);
        for index in 0..temperatures {
            payload.extend_from_slice(&(2_000_i16 + index as i16).to_be_bytes());
        }
        for value in [2_750_i16, 2_475, 5_250, 2_925, 875, 950] {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        debug_assert_eq!(
            payload.len(),
            REPORTED_CAN_OFFSET_WITHOUT_VARIABLE_VALUES + 3 * cells + 2 * temperatures
        );
        payload.push(42);
        for raw in [0x4280_0000_u32, 0x4480_0000, 0x4200_0000, 0x4400_0000] {
            payload.extend_from_slice(&raw.to_be_bytes());
        }
        payload.extend_from_slice(&10_132_i16.to_be_bytes());
        if let Some((version, status)) = extended {
            payload.push(version);
            payload.extend_from_slice(status);
            payload.push(0);
        }
        payload
    }

    fn fixture_payload(case_name: &str) -> Vec<u8> {
        let marker = format!("\"name\": \"{case_name}\"");
        let case = BMS_FIXTURES
            .split_once(&marker)
            .map(|(_, suffix)| suffix)
            .expect("committed BMS values case");
        hex_bytes(json_string(case, "payloadHex"))
    }

    fn first_auto_float_offset(cells: usize, temperatures: usize) -> usize {
        FIRST_AUTO_FLOAT_OFFSET_WITHOUT_VARIABLE_VALUES + 3 * cells + 2 * temperatures
    }

    fn json_string<'a>(document: &'a str, field: &str) -> &'a str {
        let marker = format!("\"{field}\": \"");
        let value = document
            .split_once(&marker)
            .map(|(_, suffix)| suffix)
            .expect("committed string fixture field");
        value
            .split_once('"')
            .map(|(value, _)| value)
            .expect("terminated fixture string")
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        assert!(hex.len().is_multiple_of(2), "even fixture hex");
        (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).expect("fixture hex byte"))
            .collect()
    }
}
