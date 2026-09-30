//! Closed OS observations and shared permission recovery policy.
use crate::{BackendService, os::Permission};
use serde::{Deserialize, Serialize};

macro_rules! enumeration {
    ($name:ident { $($value:ident),+ }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub enum $name { $($value),+ }
    };
}
enumeration!(Platform {
    Ios,
    Macos,
    Android29,
    Android30,
    Android31,
    Linux
});
enumeration!(Id {
    Bluetooth,
    Location,
    BackgroundLocation
});
enumeration!(Action {
    RequestPermission,
    OpenSettings
});
enumeration!(Status {
    NotDetermined,
    Granted,
    Limited,
    Denied,
    Restricted,
    Unavailable
});
enumeration!(Accuracy {
    Unknown,
    Approximate,
    Precise
});
enumeration!(Scope {
    Unknown,
    WhileInUse,
    Always
});
enumeration!(Guidance {
    None,
    Checking,
    Restricted,
    ServicesDisabled,
    UnsupportedHardware,
    ForegroundRequired,
    Rationale,
    AppSettings,
    IosSettings,
    MacBluetooth,
    MacLocation,
    BackgroundSettings,
    AlwaysAccess,
    FineLocationForScan
});

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub access: Permission,
    pub requestable: bool,
    pub rationale: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub platform: Platform,
    pub bluetooth_scan: Grant,
    pub bluetooth_connect: Grant,
    pub location: Grant,
    pub background_location: Grant,
    pub accuracy: Accuracy,
    pub scope: Scope,
    pub location_services: Option<bool>,
    pub bluetooth_service: Option<bool>,
    pub bluetooth_supported: Option<bool>,
    pub background_option_label: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Row {
    pub permission: Id,
    pub status: Status,
    pub action: Option<Action>,
    pub guidance: Guidance,
    pub pending: bool,
    pub optional: bool,
    pub services_enabled: Option<bool>,
    pub scan: Option<Permission>,
    pub connect: Option<Permission>,
    pub accuracy: Accuracy,
    pub scope: Scope,
    pub option_label: Option<String>,
}
fn granted(p: Permission) -> bool {
    matches!(p, Permission::Granted | Permission::NotRequired)
}
fn status(g: &Grant) -> Status {
    match g.access {
        Permission::Unknown => Status::Unavailable,
        Permission::NotDetermined => Status::NotDetermined,
        Permission::Granted | Permission::NotRequired => Status::Granted,
        Permission::Denied => Status::Denied,
        Permission::Restricted => Status::Restricted,
    }
}
fn action(g: &Grant) -> Option<Action> {
    match g.access {
        Permission::NotDetermined | Permission::Denied => Some(if g.requestable {
            Action::RequestPermission
        } else {
            Action::OpenSettings
        }),
        _ => None,
    }
}
impl Observation {
    pub fn unknown(platform: Platform) -> Self {
        let unknown = Grant {
            access: Permission::Unknown,
            requestable: false,
            rationale: false,
        };
        Self {
            platform,
            bluetooth_scan: unknown.clone(),
            bluetooth_connect: unknown.clone(),
            location: unknown.clone(),
            background_location: unknown,
            accuracy: Accuracy::Unknown,
            scope: Scope::Unknown,
            location_services: None,
            bluetooth_service: None,
            bluetooth_supported: None,
            background_option_label: None,
        }
    }
    pub fn location_capability(&self) -> (crate::os::LocationAuthorization, bool) {
        use crate::os::LocationAuthorization as L;
        if self.platform == Platform::Linux || self.location_services != Some(true) {
            return (L::Unavailable, false);
        }
        match self.location.access {
            Permission::Unknown => (L::Unavailable, false),
            Permission::NotDetermined => (L::NotDetermined, false),
            Permission::Denied => (L::Denied, false),
            Permission::Restricted => (L::Restricted, false),
            Permission::Granted | Permission::NotRequired => {
                let background = if matches!(self.platform, Platform::Ios | Platform::Macos) {
                    self.scope == Scope::Always
                } else {
                    granted(self.background_location.access)
                };
                (
                    if background {
                        L::Authorized
                    } else {
                        L::ForegroundOnly
                    },
                    background,
                )
            }
        }
    }

    pub fn valid(&self) -> bool {
        self.background_option_label.as_ref().is_none_or(|s| {
            !s.is_empty() && s.chars().count() <= 120 && !s.chars().any(char::is_control)
        })
    }
    pub fn rows(&self, pending: Option<Id>) -> Vec<Row> {
        let android = matches!(
            self.platform,
            Platform::Android29 | Platform::Android30 | Platform::Android31
        );
        let mut ids = vec![Id::Location];
        if !matches!(self.platform, Platform::Android29 | Platform::Android30) {
            ids.insert(0, Id::Bluetooth);
        }
        if android {
            ids.push(Id::BackgroundLocation);
        }
        ids.into_iter()
            .map(|id| {
                let grant = match id {
                    Id::Bluetooth => [&self.bluetooth_scan, &self.bluetooth_connect]
                        .into_iter()
                        .find(|g| g.access == Permission::Restricted)
                        .or_else(|| {
                            [&self.bluetooth_scan, &self.bluetooth_connect]
                                .into_iter()
                                .find(|g| g.access == Permission::Unknown)
                        })
                        .or_else(|| {
                            [&self.bluetooth_scan, &self.bluetooth_connect]
                                .into_iter()
                                .find(|g| !granted(g.access) && !g.requestable)
                        })
                        .or_else(|| {
                            [&self.bluetooth_scan, &self.bluetooth_connect]
                                .into_iter()
                                .find(|g| !granted(g.access))
                        })
                        .unwrap_or(&self.bluetooth_scan),
                    Id::Location => &self.location,
                    Id::BackgroundLocation => &self.background_location,
                };
                let mut row = Row {
                    permission: id,
                    status: status(grant),
                    action: action(grant),
                    guidance: Guidance::None,
                    pending: pending == Some(id),
                    optional: id != Id::Bluetooth
                        && !(id == Id::Location
                            && matches!(self.platform, Platform::Android29 | Platform::Android30)),
                    services_enabled: if id == Id::Bluetooth {
                        self.bluetooth_service
                    } else {
                        self.location_services
                    },
                    scan: (id == Id::Bluetooth).then_some(self.bluetooth_scan.access),
                    connect: (id == Id::Bluetooth).then_some(self.bluetooth_connect.access),
                    accuracy: if id == Id::Bluetooth {
                        Accuracy::Unknown
                    } else {
                        self.accuracy
                    },
                    scope: if id == Id::Bluetooth {
                        Scope::Unknown
                    } else {
                        self.scope
                    },
                    option_label: None,
                };
                if id == Id::Bluetooth
                    && granted(self.bluetooth_scan.access) != granted(self.bluetooth_connect.access)
                {
                    row.status = Status::Limited;
                }
                if id == Id::Location
                    && granted(grant.access)
                    && (self.accuracy == Accuracy::Approximate
                        || self.scope == Scope::WhileInUse && self.platform == Platform::Ios)
                {
                    row.status = Status::Limited;
                }
                if id == Id::Location
                    && self.platform == Platform::Ios
                    && self.scope == Scope::WhileInUse
                    && granted(grant.access)
                {
                    row.action = Some(if grant.requestable {
                        Action::RequestPermission
                    } else {
                        Action::OpenSettings
                    });
                    row.guidance = Guidance::AlwaysAccess;
                }
                if id == Id::BackgroundLocation && !granted(self.location.access) {
                    row.action = None;
                    row.guidance = Guidance::ForegroundRequired;
                } else if id == Id::BackgroundLocation
                    && !granted(grant.access)
                    && !matches!(grant.access, Permission::Unknown | Permission::Restricted)
                    && self.platform != Platform::Android29
                {
                    row.action = Some(Action::OpenSettings);
                    row.guidance = Guidance::BackgroundSettings;
                    row.option_label = self.background_option_label.clone();
                } else if row.guidance == Guidance::None {
                    row.guidance = if grant.access == Permission::Unknown {
                        Guidance::Checking
                    } else if grant.access == Permission::Restricted {
                        Guidance::Restricted
                    } else if row.action == Some(Action::RequestPermission) && grant.rationale {
                        Guidance::Rationale
                    } else if row.action == Some(Action::OpenSettings) {
                        match (self.platform, id) {
                            (Platform::Ios, _) => Guidance::IosSettings,
                            (Platform::Macos, Id::Bluetooth) => Guidance::MacBluetooth,
                            (Platform::Macos, _) => Guidance::MacLocation,
                            _ => Guidance::AppSettings,
                        }
                    } else if id == Id::Bluetooth && self.bluetooth_supported == Some(false) {
                        Guidance::UnsupportedHardware
                    } else if row.services_enabled == Some(false) {
                        Guidance::ServicesDisabled
                    } else if id == Id::Location
                        && matches!(self.platform, Platform::Android29 | Platform::Android30)
                        && self.accuracy != Accuracy::Precise
                    {
                        Guidance::FineLocationForScan
                    } else {
                        Guidance::None
                    };
                }
                if id == Id::Location
                    && matches!(self.platform, Platform::Android29 | Platform::Android30)
                    && granted(grant.access)
                    && self.accuracy == Accuracy::Approximate
                {
                    row.action = action(&self.bluetooth_scan);
                }
                // Linux has no portable OS permission prompt/settings destination.
                // Live location acquisition is intentionally unavailable in this host.
                if self.platform == Platform::Linux {
                    row.action = None;
                    row.pending = false;
                    if id == Id::Location {
                        row.status = Status::Unavailable;
                        row.guidance = Guidance::None;
                        row.accuracy = Accuracy::Unknown;
                        row.scope = Scope::Unknown;
                    } else {
                        row.guidance = if self.bluetooth_supported == Some(false) {
                            Guidance::UnsupportedHardware
                        } else if grant.access == Permission::Restricted
                            || grant.access == Permission::Denied
                        {
                            Guidance::Restricted
                        } else if self.bluetooth_service == Some(false) {
                            Guidance::ServicesDisabled
                        } else if grant.access == Permission::Unknown {
                            Guidance::Checking
                        } else {
                            Guidance::None
                        };
                        if self.bluetooth_supported == Some(false) {
                            row.status = Status::Unavailable;
                        }
                    }
                }
                row
            })
            .collect()
    }
}
impl BackendService {
    pub(crate) fn location_recovery_row(&self) -> Option<Row> {
        let rows = self.permission_rows();
        rows.iter()
            .find(|r| r.permission == Id::Location && r.action.is_some())
            .or_else(|| {
                rows.iter()
                    .find(|r| r.permission == Id::BackgroundLocation && r.action.is_some())
            })
            .or_else(|| rows.iter().find(|r| r.permission == Id::Location))
            .cloned()
    }
    pub(crate) fn permission_rows(&self) -> Vec<Row> {
        self.policy
            .permissions
            .as_ref()
            .map(|p| p.rows(self.policy.permission_presentation.map(|(_, id, _)| id)))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Dispatch,
        operations::{Failure, Operation},
        os::{Envelope, Fact},
    };
    fn grant(access: Permission, requestable: bool) -> Grant {
        Grant {
            access,
            requestable,
            rationale: false,
        }
    }
    fn observation(platform: Platform) -> Observation {
        Observation {
            platform,
            bluetooth_scan: grant(Permission::NotDetermined, true),
            bluetooth_connect: grant(Permission::NotDetermined, true),
            location: grant(Permission::NotDetermined, true),
            background_location: grant(Permission::NotDetermined, true),
            accuracy: Accuracy::Unknown,
            scope: Scope::Unknown,
            location_services: Some(true),
            bluetooth_service: Some(true),
            bluetooth_supported: Some(true),
            background_option_label: None,
        }
    }
    #[test]
    fn linux_inventory_has_no_fabricated_recovery_or_location_capability() {
        let mut o = observation(Platform::Linux);
        for access in [
            Permission::Unknown,
            Permission::NotDetermined,
            Permission::Granted,
            Permission::Denied,
            Permission::Restricted,
            Permission::NotRequired,
        ] {
            o.bluetooth_scan = grant(access, true);
            o.bluetooth_connect = grant(access, true);
            o.location = grant(Permission::Granted, true);
            o.background_location = grant(Permission::Granted, true);
            o.scope = Scope::Always;
            let rows = o.rows(Some(Id::Bluetooth));
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|r| r.action.is_none() && !r.pending));
            assert_eq!(rows[1].status, Status::Unavailable);
            assert!(rows[1].optional);
            assert_eq!(
                o.location_capability(),
                (crate::os::LocationAuthorization::Unavailable, false)
            );
        }
        o.bluetooth_supported = Some(false);
        assert_eq!(o.rows(None)[0].status, Status::Unavailable);
        assert_eq!(o.rows(None)[0].guidance, Guidance::UnsupportedHardware);
        o.bluetooth_supported = Some(true);
        o.bluetooth_service = Some(false);
        assert_eq!(o.rows(None)[0].guidance, Guidance::ServicesDisabled);
        o.bluetooth_scan = grant(Permission::Denied, false);
        o.bluetooth_connect = grant(Permission::Denied, false);
        assert_eq!(o.rows(None)[0].status, Status::Denied);
        assert_eq!(o.rows(None)[0].guidance, Guidance::Restricted);
    }

    #[test]
    fn linux_unavailable_location_does_not_gate_bluetooth_or_offer_recovery() {
        let root = std::env::temp_dir().join(format!("linux-permissions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut service = BackendService::open(&root).unwrap();
        let mut observation = Observation::unknown(Platform::Linux);
        observation.bluetooth_scan = grant(Permission::NotRequired, false);
        observation.bluetooth_connect = grant(Permission::NotRequired, false);
        observation.bluetooth_supported = Some(true);
        observation.bluetooth_service = Some(true);
        observation.location_services = Some(false);
        for (index, fact) in [
            Fact::Permissions { observation },
            Fact::Bluetooth {
                radio: crate::os::Radio::PoweredOn,
                scan: Permission::NotRequired,
                connect: Permission::NotRequired,
            },
        ]
        .into_iter()
        .enumerate()
        {
            service
                .apply_os(Envelope {
                    lifetime: service.lifetime(),
                    sequence: index as u64 + 1,
                    monotonic_ms: index as u64 + 1,
                    epoch_ms: 1_000 + index as i64,
                    fact,
                })
                .unwrap();
        }
        assert!(service.start_scan(1_000).is_ok());
        let state = service.published_snapshot();
        assert_eq!(state.location.authorization, "unavailable");
        assert_eq!(state.location.background_capture, "unavailable");
        assert!(state.location.permission_action.is_none());
        assert!(matches!(
            service.dispatch(
                1,
                Operation::RecoverPermission {
                    permission: Id::Location,
                    action: Action::RequestPermission
                }
            ),
            Err(Failure::InvalidState)
        ));
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inventory_matrix_separates_authorization_services_accuracy_and_prerequisites() {
        for platform in [
            Platform::Ios,
            Platform::Macos,
            Platform::Android29,
            Platform::Android30,
            Platform::Android31,
        ] {
            let mut o = observation(platform);
            let rows = o.rows(None);
            assert_eq!(
                rows.len(),
                if platform == Platform::Android31 {
                    3
                } else {
                    2
                }
            );
            assert_eq!(
                rows.iter().any(|r| r.permission == Id::Bluetooth),
                !matches!(platform, Platform::Android29 | Platform::Android30)
            );
            if let Some(bg) = rows.iter().find(|r| r.permission == Id::BackgroundLocation) {
                assert_eq!(bg.action, None);
                assert_eq!(bg.guidance, Guidance::ForegroundRequired);
            }
            o.location = grant(Permission::Granted, false);
            o.accuracy = Accuracy::Approximate;
            o.scope = Scope::WhileInUse;
            o.location_services = Some(false);
            let rows = o.rows(None);
            let loc = rows.iter().find(|r| r.permission == Id::Location).unwrap();
            assert_eq!(loc.status, Status::Limited);
            assert_eq!(loc.services_enabled, Some(false));
            if let Some(bg) = rows.iter().find(|r| r.permission == Id::BackgroundLocation) {
                assert_eq!(
                    bg.action,
                    Some(if platform == Platform::Android29 {
                        Action::RequestPermission
                    } else {
                        Action::OpenSettings
                    })
                );
            }
            o.location = grant(Permission::Restricted, true);
            assert_eq!(
                o.rows(None)
                    .iter()
                    .find(|r| r.permission == Id::Location)
                    .unwrap()
                    .action,
                None
            );
            o.location = grant(Permission::Unknown, true);
            o.scope = Scope::Unknown;
            assert_eq!(
                o.rows(None)
                    .iter()
                    .find(|r| r.permission == Id::Location)
                    .unwrap()
                    .status,
                Status::Unavailable
            );
        }
    }
    #[test]
    fn denial_rationale_partial_grants_and_closed_decoding() {
        let mut o = observation(Platform::Android31);
        o.bluetooth_scan = grant(Permission::Granted, false);
        o.bluetooth_connect = grant(Permission::Denied, true);
        o.bluetooth_connect.rationale = true;
        let row = &o.rows(None)[0];
        assert_eq!(row.status, Status::Limited);
        assert_eq!(row.guidance, Guidance::Rationale);
        assert_eq!(row.action, Some(Action::RequestPermission));
        o.bluetooth_connect.requestable = false;
        assert_eq!(o.rows(None)[0].action, Some(Action::OpenSettings));
        let mut value = serde_json::to_value(&o).unwrap();
        value["platform"] = "android99".into();
        assert!(serde_json::from_value::<Observation>(value).is_err());
        let mut value = serde_json::to_value(&o).unwrap();
        value["location"]["access"] = "maybe".into();
        assert!(serde_json::from_value::<Observation>(value).is_err());
        o.background_option_label = Some("x".repeat(121));
        assert!(!o.valid());
    }
    #[test]
    fn reconstruction_and_observation_changes_never_depend_on_recording_preference() {
        let mut o = observation(Platform::Ios);
        o.location = grant(Permission::Granted, false);
        o.scope = Scope::Always;
        o.accuracy = Accuracy::Precise;
        assert_eq!(o.rows(None)[1].status, Status::Granted);
        o.location_services = Some(false);
        assert_eq!(o.rows(None)[1].status, Status::Granted);
        assert_eq!(o.rows(None)[1].guidance, Guidance::ServicesDisabled);
        o.location = grant(Permission::Denied, false);
        o.scope = Scope::Unknown;
        assert_eq!(o.rows(None)[1].action, Some(Action::OpenSettings));
        o.location = grant(Permission::Unknown, false);
        assert_eq!(o.rows(None)[1].status, Status::Unavailable);
        assert_eq!(o.rows(None)[1].action, None);
        o.bluetooth_supported = Some(false);
        o.bluetooth_scan = grant(Permission::Granted, false);
        o.bluetooth_connect = grant(Permission::Granted, false);
        assert_eq!(o.rows(None)[0].guidance, Guidance::UnsupportedHardware);
    }

    #[test]
    fn recovery_is_single_slot_and_timeout_disconnect_and_late_results_are_safe() {
        let root = std::env::temp_dir().join(format!("permissions-service-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut service = BackendService::open(&root).unwrap();
        fn fact(s: &mut BackendService, fact: Fact, now: u64) {
            let p = &s.policy.exchange;
            let e = Envelope {
                lifetime: p.lifetime,
                sequence: p.sequence + 1,
                monotonic_ms: now,
                epoch_ms: now as i64,
                fact,
            };
            s.apply_os(e).unwrap();
        }
        fact(
            &mut service,
            Fact::Permissions {
                observation: observation(Platform::Ios),
            },
            1,
        );
        let recover = || Operation::RecoverPermission {
            permission: Id::Location,
            action: Action::RequestPermission,
        };
        assert!(matches!(
            service.dispatch(
                1,
                Operation::RecoverPermission {
                    permission: Id::BackgroundLocation,
                    action: Action::RequestPermission
                }
            ),
            Err(Failure::InvalidState)
        ));
        assert!(matches!(
            service.dispatch(
                1,
                Operation::RecoverPermission {
                    permission: Id::Location,
                    action: Action::OpenSettings
                }
            ),
            Err(Failure::InvalidState)
        ));
        fact(&mut service, Fact::Foreground { foreground: false }, 2);
        assert!(matches!(
            service.dispatch(1, recover()),
            Err(Failure::Unavailable)
        ));
        fact(&mut service, Fact::Foreground { foreground: true }, 3);
        let Dispatch::Pending(mut reply) = service.dispatch(1, recover()).unwrap() else {
            panic!()
        };
        let id = service.policy.permission_presentation.unwrap().0;
        assert!(!service.policy.settings.location_recording_enabled);
        assert!(service.policy.scan.is_none());
        assert!(service.policy.capture.is_none());
        assert!(service.permission_rows().iter().any(|r| r.pending));
        assert!(matches!(
            service.dispatch(2, Operation::RequestBluetoothPermission),
            Err(Failure::Conflict)
        ));
        service.disconnect_client(1);
        assert!(matches!(reply.try_recv(), Ok(Err(Failure::Unavailable))));
        assert!(matches!(
            service.dispatch(2, recover()),
            Err(Failure::Conflict)
        ));
        fact(&mut service, Fact::Clock {}, 30_004);
        let Dispatch::Pending(mut next) = service.dispatch(2, recover()).unwrap() else {
            panic!()
        };
        let next_id = service.policy.permission_presentation.unwrap().0;
        fact(
            &mut service,
            Fact::UiCompleted { id, success: true },
            30_005,
        );
        assert!(next.try_recv().is_err());
        assert_eq!(service.policy.permission_presentation.unwrap().0, next_id);
        fact(
            &mut service,
            Fact::UiCompleted {
                id: next_id,
                success: false,
            },
            30_006,
        );
        assert!(matches!(next.try_recv(), Ok(Err(Failure::Unavailable))));
        assert!(!service.permission_rows().iter().any(|r| r.pending));
        let Dispatch::Pending(mut timeout) = service.dispatch(3, recover()).unwrap() else {
            panic!()
        };
        let id = service.policy.permission_presentation.unwrap().0;
        fact(
            &mut service,
            Fact::UiCompleted { id, success: true },
            60_007,
        );
        assert!(matches!(timeout.try_recv(), Ok(Err(Failure::Unavailable))));
        assert_eq!(service.permission_rows()[1].status, Status::NotDetermined);
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }
}
