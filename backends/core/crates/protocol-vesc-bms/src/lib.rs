//! VESC BMS identity, values, routing, polling, and discovery owner.

use std::collections::{HashMap, VecDeque};

use domain_types::{CanId, FirmwareRecord};
use protocol_vesc::{ByteReader, ControllerRoute, ProtocolError, VescCommand, route_command};

pub const MAX_CELLS: usize = 50;
pub const MAX_TEMPERATURES: usize = 50;
pub const MAX_STATUS_BYTES: usize = 40;
pub const MAX_DISCOVERY_NODES: usize = 64;
pub const MAX_CANDIDATES: usize = 16;
pub const POLL_INTERVAL_MILLIS: u64 = 1_000;
pub const FAILURE_LIMIT: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BmsConnection {
    Direct,
    Can(CanId),
}

impl BmsConnection {
    #[must_use]
    pub const fn route(self) -> ControllerRoute {
        match self {
            Self::Direct => ControllerRoute::Direct,
            Self::Can(can_id) => ControllerRoute::Can(can_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BmsSnapshot {
    pub total_voltage_v: f64,
    pub charge_voltage_v: f64,
    pub input_current_a: f64,
    pub input_ic_current_a: f64,
    pub session_amp_hours_ah: f64,
    pub session_watt_hours_wh: f64,
    pub cell_voltages_v: Vec<f64>,
    pub balancing: Vec<bool>,
    pub temperatures_c: Vec<f64>,
    pub ic_temperature_c: f64,
    pub humidity_sensor_temperature_c: f64,
    pub humidity_percent: f64,
    pub highest_cell_temperature_c: f64,
    pub state_of_charge: f64,
    pub state_of_health: f64,
    pub reported_can_id: CanId,
    pub lifetime_charge_amp_hours_ah: f64,
    pub lifetime_charge_watt_hours_wh: f64,
    pub lifetime_discharge_amp_hours_ah: f64,
    pub lifetime_discharge_watt_hours_wh: f64,
    pub pressure_pa: f64,
    pub data_version: Option<u8>,
    pub status: Option<String>,
}

/// Fail-closed support for the two audited BMS firmware identities.
#[must_use]
pub fn supports_firmware(firmware: &FirmwareRecord) -> bool {
    (firmware.hardware_type == 1 && firmware.major == 6 && firmware.minor == 0)
        || (firmware.hardware_type == 2
            && firmware.major == 6
            && firmware.minor == 6
            && firmware.hardware_name.as_deref() == Some("VBMS32"))
}

#[must_use]
pub fn values_request(connection: BmsConnection) -> Vec<u8> {
    route_command(&[VescCommand::BmsGetValues as u8], connection.route())
}

/// Decodes the exact legacy or extended `COMM_BMS_GET_VALUES` response.
///
/// # Errors
///
/// Rejects the wrong command, truncated or trailing data, oversized arrays,
/// invalid flags/CAN routes/UTF-8, and non-finite auto-float values.
pub fn decode_values(
    payload: &[u8],
    expected_can_id: Option<CanId>,
) -> Result<BmsSnapshot, ProtocolError> {
    let mut reader = ByteReader::new(payload);
    let command = reader.u8()?;
    if command != VescCommand::BmsGetValues as u8 {
        return Err(ProtocolError::UnsupportedCommand(command));
    }

    let total_voltage_v = fixed_i32(&mut reader, 1_000_000.0)?;
    let charge_voltage_v = fixed_i32(&mut reader, 1_000_000.0)?;
    let input_current_a = fixed_i32(&mut reader, 1_000_000.0)?;
    let input_ic_current_a = fixed_i32(&mut reader, 1_000_000.0)?;
    let session_amp_hours_ah = fixed_i32(&mut reader, 1_000.0)?;
    let session_watt_hours_wh = fixed_i32(&mut reader, 1_000.0)?;

    let cell_count = usize::from(reader.u8()?);
    if cell_count > MAX_CELLS {
        return Err(ProtocolError::TruncatedPayload);
    }
    let mut cell_voltages_v = Vec::with_capacity(cell_count);
    for _ in 0..cell_count {
        cell_voltages_v.push(fixed_i16(&mut reader, 1_000.0)?);
    }
    let mut balancing = Vec::with_capacity(cell_count);
    for _ in 0..cell_count {
        balancing.push(match reader.u8()? {
            0 => false,
            1 => true,
            _ => return Err(ProtocolError::TruncatedPayload),
        });
    }

    let temperature_count = usize::from(reader.u8()?);
    if temperature_count > MAX_TEMPERATURES {
        return Err(ProtocolError::TruncatedPayload);
    }
    let mut temperatures_c = Vec::with_capacity(temperature_count);
    for _ in 0..temperature_count {
        temperatures_c.push(fixed_i16(&mut reader, 100.0)?);
    }
    let ic_temperature_c = fixed_i16(&mut reader, 100.0)?;
    let humidity_sensor_temperature_c = fixed_i16(&mut reader, 100.0)?;
    let humidity_percent = fixed_i16(&mut reader, 100.0)?;
    let highest_cell_temperature_c = fixed_i16(&mut reader, 100.0)?;
    let state_of_charge = fixed_i16(&mut reader, 1_000.0)?;
    let state_of_health = fixed_i16(&mut reader, 1_000.0)?;
    let reported_can_id = CanId::new(reader.u8()?).map_err(|_| ProtocolError::TruncatedPayload)?;
    if expected_can_id.is_some_and(|expected| expected != reported_can_id) {
        return Err(ProtocolError::TruncatedPayload);
    }
    let lifetime_charge_amp_hours_ah = float32_auto(&mut reader)?;
    let lifetime_charge_watt_hours_wh = float32_auto(&mut reader)?;
    let lifetime_discharge_amp_hours_ah = float32_auto(&mut reader)?;
    let lifetime_discharge_watt_hours_wh = float32_auto(&mut reader)?;
    let pressure_pa = fixed_i16(&mut reader, 0.1)?;

    let (data_version, status) = if reader.remaining() == 0 {
        (None, None)
    } else {
        let version = reader.u8()?;
        let status = reader.terminated_utf8(MAX_STATUS_BYTES)?;
        if reader.remaining() != 0 {
            return Err(ProtocolError::TruncatedPayload);
        }
        (Some(version), Some(status))
    };

    Ok(BmsSnapshot {
        total_voltage_v,
        charge_voltage_v,
        input_current_a,
        input_ic_current_a,
        session_amp_hours_ah,
        session_watt_hours_wh,
        cell_voltages_v,
        balancing,
        temperatures_c,
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

fn fixed_i16(reader: &mut ByteReader<'_>, scale: f64) -> Result<f64, ProtocolError> {
    Ok(f64::from(reader.i16()?) / scale)
}

fn fixed_i32(reader: &mut ByteReader<'_>, scale: f64) -> Result<f64, ProtocolError> {
    Ok(f64::from(reader.i32()?) / scale)
}

fn float32_auto(reader: &mut ByteReader<'_>) -> Result<f64, ProtocolError> {
    let raw = u32::from_be_bytes([reader.u8()?, reader.u8()?, reader.u8()?, reader.u8()?]);
    let exponent =
        i32::try_from((raw >> 23) & 0xff).map_err(|_| ProtocolError::TruncatedPayload)?;
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
        return Err(ProtocolError::TruncatedPayload);
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollDecision {
    ScheduleAfter(u64),
    MarkUnavailable,
}

#[derive(Debug, Default)]
pub struct BmsPollModel {
    consecutive_failures: u8,
}

impl BmsPollModel {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            consecutive_failures: 0,
        }
    }

    pub fn success(&mut self) -> PollDecision {
        self.consecutive_failures = 0;
        PollDecision::ScheduleAfter(POLL_INTERVAL_MILLIS)
    }

    pub fn failure(&mut self) -> PollDecision {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= FAILURE_LIMIT {
            PollDecision::MarkUnavailable
        } else {
            PollDecision::ScheduleAfter(POLL_INTERVAL_MILLIS)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BmsCandidate {
    pub connection: BmsConnection,
    pub hardware_name: Option<String>,
    pub vesc_version: String,
}

impl BmsCandidate {
    /// Produces only public discovery data. UUID and hardware CRC never cross
    /// this privacy boundary.
    #[must_use]
    pub fn from_firmware(connection: BmsConnection, firmware: &FirmwareRecord) -> Self {
        Self {
            connection,
            hardware_name: firmware.hardware_name.as_deref().map(bounded_display),
            vesc_version: format!("{}.{}", firmware.major, firmware.minor),
        }
    }
}

fn bounded_display(value: &str) -> String {
    value.chars().take(64).collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryStatus {
    Discovering,
    NotFound,
    Attached,
    Unavailable,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveryEffect {
    PingCan,
    ProbeFirmware(ControllerRoute),
    RequestValues(BmsConnection),
    None,
}

#[derive(Debug)]
pub struct AuxiliaryDiscovery {
    pub generation: u64,
    pub status: DiscoveryStatus,
    pub selected_connection: Option<BmsConnection>,
    pub active_probe: Option<CanId>,
    pub candidates: Vec<BmsCandidate>,
    preferred_can_id: Option<CanId>,
    pending: VecDeque<CanId>,
    discovered: HashMap<BmsConnection, FirmwareRecord>,
}

impl Default for AuxiliaryDiscovery {
    fn default() -> Self {
        Self {
            generation: 0,
            status: DiscoveryStatus::Discovering,
            selected_connection: None,
            active_probe: None,
            candidates: Vec::new(),
            preferred_can_id: None,
            pending: VecDeque::new(),
            discovered: HashMap::new(),
        }
    }
}

impl AuxiliaryDiscovery {
    pub fn begin(
        &mut self,
        local: &FirmwareRecord,
        reusable_inventory: Option<&[FirmwareRecord]>,
        preferred_can_id: Option<CanId>,
    ) -> DiscoveryEffect {
        self.reset();
        self.preferred_can_id = preferred_can_id;
        if supports_firmware(local) {
            self.discovered.insert(BmsConnection::Direct, local.clone());
            return self.select(BmsConnection::Direct);
        }
        if let Some(preferred) = preferred_can_id {
            if let Some(identity) = reusable_inventory
                .and_then(|items| items.iter().find(|item| item.can_id == Some(preferred)))
            {
                return if supports_firmware(identity) {
                    self.select(BmsConnection::Can(preferred))
                } else {
                    self.unavailable()
                };
            }
            return self.probe(preferred);
        }
        if let Some(inventory) = reusable_inventory {
            for identity in inventory.iter().filter(|item| supports_firmware(item)) {
                if let Some(can_id) = identity.can_id {
                    self.discovered
                        .insert(BmsConnection::Can(can_id), identity.clone());
                }
            }
            return self.finish();
        }
        DiscoveryEffect::PingCan
    }

    pub fn accept_can_ids(&mut self, raw_ids: &[u8], generation: u64) -> DiscoveryEffect {
        if generation != self.generation || self.preferred_can_id.is_some() {
            return DiscoveryEffect::None;
        }
        let mut ids: Vec<CanId> = raw_ids
            .iter()
            .filter_map(|value| CanId::new(*value).ok())
            .collect();
        ids.sort_by_key(|id| id.get());
        ids.dedup();
        if ids.len() > MAX_DISCOVERY_NODES {
            return self.unavailable();
        }
        self.pending = ids.into();
        self.next_probe()
    }

    pub fn accept_firmware(
        &mut self,
        firmware: FirmwareRecord,
        generation: u64,
    ) -> DiscoveryEffect {
        let Some(can_id) = firmware.can_id else {
            return DiscoveryEffect::None;
        };
        if generation != self.generation || self.active_probe != Some(can_id) {
            return DiscoveryEffect::None;
        }
        self.active_probe = None;
        if let Some(preferred) = self.preferred_can_id {
            return if preferred == can_id && supports_firmware(&firmware) {
                self.select(BmsConnection::Can(can_id))
            } else {
                self.unavailable()
            };
        }
        if supports_firmware(&firmware) {
            self.discovered.insert(BmsConnection::Can(can_id), firmware);
        }
        self.next_probe()
    }

    pub fn probe_timed_out(&mut self, can_id: CanId, generation: u64) -> DiscoveryEffect {
        if generation != self.generation || self.active_probe != Some(can_id) {
            return DiscoveryEffect::None;
        }
        self.active_probe = None;
        if self.preferred_can_id.is_some() {
            self.unavailable()
        } else {
            self.next_probe()
        }
    }

    pub fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.status = DiscoveryStatus::Discovering;
        self.selected_connection = None;
        self.active_probe = None;
        self.candidates.clear();
        self.preferred_can_id = None;
        self.pending.clear();
        self.discovered.clear();
    }

    fn probe(&mut self, can_id: CanId) -> DiscoveryEffect {
        self.active_probe = Some(can_id);
        DiscoveryEffect::ProbeFirmware(ControllerRoute::Can(can_id))
    }

    fn next_probe(&mut self) -> DiscoveryEffect {
        match self.pending.pop_front() {
            Some(can_id) => self.probe(can_id),
            None => self.finish(),
        }
    }

    fn finish(&mut self) -> DiscoveryEffect {
        let mut entries: Vec<_> = self.discovered.iter().collect();
        entries.sort_by_key(|(connection, _)| match connection {
            BmsConnection::Direct => -1,
            BmsConnection::Can(can_id) => i16::from(can_id.get()),
        });
        self.candidates = entries
            .iter()
            .take(MAX_CANDIDATES)
            .map(|(connection, firmware)| BmsCandidate::from_firmware(**connection, firmware))
            .collect();
        if entries.len() > MAX_CANDIDATES || entries.len() > 1 {
            self.status = DiscoveryStatus::Ambiguous;
            return DiscoveryEffect::None;
        }
        let Some((connection, _)) = entries.first() else {
            self.status = DiscoveryStatus::NotFound;
            return DiscoveryEffect::None;
        };
        self.select(**connection)
    }

    fn select(&mut self, connection: BmsConnection) -> DiscoveryEffect {
        self.selected_connection = Some(connection);
        self.status = DiscoveryStatus::Discovering;
        DiscoveryEffect::RequestValues(connection)
    }

    fn unavailable(&mut self) -> DiscoveryEffect {
        self.status = DiscoveryStatus::Unavailable;
        DiscoveryEffect::None
    }
}

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures {
    pub const SOURCE_DERIVED: &str =
        include_str!("../../../../fixtures/vesc-bms/source-derived.json");
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        cases: Vec<Case>,
        expected_values: ExpectedValues,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        name: String,
        kind: String,
        payload_hex: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedValues {
        total_voltage_v: f64,
        charge_voltage_v: f64,
        input_current_a: f64,
        input_ic_current_a: f64,
        session_amp_hours_ah: f64,
        session_watt_hours_wh: f64,
        cell_voltages_v: Vec<f64>,
        balancing: Vec<bool>,
        temperatures_c: Vec<f64>,
        ic_temperature_c: f64,
        humidity_sensor_temperature_c: f64,
        humidity_percent: f64,
        highest_cell_temperature_c: f64,
        state_of_charge: f64,
        state_of_health: f64,
        reported_can_id: u8,
        lifetime_charge_amp_hours_ah: f64,
        lifetime_charge_watt_hours_wh: f64,
        lifetime_discharge_amp_hours_ah: f64,
        lifetime_discharge_watt_hours_wh: f64,
        pressure_pa: f64,
        extended_data_version: u8,
        extended_status: String,
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

    fn fixture() -> Fixture {
        serde_json::from_str(fixtures::SOURCE_DERIVED).expect("source-derived fixture")
    }

    #[test]
    fn source_derived_firmware_identities_are_fail_closed() {
        let fixture = fixture();
        let firmware: Vec<_> = fixture
            .cases
            .iter()
            .filter(|case| case.kind == "firmware")
            .map(|case| protocol_vesc::decode_firmware(&hex(&case.payload_hex)).expect(&case.name))
            .collect();
        assert_eq!(firmware.len(), 2);
        assert!(firmware.iter().all(supports_firmware));
        let mut wrong = firmware[1].clone();
        wrong.minor = 7;
        assert!(!supports_firmware(&wrong));
        wrong = firmware[1].clone();
        wrong.hardware_name = Some("VBMS16".into());
        assert!(!supports_firmware(&wrong));
    }

    #[test]
    fn legacy_and_extended_values_match_source_derived_expectations() {
        let fixture = fixture();
        let values: Vec<_> = fixture
            .cases
            .iter()
            .filter(|case| case.kind == "values")
            .map(|case| {
                decode_values(&hex(&case.payload_hex), CanId::new(42).ok()).expect(&case.name)
            })
            .collect();
        assert_eq!(values.len(), 2);
        for value in &values {
            let expected = &fixture.expected_values;
            assert_eq!(value.total_voltage_v, expected.total_voltage_v);
            assert_eq!(value.charge_voltage_v, expected.charge_voltage_v);
            assert_eq!(value.input_current_a, expected.input_current_a);
            assert_eq!(value.input_ic_current_a, expected.input_ic_current_a);
            assert_eq!(value.session_amp_hours_ah, expected.session_amp_hours_ah);
            assert_eq!(value.session_watt_hours_wh, expected.session_watt_hours_wh);
            assert_eq!(value.cell_voltages_v, expected.cell_voltages_v);
            assert_eq!(value.balancing, expected.balancing);
            assert_eq!(value.temperatures_c, expected.temperatures_c);
            assert_eq!(value.ic_temperature_c, expected.ic_temperature_c);
            assert_eq!(
                value.humidity_sensor_temperature_c,
                expected.humidity_sensor_temperature_c
            );
            assert_eq!(value.humidity_percent, expected.humidity_percent);
            assert_eq!(
                value.highest_cell_temperature_c,
                expected.highest_cell_temperature_c
            );
            assert_eq!(value.state_of_charge, expected.state_of_charge);
            assert_eq!(value.state_of_health, expected.state_of_health);
            assert_eq!(value.reported_can_id.get(), expected.reported_can_id);
            assert_eq!(
                value.lifetime_charge_amp_hours_ah,
                expected.lifetime_charge_amp_hours_ah
            );
            assert_eq!(
                value.lifetime_charge_watt_hours_wh,
                expected.lifetime_charge_watt_hours_wh
            );
            assert_eq!(
                value.lifetime_discharge_amp_hours_ah,
                expected.lifetime_discharge_amp_hours_ah
            );
            assert_eq!(
                value.lifetime_discharge_watt_hours_wh,
                expected.lifetime_discharge_watt_hours_wh
            );
            assert_eq!(value.pressure_pa, expected.pressure_pa);
        }
        assert_eq!(values[0].data_version, None);
        assert_eq!(values[0].status, None);
        assert_eq!(
            values[1].data_version,
            Some(fixture.expected_values.extended_data_version)
        );
        assert_eq!(
            values[1].status.as_deref(),
            Some(fixture.expected_values.extended_status.as_str())
        );
    }

    #[test]
    fn malformed_values_routes_and_polling_are_bounded() {
        let fixture = fixture();
        let legacy = hex(&fixture
            .cases
            .iter()
            .find(|case| case.name == "vbms-values-legacy-typical")
            .expect("legacy")
            .payload_hex);
        for length in 0..legacy.len() {
            assert!(decode_values(&legacy[..length], None).is_err());
        }
        assert!(decode_values(&legacy, CanId::new(41).ok()).is_err());
        let mut trailing = legacy.clone();
        trailing.push(1);
        assert!(decode_values(&trailing, None).is_err());

        assert_eq!(values_request(BmsConnection::Direct), vec![96]);
        let can = CanId::new(42).expect("CAN");
        assert_eq!(values_request(BmsConnection::Can(can)), vec![34, 42, 96]);

        let mut poll = BmsPollModel::new();
        assert_eq!(poll.failure(), PollDecision::ScheduleAfter(1_000));
        assert_eq!(poll.failure(), PollDecision::ScheduleAfter(1_000));
        assert_eq!(poll.failure(), PollDecision::MarkUnavailable);
        assert_eq!(poll.success(), PollDecision::ScheduleAfter(1_000));
    }

    #[test]
    fn discovery_is_generation_owned_and_candidates_are_privacy_safe() {
        let fixture = fixture();
        let mut bms =
            protocol_vesc::decode_firmware(&hex(&fixture.cases[0].payload_hex)).expect("BMS");
        let can = CanId::new(42).expect("CAN");
        bms.can_id = Some(can);
        let local = FirmwareRecord {
            major: 6,
            minor: 6,
            test_version: 0,
            hardware_name: Some("controller".into()),
            firmware_name: None,
            hardware_type: 0,
            can_id: None,
            controller_uuid: Some([9; 12]),
            custom_configuration_count: 0,
            hardware_configuration_crc: Some(123),
        };
        let mut discovery = AuxiliaryDiscovery::default();
        assert_eq!(
            discovery.begin(&local, None, None),
            DiscoveryEffect::PingCan
        );
        let generation = discovery.generation;
        assert_eq!(
            discovery.accept_can_ids(&[42, 42, 254], generation),
            DiscoveryEffect::ProbeFirmware(ControllerRoute::Can(can))
        );
        assert_eq!(
            discovery.accept_firmware(bms.clone(), generation.wrapping_sub(1)),
            DiscoveryEffect::None
        );
        assert_eq!(
            discovery.accept_firmware(bms, generation),
            DiscoveryEffect::RequestValues(BmsConnection::Can(can))
        );
        assert_eq!(discovery.candidates.len(), 1);
        assert_eq!(
            discovery.candidates[0].hardware_name.as_deref(),
            Some("VBMS")
        );
    }
}
