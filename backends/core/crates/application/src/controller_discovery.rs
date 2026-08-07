use std::collections::{BTreeMap, BTreeSet};

use float_view_domain::{CanId, ControllerCandidate, FirmwareRecord};
use float_view_vesc::{ControllerRoute, firmware_access};

pub const MAXIMUM_CAN_IDS: usize = 64;
pub const MAXIMUM_CANDIDATES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionIssueCode {
    IdentificationTimeout,
    NoControllerFound,
    TooManyCanDevices,
    TooManyControllers,
    PreferredControllerUnavailable,
    ControllerChanged,
    UnsupportedController,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveryEffect {
    ProbeFirmware(ControllerRoute),
    PingCan,
    ClearHint,
    RequestRefloat {
        route: ControllerRoute,
        remember: Option<FirmwareRecord>,
    },
    Selecting,
    Failed(ConnectionIssueCode),
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoutingHint {
    pub can_id: CanId,
    pub controller_uuid: Option<[u8; 12]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Continuity {
    pub has_active_session: bool,
    pub route: Option<ControllerRoute>,
    pub controller_uuid: Option<[u8; 12]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeIntent {
    Preferred(CanId),
    Hint(CanId, [u8; 12]),
    Enumerating,
}

pub struct ControllerDiscovery {
    pub firmware: Option<FirmwareRecord>,
    pub transport_firmware: Option<FirmwareRecord>,
    pub controller_route: Option<ControllerRoute>,
    pub controller_candidates: Vec<ControllerCandidate>,
    pub classified_inventory: Vec<FirmwareRecord>,
    pub inventory_complete: bool,
    pub connection_issue: Option<ConnectionIssueCode>,
    pending_can_ids: Vec<CanId>,
    probed_controllers: BTreeMap<u8, FirmwareRecord>,
    probed_nodes: BTreeMap<u8, FirmwareRecord>,
    probe_intent: Option<ProbeIntent>,
    recovery_uuid: Option<[u8; 12]>,
    active_probe: Option<CanId>,
    continuity: Continuity,
}

impl Default for ControllerDiscovery {
    fn default() -> Self {
        Self {
            firmware: None,
            transport_firmware: None,
            controller_route: None,
            controller_candidates: Vec::new(),
            classified_inventory: Vec::new(),
            inventory_complete: false,
            connection_issue: None,
            pending_can_ids: Vec::new(),
            probed_controllers: BTreeMap::new(),
            probed_nodes: BTreeMap::new(),
            probe_intent: None,
            recovery_uuid: None,
            active_probe: None,
            continuity: Continuity {
                has_active_session: false,
                route: None,
                controller_uuid: None,
            },
        }
    }
}

impl ControllerDiscovery {
    #[must_use]
    pub fn begin(&self) -> DiscoveryEffect {
        DiscoveryEffect::ProbeFirmware(ControllerRoute::Direct)
    }

    pub fn accept_transport(
        &mut self,
        value: &FirmwareRecord,
        preferred: Option<CanId>,
        hint: Option<RoutingHint>,
    ) -> DiscoveryEffect {
        self.transport_firmware = Some(value.clone());
        self.classified_inventory = vec![value.clone()];
        if value.hardware_type == 0 {
            self.controller_route = Some(ControllerRoute::Direct);
            self.firmware = Some(value.clone());
            return if firmware_access(value).is_some() {
                DiscoveryEffect::RequestRefloat {
                    route: ControllerRoute::Direct,
                    remember: None,
                }
            } else {
                self.fail(ConnectionIssueCode::UnsupportedController)
            };
        }
        if let Some(preferred) = preferred {
            self.recovery_uuid = hint
                .filter(|routing| routing.can_id == preferred)
                .and_then(|routing| meaningful_uuid(routing.controller_uuid));
            self.probe_intent = Some(ProbeIntent::Preferred(preferred));
            return self.probe(preferred);
        }
        if let Some(hint) = hint
            && let Some(uuid) = meaningful_uuid(hint.controller_uuid)
        {
            self.recovery_uuid = Some(uuid);
            self.probe_intent = Some(ProbeIntent::Hint(hint.can_id, uuid));
            return self.probe(hint.can_id);
        }
        self.begin_enumeration()
    }

    pub fn accept_can_ids(&mut self, raw: &[u8], continuity: Continuity) -> DiscoveryEffect {
        self.continuity = continuity;
        if self.probe_intent != Some(ProbeIntent::Enumerating) {
            return DiscoveryEffect::None;
        }
        let normalized: BTreeSet<u8> = raw.iter().copied().filter(|value| *value <= 253).collect();
        if normalized.len() > MAXIMUM_CAN_IDS {
            return self.fail(ConnectionIssueCode::TooManyCanDevices);
        }
        self.pending_can_ids = normalized
            .into_iter()
            .filter_map(|value| CanId::new(value).ok())
            .collect();
        self.next_probe()
    }

    pub fn accept_can_firmware(
        &mut self,
        value: FirmwareRecord,
        continuity: Continuity,
    ) -> DiscoveryEffect {
        self.continuity = continuity;
        let Some(can_id) = value.can_id else {
            return DiscoveryEffect::None;
        };
        if self.active_probe != Some(can_id) {
            return DiscoveryEffect::None;
        }
        self.active_probe = None;
        match self.probe_intent {
            Some(ProbeIntent::Preferred(expected)) if expected == can_id => {
                if value.hardware_type != 0 {
                    return self.fail(ConnectionIssueCode::PreferredControllerUnavailable);
                }
                if self.recovery_uuid.is_some()
                    && meaningful_uuid(value.controller_uuid) != self.recovery_uuid
                {
                    return self.fail(ConnectionIssueCode::ControllerChanged);
                }
                self.choose(value)
            }
            Some(ProbeIntent::Hint(expected, uuid)) if expected == can_id => {
                if value.hardware_type == 0 && meaningful_uuid(value.controller_uuid) == Some(uuid)
                {
                    self.choose(value)
                } else {
                    self.begin_enumeration();
                    DiscoveryEffect::ClearHint
                }
            }
            Some(ProbeIntent::Enumerating) => {
                self.probed_nodes.insert(can_id.get(), value.clone());
                if value.hardware_type == 0 {
                    self.probed_controllers.insert(can_id.get(), value);
                    if self.probed_controllers.len() > MAXIMUM_CANDIDATES {
                        return self.fail(ConnectionIssueCode::TooManyControllers);
                    }
                }
                self.next_probe()
            }
            _ => DiscoveryEffect::None,
        }
    }

    #[must_use]
    pub fn clear_hint_then_continue(&self) -> DiscoveryEffect {
        if self.probe_intent == Some(ProbeIntent::Enumerating) {
            DiscoveryEffect::PingCan
        } else {
            DiscoveryEffect::None
        }
    }

    pub fn probe_timed_out(&mut self) -> DiscoveryEffect {
        self.active_probe = None;
        match self.probe_intent {
            Some(ProbeIntent::Preferred(_)) => {
                self.fail(ConnectionIssueCode::PreferredControllerUnavailable)
            }
            Some(ProbeIntent::Hint(_, _)) => {
                self.begin_enumeration();
                DiscoveryEffect::ClearHint
            }
            Some(ProbeIntent::Enumerating) => self.next_probe(),
            None => DiscoveryEffect::None,
        }
    }

    fn begin_enumeration(&mut self) -> DiscoveryEffect {
        self.probe_intent = Some(ProbeIntent::Enumerating);
        self.pending_can_ids.clear();
        self.probed_controllers.clear();
        self.probed_nodes.clear();
        self.active_probe = None;
        DiscoveryEffect::PingCan
    }

    fn probe(&mut self, can_id: CanId) -> DiscoveryEffect {
        self.active_probe = Some(can_id);
        DiscoveryEffect::ProbeFirmware(ControllerRoute::Can(can_id))
    }

    fn next_probe(&mut self) -> DiscoveryEffect {
        if self.pending_can_ids.is_empty() {
            return self.finish();
        }
        let can_id = self.pending_can_ids.remove(0);
        self.probe(can_id)
    }

    fn finish(&mut self) -> DiscoveryEffect {
        self.classified_inventory = self
            .transport_firmware
            .iter()
            .cloned()
            .chain(self.probed_nodes.values().cloned())
            .collect();
        self.inventory_complete = true;
        let records: Vec<FirmwareRecord> = self.probed_controllers.values().cloned().collect();
        self.controller_candidates = records
            .iter()
            .filter_map(|record| {
                Some(ControllerCandidate {
                    can_id: record.can_id?,
                    hardware_name: record.hardware_name.as_ref().map(|value| bounded(value)),
                    firmware_name: record.firmware_name.as_ref().map(|value| bounded(value)),
                    vesc_version: format!("{}.{:02}", record.major, record.minor),
                    vesc_supported: firmware_access(record).is_some(),
                })
            })
            .collect();
        if records.is_empty() {
            return self.fail(if self.continuity.has_active_session {
                ConnectionIssueCode::ControllerChanged
            } else {
                ConnectionIssueCode::NoControllerFound
            });
        }
        if let Some(expected) = self.recovery_uuid
            && let Some(record) = records
                .iter()
                .find(|record| meaningful_uuid(record.controller_uuid) == Some(expected))
        {
            return self.choose(record.clone());
        }
        if self.continuity.has_active_session {
            if self.continuity.controller_uuid.is_some() {
                return self.fail(ConnectionIssueCode::ControllerChanged);
            }
            if let Some(ControllerRoute::Can(previous)) = self.continuity.route
                && let Some(record) = records.iter().find(|record| {
                    record.can_id == Some(previous) && firmware_access(record).is_some()
                })
            {
                return self.choose(record.clone());
            }
            return self.fail(ConnectionIssueCode::ControllerChanged);
        }
        if records.len() == 1 {
            return self.choose(records[0].clone());
        }
        DiscoveryEffect::Selecting
    }

    fn choose(&mut self, value: FirmwareRecord) -> DiscoveryEffect {
        let Some(can_id) = value.can_id else {
            return DiscoveryEffect::None;
        };
        let route = ControllerRoute::Can(can_id);
        self.controller_route = Some(route);
        self.firmware = Some(value.clone());
        self.controller_candidates.clear();
        if firmware_access(&value).is_none() {
            return self.fail(ConnectionIssueCode::UnsupportedController);
        }
        DiscoveryEffect::RequestRefloat {
            route,
            remember: Some(value),
        }
    }

    fn fail(&mut self, issue: ConnectionIssueCode) -> DiscoveryEffect {
        self.connection_issue = Some(issue);
        DiscoveryEffect::Failed(issue)
    }
}

#[must_use]
pub fn meaningful_uuid(value: Option<[u8; 12]>) -> Option<[u8; 12]> {
    value.filter(|bytes| {
        bytes.iter().any(|byte| *byte != 0) && bytes.iter().any(|byte| *byte != u8::MAX)
    })
}

#[must_use]
pub fn preferred_can_id(values_json: &str, key: &str) -> Option<CanId> {
    let value: serde_json::Value = serde_json::from_str(values_json).ok()?;
    let number = value.get(key)?.as_u64()?;
    CanId::new(u8::try_from(number).ok()?).ok()
}

fn bounded(value: &str) -> String {
    value.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTORS: &str = include_str!("../../../../fixtures/connection/controller-discovery.tsv");

    fn firmware(
        hardware_type: u8,
        can_id: Option<u8>,
        uuid: Option<u8>,
        minor: u8,
    ) -> FirmwareRecord {
        FirmwareRecord {
            major: 6,
            minor,
            test_version: 0,
            hardware_name: Some(
                if hardware_type == 0 {
                    "THOR300"
                } else {
                    "Gateway"
                }
                .into(),
            ),
            firmware_name: None,
            hardware_type,
            can_id: can_id.map(|value| CanId::new(value).unwrap()),
            controller_uuid: uuid.filter(|value| *value != 0).map(|value| [value; 12]),
            custom_configuration_count: 0,
            hardware_configuration_crc: None,
        }
    }

    fn values(text: &str) -> Vec<u8> {
        if text.is_empty() {
            Vec::new()
        } else {
            text.split(',')
                .map(|value| value.parse().unwrap())
                .collect()
        }
    }

    fn optional(text: &str) -> Option<u8> {
        (!text.is_empty()).then(|| text.parse().unwrap())
    }

    fn stable_effect(effect: &DiscoveryEffect) -> &'static str {
        match effect {
            DiscoveryEffect::RequestRefloat { .. } => "request_refloat",
            DiscoveryEffect::Selecting => "selecting",
            DiscoveryEffect::Failed(_) => "failed",
            DiscoveryEffect::ProbeFirmware(_) => "probe",
            DiscoveryEffect::PingCan => "ping",
            DiscoveryEffect::ClearHint => "clear_hint",
            DiscoveryEffect::None => "none",
        }
    }

    fn stable_issue(issue: Option<ConnectionIssueCode>) -> &'static str {
        match issue {
            Some(ConnectionIssueCode::TooManyCanDevices) => "too_many_can_devices",
            Some(ConnectionIssueCode::ControllerChanged) => "controller_changed",
            Some(ConnectionIssueCode::NoControllerFound) => "no_controller_found",
            Some(ConnectionIssueCode::TooManyControllers) => "too_many_controllers",
            Some(ConnectionIssueCode::PreferredControllerUnavailable) => {
                "preferred_controller_unavailable"
            }
            Some(ConnectionIssueCode::UnsupportedController) => "unsupported_controller",
            Some(ConnectionIssueCode::IdentificationTimeout) => "identification_timeout",
            None => "",
        }
    }

    #[test]
    fn shared_discovery_vectors_match_exact_routes_candidates_and_failures() {
        for line in VECTORS
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let columns: Vec<&str> = line.split('|').collect();
            let transport_type = columns[1].parse().unwrap();
            let preferred = optional(columns[2]).map(|value| CanId::new(value).unwrap());
            let hint_id = optional(columns[3]);
            let hint_uuid = optional(columns[4]);
            let hint = hint_id.map(|value| RoutingHint {
                can_id: CanId::new(value).unwrap(),
                controller_uuid: hint_uuid.filter(|byte| *byte != 0).map(|byte| [byte; 12]),
            });
            let continuity = Continuity {
                has_active_session: !columns[7].is_empty(),
                route: optional(columns[7])
                    .map(|value| ControllerRoute::Can(CanId::new(value).unwrap())),
                controller_uuid: optional(columns[8])
                    .filter(|byte| *byte != 0)
                    .map(|byte| [byte; 12]),
            };
            let controllers: BTreeMap<u8, FirmwareRecord> = if columns[6].is_empty() {
                BTreeMap::new()
            } else {
                columns[6]
                    .split(';')
                    .map(|encoded| {
                        let fields: Vec<u8> = encoded
                            .split(':')
                            .map(|value| value.parse().unwrap())
                            .collect();
                        (
                            fields[0],
                            firmware(fields[1], Some(fields[0]), Some(fields[2]), fields[3]),
                        )
                    })
                    .collect()
            };
            let mut discovery = ControllerDiscovery::default();
            let transport = firmware(transport_type, None, None, 6);
            let mut effect = discovery.accept_transport(&transport, preferred, hint);
            for _ in 0..128 {
                effect = match effect {
                    DiscoveryEffect::ProbeFirmware(ControllerRoute::Can(can_id)) => {
                        if let Some(record) = controllers.get(&can_id.get()) {
                            discovery.accept_can_firmware(record.clone(), continuity)
                        } else {
                            discovery.probe_timed_out()
                        }
                    }
                    DiscoveryEffect::PingCan => {
                        discovery.accept_can_ids(&values(columns[5]), continuity)
                    }
                    DiscoveryEffect::ClearHint => discovery.clear_hint_then_continue(),
                    _ => break,
                };
            }
            assert_eq!(stable_effect(&effect), columns[9], "{}", columns[0]);
            let route = match discovery.controller_route {
                Some(ControllerRoute::Direct) => "direct".to_owned(),
                Some(ControllerRoute::Can(can_id)) => format!("can:{}", can_id.get()),
                None => String::new(),
            };
            assert_eq!(route, columns[10], "{}", columns[0]);
            let candidates = discovery
                .controller_candidates
                .iter()
                .map(|value| value.can_id.get().to_string())
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(candidates, columns[11], "{}", columns[0]);
            assert_eq!(
                stable_issue(discovery.connection_issue),
                columns[12],
                "{}",
                columns[0]
            );
        }
    }

    #[test]
    fn preferred_ids_and_private_identity_are_strict() {
        assert_eq!(
            preferred_can_id(r#"{"preferredCanId":42}"#, "preferredCanId")
                .unwrap()
                .get(),
            42
        );
        assert_eq!(
            preferred_can_id(r#"{"preferredCanId":254}"#, "preferredCanId"),
            None
        );
        assert_eq!(
            preferred_can_id(r#"{"preferredCanId":1.5}"#, "preferredCanId"),
            None
        );
        assert_eq!(meaningful_uuid(Some([0; 12])), None);
        assert_eq!(meaningful_uuid(Some([u8::MAX; 12])), None);
        assert_eq!(meaningful_uuid(Some([42; 12])), Some([42; 12]));
    }
}
