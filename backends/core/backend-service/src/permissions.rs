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
    Android31
});
enumeration!(Id {
    Bluetooth,
    Location,
    BackgroundLocation,
    Notifications,
    LiveUpdates
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
    FineLocationForScan,
    NotificationSettings,
    LiveUpdateSettings,
    NotificationsRequired
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
    pub notifications: Option<Grant>,
    pub live_updates: Option<Grant>,
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
            notifications: None,
            live_updates: None,
            accuracy: Accuracy::Unknown,
            scope: Scope::Unknown,
            location_services: None,
            bluetooth_service: None,
            bluetooth_supported: None,
            background_option_label: None,
        }
    }
    /// The Android host scans without neverForLocation, so discovery needs precise
    /// foreground location and the device Location switch, independently of recording.
    pub fn scan_location_ready(&self) -> bool {
        !matches!(
            self.platform,
            Platform::Android29 | Platform::Android30 | Platform::Android31
        ) || (granted(self.location.access)
            && self.accuracy == Accuracy::Precise
            && self.location_services == Some(true))
    }
    pub fn location_capability(&self) -> (crate::os::LocationAuthorization, bool) {
        use crate::os::LocationAuthorization as L;
        if self.location_services != Some(true) {
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
            if self.notifications.is_some() {
                ids.push(Id::Notifications);
            }
            if self.live_updates.is_some() {
                ids.push(Id::LiveUpdates);
            }
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
                    Id::Notifications => self
                        .notifications
                        .as_ref()
                        .expect("included notification grant"),
                    Id::LiveUpdates => self
                        .live_updates
                        .as_ref()
                        .expect("included promotion grant"),
                };
                let notification = matches!(id, Id::Notifications | Id::LiveUpdates);
                let mut row = Row {
                    permission: id,
                    status: status(grant),
                    action: action(grant),
                    guidance: Guidance::None,
                    pending: pending == Some(id),
                    optional: id != Id::Bluetooth && !(id == Id::Location && android),
                    services_enabled: if notification {
                        None
                    } else if id == Id::Bluetooth {
                        self.bluetooth_service
                    } else {
                        self.location_services
                    },
                    scan: (id == Id::Bluetooth).then_some(self.bluetooth_scan.access),
                    connect: (id == Id::Bluetooth).then_some(self.bluetooth_connect.access),
                    accuracy: if id == Id::Bluetooth || notification {
                        Accuracy::Unknown
                    } else {
                        self.accuracy
                    },
                    scope: if id == Id::Bluetooth || notification {
                        Scope::Unknown
                    } else {
                        self.scope
                    },
                    option_label: None,
                };
                if notification {
                    if !matches!(grant.access, Permission::Unknown | Permission::Restricted) {
                        row.action = Some(
                            if id == Id::Notifications
                                && !granted(grant.access)
                                && grant.requestable
                            {
                                Action::RequestPermission
                            } else {
                                Action::OpenSettings
                            },
                        );
                    }
                    row.guidance = match grant.access {
                        Permission::Unknown => Guidance::Checking,
                        Permission::Restricted => Guidance::Restricted,
                        _ if row.action == Some(Action::RequestPermission) => {
                            if grant.rationale {
                                Guidance::Rationale
                            } else {
                                Guidance::None
                            }
                        }
                        _ if id == Id::Notifications => Guidance::NotificationSettings,
                        _ => Guidance::LiveUpdateSettings,
                    };
                    if id == Id::LiveUpdates
                        && self
                            .notifications
                            .as_ref()
                            .is_some_and(|g| !granted(g.access))
                    {
                        row.action = None;
                        row.guidance = Guidance::NotificationsRequired;
                    }
                    return row;
                }
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
                    } else if id == Id::Location && android && self.accuracy != Accuracy::Precise {
                        Guidance::FineLocationForScan
                    } else {
                        Guidance::None
                    };
                }
                if id == Id::Location && android {
                    if granted(grant.access) && self.accuracy == Accuracy::Approximate {
                        row.action = Some(if grant.requestable {
                            Action::RequestPermission
                        } else {
                            Action::OpenSettings
                        });
                        row.guidance = Guidance::FineLocationForScan;
                    } else if granted(grant.access) && self.location_services == Some(false) {
                        row.action = Some(Action::OpenSettings);
                        row.guidance = Guidance::ServicesDisabled;
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
            notifications: None,
            live_updates: None,
            accuracy: Accuracy::Unknown,
            scope: Scope::Unknown,
            location_services: Some(true),
            bluetooth_service: Some(true),
            bluetooth_supported: Some(true),
            background_option_label: None,
        }
    }
    #[test]
    fn android_discovery_requires_precise_foreground_location_and_services() {
        for platform in [
            Platform::Android29,
            Platform::Android30,
            Platform::Android31,
        ] {
            let mut o = observation(platform);
            assert!(!o.scan_location_ready());
            let location = |o: &Observation| {
                o.rows(None)
                    .into_iter()
                    .find(|r| r.permission == Id::Location)
                    .unwrap()
            };
            assert!(!location(&o).optional);
            assert_eq!(location(&o).action, Some(Action::RequestPermission));
            o.location = grant(Permission::Granted, true);
            o.accuracy = Accuracy::Approximate;
            assert_eq!(location(&o).action, Some(Action::RequestPermission));
            assert_eq!(location(&o).guidance, Guidance::FineLocationForScan);
            assert!(!o.scan_location_ready());
            o.location.requestable = false;
            assert_eq!(location(&o).action, Some(Action::OpenSettings));
            o.accuracy = Accuracy::Precise;
            o.scope = Scope::WhileInUse;
            assert!(o.scan_location_ready());
            o.location_services = Some(false);
            assert!(!o.scan_location_ready());
            assert_eq!(location(&o).action, Some(Action::OpenSettings));
            assert_eq!(location(&o).guidance, Guidance::ServicesDisabled);
        }
        for platform in [Platform::Ios, Platform::Macos] {
            assert!(observation(platform).scan_location_ready());
        }
    }

    #[test]
    fn scan_admission_and_revocation_follow_android_location_facts() {
        let root = std::env::temp_dir().join(format!("scan-location-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut service = BackendService::open(&root).unwrap();
        service.policy.scan_permission = Permission::Granted;
        service.policy.radio = crate::os::Radio::PoweredOn;
        let mut o = observation(Platform::Android31);
        service.policy.permissions = Some(o.clone());
        assert!(matches!(
            service.start_scan(5000),
            Err(Failure::PermissionRequired)
        ));
        assert!(service.policy.scan.is_none());
        o.location = grant(Permission::Granted, false);
        o.accuracy = Accuracy::Precise;
        o.scope = Scope::WhileInUse;
        service.policy.permissions = Some(o.clone());
        assert!(service.start_scan(5000).is_ok());
        assert!(service.policy.scan.is_some());
        o.location_services = Some(false);
        let exchange = &service.policy.exchange;
        service
            .apply_os(Envelope {
                lifetime: exchange.lifetime,
                sequence: exchange.sequence + 1,
                monotonic_ms: 1,
                epoch_ms: 1,
                fact: Fact::Permissions { observation: o },
            })
            .unwrap();
        assert!(service.policy.scan.is_none());
        assert!(matches!(
            service.start_scan(5000),
            Err(Failure::PermissionRequired)
        ));
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn notification_inventory_tracks_requests_denial_settings_and_promotion() {
        let mut o = observation(Platform::Android31);
        o.notifications = Some(grant(Permission::NotDetermined, true));
        o.live_updates = Some(grant(Permission::Denied, false));
        let rows = o.rows(Some(Id::Notifications));
        assert_eq!(rows.len(), 5);
        let notification = &rows[3];
        assert!(notification.optional && notification.pending);
        assert_eq!(notification.action, Some(Action::RequestPermission));
        assert_eq!(notification.accuracy, Accuracy::Unknown);
        assert_eq!(notification.scope, Scope::Unknown);
        assert_eq!(notification.services_enabled, None);
        assert_eq!(rows[4].guidance, Guidance::NotificationsRequired);
        assert_eq!(rows[4].action, None);
        o.notifications = Some(grant(Permission::Denied, false));
        assert_eq!(o.rows(None)[3].action, Some(Action::OpenSettings));
        assert_eq!(o.rows(None)[3].guidance, Guidance::NotificationSettings);
        o.notifications = Some(grant(Permission::Denied, true));
        o.notifications.as_mut().unwrap().rationale = true;
        assert_eq!(o.rows(None)[3].guidance, Guidance::Rationale);
        o.notifications = Some(grant(Permission::Granted, false));
        assert_eq!(o.rows(None)[3].status, Status::Granted);
        assert_eq!(o.rows(None)[4].action, Some(Action::OpenSettings));
        assert_eq!(o.rows(None)[4].guidance, Guidance::LiveUpdateSettings);
        o.live_updates = Some(grant(Permission::Granted, false));
        assert_eq!(o.rows(None)[4].status, Status::Granted);
        o.notifications = Some(grant(Permission::Restricted, false));
        assert_eq!(o.rows(None)[3].action, None);
        o.notifications = Some(grant(Permission::Unknown, false));
        assert_eq!(o.rows(None)[3].guidance, Guidance::Checking);
        o.live_updates = None;
        assert_eq!(o.rows(None).len(), 4);
        o.platform = Platform::Ios;
        assert_eq!(o.rows(None).len(), 2);
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
