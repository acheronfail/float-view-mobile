//! Ordered portable request dispatch and exactly-once completion ownership.
use crate::{
    BackendService, facts as f,
    operations::{Failure, Operation},
    os, projection as p, store_failure,
};
use float_view_application_runtime::{Command, coordinator as c};
use float_view_product_store as store;
use std::collections::BTreeMap;
use tokio::sync::oneshot;

pub enum Dispatch {
    Complete(Box<f::Completion>),
    Pending(oneshot::Receiver<Result<f::Completion, Failure>>),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingKind {
    Permission,
    Bluetooth,
    Location,
    Settings,
    Controls,
    Read(bool),
    Write,
}
pub(crate) struct Pending {
    pub owner: u64,
    pub kind: PendingKind,
    pub deadline: u64,
    pub sender: oneshot::Sender<Result<f::Completion, Failure>>,
}
impl BackendService {
    pub fn dispatch(&mut self, owner: u64, operation: Operation) -> Result<Dispatch, Failure> {
        if self.stopping {
            return Err(Failure::Unavailable);
        }
        self.admit_revision()?;
        self.policy.exchange.admit()?;
        if self.pending.len() >= 64
            || self.pending.values().filter(|p| p.owner == owner).count() >= 8
        {
            return Err(Failure::Unavailable);
        }
        let now = self.policy.exchange.now;
        let epoch = self.policy.exchange.epoch;
        let generation = self.runtime.snapshot().application.generation;
        let result = match operation {
            Operation::AddRideMarker {
                session_id,
                operation_id,
            } => {
                let active = self
                    .policy
                    .active
                    .as_ref()
                    .filter(|a| a.saved)
                    .ok_or(Failure::InvalidState)?;
                active
                    .session
                    .as_ref()
                    .filter(|s| s.session_id == session_id)
                    .ok_or(Failure::InvalidState)?;
                let elapsed =
                    now.saturating_sub(active.session_start.ok_or(Failure::InvalidState)?);
                let capture = self.policy.settings.location_recording_enabled
                    && (self.policy.foreground
                        || (self.policy.location_background
                            && self.policy.location_authorization
                                == os::LocationAuthorization::Authorized))
                    && matches!(
                        self.policy.location_authorization,
                        os::LocationAuthorization::Authorized
                            | os::LocationAuthorization::ForegroundOnly
                    );
                let (marker, boundary) = self
                    .runtime
                    .save_ride_marker(&operation_id, elapsed, epoch, capture)
                    .map_err(|e| match e {
                        float_view_application_runtime::RuntimeError::Persistence(
                            float_view_application_runtime::session::WriterError::InvalidInput,
                        ) => Failure::InvalidRequest,
                        _ => Failure::Unavailable,
                    })?;
                self.diagnostics.event(
                    now,
                    epoch,
                    "ride_marker",
                    serde_json::json!({"sequence":marker.sequence,"elapsed_ms":marker.elapsed_ms}),
                    true,
                );
                self.diagnostics.capture(now, epoch, "ride_marker", true);
                self.store
                    .apply_session_marker(&session_id, &boundary)
                    .map_err(store_failure)?;
                let session = self.store.session(&session_id).map_err(store_failure)?;
                let boundary = p::boundary(&session);
                self.policy
                    .active
                    .as_mut()
                    .ok_or(Failure::InvalidState)?
                    .session = Some(session);
                f::Completion::RideMarker { marker, boundary }
            }
            Operation::RecoverPermission { permission, action } => {
                return self.recover_permission(owner, permission, action, PendingKind::Permission);
            }
            Operation::RequestBluetoothPermission => {
                return self.request_ui(owner, PendingKind::Bluetooth);
            }
            Operation::RequestLocationPermission => {
                return self.request_ui(owner, PendingKind::Location);
            }
            Operation::OpenLocationSettings => {
                return self.request_ui(owner, PendingKind::Settings);
            }
            Operation::Scan { timeout_ms } => {
                self.start_scan(timeout_ms)?;
                f::Completion::Unit
            }
            Operation::Connect { device_id, saved } => {
                self.connect(&device_id, saved)?;
                f::Completion::Unit
            }
            Operation::Disconnect => {
                self.disconnect("explicit_disconnect")?;
                f::Completion::Unit
            }
            Operation::SelectController { device_id, can_id } => {
                let a = self
                    .policy
                    .active
                    .as_ref()
                    .filter(|a| a.device.id == device_id)
                    .ok_or(Failure::InvalidState)?;
                if self.runtime.snapshot().application.phase != c::Phase::SelectingController
                    || !self
                        .runtime
                        .snapshot()
                        .application
                        .discovery
                        .candidates
                        .iter()
                        .any(|v| v.can_id == can_id)
                {
                    return Err(Failure::InvalidState);
                }
                if a.saved {
                    self.policy
                        .active
                        .as_mut()
                        .ok_or(Failure::InvalidState)?
                        .selected_can_id = Some(can_id);
                }
                self.runtime_command(Command::Application(c::Command::SelectController {
                    generation,
                    can_id,
                    now_ms: now,
                }))?;
                f::Completion::Unit
            }
            Operation::SetControl { control, enabled } => {
                let control = match control.as_str() {
                    "package" => c::mutation::BooleanControl::Package,
                    "flywheel" => c::mutation::BooleanControl::Flywheel,
                    "leds" => c::mutation::BooleanControl::Leds,
                    "headlights" => c::mutation::BooleanControl::Headlights,
                    _ => return Err(Failure::InvalidRequest),
                };
                return self.kernel_request(
                    owner,
                    PendingKind::Controls,
                    c::Command::SetControl {
                        generation,
                        control,
                        enabled,
                        now_ms: now,
                    },
                );
            }
            Operation::ClearFatalError => {
                return self.kernel_request(
                    owner,
                    PendingKind::Controls,
                    c::Command::ClearFatalError {
                        generation,
                        now_ms: now,
                    },
                );
            }
            Operation::ShutdownController => {
                self.runtime_command(Command::Application(c::Command::ShutdownController {
                    generation,
                    now_ms: now,
                }))?;
                f::Completion::Unit
            }
            Operation::ConfigurationSchema => f::Completion::ConfigurationSchema(schema(
                self.runtime
                    .configuration_schema()
                    .ok_or(Failure::InvalidState)?,
            )),
            Operation::ReadConfiguration { defaults } => {
                return self.kernel_request(
                    owner,
                    PendingKind::Read(defaults),
                    c::Command::ReadConfiguration {
                        generation,
                        defaults,
                        now_ms: now,
                    },
                );
            }
            Operation::WriteConfiguration {
                package_version,
                fingerprint,
                signature,
                revision,
                values,
            } => {
                return self.kernel_request(
                    owner,
                    PendingKind::Write,
                    c::Command::WriteConfiguration {
                        generation,
                        mutation: c::mutation::ConfigurationMutation::Write,
                        package_version,
                        fingerprint,
                        wire_signature: signature,
                        base_revision: revision,
                        values: codec_values(values),
                        now_ms: now,
                    },
                );
            }
            Operation::ListTunes {
                identity,
                limit,
                cursor,
            } => {
                let page = self
                    .store
                    .tunes(&identity_store(identity), limit, cursor.as_deref())
                    .map_err(store_failure)?;
                f::Completion::Tunes(f::TunePage {
                    items: page.items.iter().map(tune).collect::<Result<_, _>>()?,
                    next_cursor: page.next_cursor,
                })
            }
            Operation::GetTune { id } => {
                f::Completion::Tune(tune(&self.store.tune(&id).map_err(store_failure)?)?)
            }
            Operation::CreateTune {
                name,
                description,
                identity,
                revision,
                values,
            } => {
                self.validate_configuration_source(&identity, revision)?;
                let schema = self
                    .runtime
                    .configuration_schema()
                    .ok_or(Failure::InvalidState)?;
                f::Completion::Tune(tune(
                    &self
                        .store
                        .create_tune(
                            &name,
                            description.as_deref(),
                            &identity_store(identity),
                            &store_values(values),
                            schema,
                            epoch,
                        )
                        .map_err(store_failure)?,
                )?)
            }
            Operation::UpdateTune {
                id,
                revision,
                name,
                description,
                values,
            } => {
                let schema = self
                    .runtime
                    .configuration_schema()
                    .ok_or(Failure::InvalidState)?;
                f::Completion::Tune(tune(
                    &self
                        .store
                        .update_tune(
                            &id,
                            revision,
                            &name,
                            description.as_deref(),
                            &store_values(values),
                            schema,
                            epoch,
                        )
                        .map_err(store_failure)?,
                )?)
            }
            Operation::DeleteTune { id, revision } => {
                self.store
                    .delete_tune(&id, revision)
                    .map_err(store_failure)?;
                f::Completion::Unit
            }
            Operation::ApplyTune {
                id,
                revision,
                base_revision,
            } => {
                let tune = self.store.tune(&id).map_err(store_failure)?;
                if tune.revision != revision {
                    return Err(Failure::Conflict);
                }
                let schema = self
                    .runtime
                    .configuration_schema()
                    .ok_or(Failure::InvalidState)?;
                store::validate_tune_values(&tune.identity, &tune.values, schema)
                    .map_err(store_failure)?;
                return self.kernel_request(
                    owner,
                    PendingKind::Write,
                    c::Command::WriteConfiguration {
                        generation,
                        mutation: c::mutation::ConfigurationMutation::TuneApply,
                        package_version: tune.identity.package_version,
                        fingerprint: tune.identity.schema_fingerprint,
                        wire_signature: tune.identity.wire_signature,
                        base_revision,
                        values: tune
                            .values
                            .into_iter()
                            .map(|(k, v)| {
                                (
                                    k,
                                    match v {
                                        store::TuneValue::Double(v) => {
                                            c::configuration_codec::ConfigurationValue::Decimal(v)
                                        }
                                        store::TuneValue::Integer(v) => {
                                            c::configuration_codec::ConfigurationValue::Integer(v)
                                        }
                                        store::TuneValue::Boolean(v) => {
                                            c::configuration_codec::ConfigurationValue::Boolean(v)
                                        }
                                        store::TuneValue::String(v) => {
                                            c::configuration_codec::ConfigurationValue::String(v)
                                        }
                                    },
                                )
                            })
                            .collect(),
                        now_ms: now,
                    },
                );
            }
            Operation::UpdateSavedBoard { id, revision, name } => {
                let b = self
                    .store
                    .update_board(&id, revision, name.as_deref())
                    .map_err(store_failure)?;
                self.refresh_boards().map_err(store_failure)?;
                f::Completion::SavedBoard(self.board_facts(&b).map_err(store_failure)?)
            }
            Operation::DeleteSavedBoard { id } => {
                self.store.forget_board(&id).map_err(store_failure)?;
                self.refresh_boards().map_err(store_failure)?;
                f::Completion::Unit
            }
            Operation::UpdateSettings {
                revision,
                patch_json,
            } => {
                self.policy.settings = self
                    .store
                    .update_settings(revision, &patch_json)
                    .map_err(store_failure)?;
                if self.policy.active.is_some() {
                    self.runtime_command(Command::Application(c::Command::UpdateSettings {
                        generation,
                        settings: c::telemetry::Settings::new(
                            self.policy.settings.idle_probe_interval_ms,
                            self.policy.settings.post_ride_tail_ms,
                        )
                        .map_err(|_| Failure::InvalidRequest)?,
                        now_ms: now,
                    }))?;
                }
                self.location_eligibility()?;
                f::Completion::Settings(p::settings(&self.policy.settings))
            }
            Operation::GetProfile { id } => {
                f::Completion::Profile(profile(&self.store.profile(&id).map_err(store_failure)?)?)
            }
            Operation::UpdateProfile {
                id,
                revision,
                patch_json,
            } => {
                let value = self
                    .store
                    .update_profile(&id, revision, &patch_json, epoch)
                    .map_err(store_failure)?;
                if self
                    .policy
                    .active
                    .as_ref()
                    .and_then(|a| a.board.as_ref())
                    .is_some_and(|b| b.saved_device_id == id)
                    && self.runtime.snapshot().session_attached
                {
                    self.runtime_command(Command::ProfileChanged {
                        saved_device_id: id,
                        revision: value.revision,
                        values_json: serde_json::to_string(&value.values)
                            .map_err(|_| Failure::Internal)?,
                        at_ms: epoch,
                    })?;
                }
                self.refresh_boards().map_err(store_failure)?;
                f::Completion::Profile(profile(&value)?)
            }
            Operation::ListSessions {
                history_id,
                limit,
                cursor,
                completed_only,
            } => {
                let page = self
                    .store
                    .sessions(
                        history_id.as_deref(),
                        limit,
                        cursor.as_deref(),
                        completed_only,
                    )
                    .map_err(store_failure)?;
                f::Completion::Sessions(f::SessionPage {
                    items: page.items.iter().map(p::session).collect(),
                    next_cursor: page.next_cursor,
                })
            }
            Operation::GetSession { id } => {
                let value = self.store.session(&id).map_err(store_failure)?;
                f::Completion::Session(f::SessionSource {
                    summary: p::session(&value),
                    boundary: p::boundary(&value),
                    path: self
                        .store
                        .session_path(&id)
                        .map_err(store_failure)?
                        .to_str()
                        .ok_or(Failure::Internal)?
                        .into(),
                })
            }
            Operation::DeleteSession { id } => {
                self.store.delete_session(&id).map_err(store_failure)?;
                self.refresh_boards().map_err(store_failure)?;
                f::Completion::Unit
            }
            Operation::CancelDelete { id } => f::Completion::CancelDelete(
                self.store
                    .cancel_pending_delete(&id)
                    .map_err(store_failure)?,
            ),
            Operation::GetMaintenance => f::Completion::Maintenance(
                self.store
                    .maintenance()
                    .map_err(store_failure)?
                    .as_ref()
                    .map(p::maintenance),
            ),
            Operation::CancelMaintenance { id } => f::Completion::Maintenance(Some(
                p::maintenance(&self.cancel_maintenance(&id, epoch).map_err(store_failure)?),
            )),
            Operation::ExportSession { .. }
            | Operation::WriteTransfer { .. }
            | Operation::ReadTransfer { .. }
            | Operation::BeginImport { .. }
            | Operation::FinishImport { .. }
            | Operation::CleanupTransfer { .. } => return Err(Failure::InvalidRequest),
        };
        self.changed()?;
        Ok(Dispatch::Complete(Box::new(result)))
    }
    fn validate_configuration_source(
        &self,
        identity: &f::ConfigurationIdentity,
        revision: u64,
    ) -> Result<(), Failure> {
        let config = self.runtime.snapshot().application.configuration;
        let actual = config.identity.ok_or(Failure::InvalidState)?;
        if identity.package_name != "refloat"
            || identity.package_version != actual.package_version
            || identity.schema_fingerprint != actual.fingerprint
            || identity.wire_signature != actual.wire_signature
        {
            return Err(Failure::InvalidState);
        }
        if config
            .current
            .as_ref()
            .is_none_or(|v| v.revision != revision)
            && config
                .defaults
                .as_ref()
                .is_none_or(|v| v.revision != revision)
        {
            return Err(Failure::Conflict);
        }
        Ok(())
    }
    fn request_ui(&mut self, owner: u64, kind: PendingKind) -> Result<Dispatch, Failure> {
        use crate::permissions::{Action, Id};
        let permission = if kind == PendingKind::Bluetooth {
            if self
                .permission_rows()
                .iter()
                .any(|r| r.permission == Id::Bluetooth)
            {
                Id::Bluetooth
            } else {
                Id::Location
            }
        } else {
            self.location_recovery_row()
                .map(|r| r.permission)
                .unwrap_or(Id::Location)
        };
        let row = self
            .permission_rows()
            .into_iter()
            .find(|r| r.permission == permission)
            .ok_or(Failure::Unavailable)?;
        if row.action != Some(Action::RequestPermission) && kind != PendingKind::Settings {
            return Ok(Dispatch::Complete(Box::new(
                if kind == PendingKind::Bluetooth {
                    f::Completion::Bluetooth(self.bluetooth_facts())
                } else {
                    f::Completion::Location(self.location_facts())
                },
            )));
        }
        let action = if kind == PendingKind::Settings {
            Action::OpenSettings
        } else {
            row.action.ok_or(Failure::InvalidState)?
        };
        self.recover_permission(owner, permission, action, kind)
    }
    fn recover_permission(
        &mut self,
        owner: u64,
        permission: crate::permissions::Id,
        action: crate::permissions::Action,
        kind: PendingKind,
    ) -> Result<Dispatch, Failure> {
        if self.policy.permission_presentation.is_some() {
            return Err(Failure::Conflict);
        }
        let row = self
            .permission_rows()
            .into_iter()
            .find(|r| r.permission == permission)
            .ok_or(Failure::InvalidState)?;
        if row.action != Some(action) {
            return Err(Failure::InvalidState);
        }
        if !self.policy.foreground {
            return Err(Failure::Unavailable);
        }
        let (id, receiver) = self.add_pending(owner, kind, 30_000)?;
        self.policy.permission_presentation = Some((
            id,
            permission,
            self.policy.exchange.now.saturating_add(30_000),
        ));
        self.policy.exchange.push(os::Effect::RecoverPermission {
            id,
            permission,
            action,
        })?;
        self.changed()?;
        Ok(Dispatch::Pending(receiver))
    }
    fn add_pending(
        &mut self,
        owner: u64,
        kind: PendingKind,
        timeout: u64,
    ) -> Result<(u64, oneshot::Receiver<Result<f::Completion, Failure>>), Failure> {
        let id = self.policy.exchange.id()?;
        let (sender, receiver) = oneshot::channel();
        self.pending.insert(
            id,
            Pending {
                owner,
                kind,
                deadline: self.policy.exchange.now.saturating_add(timeout),
                sender,
            },
        );
        self.arm_timer(crate::policy::Timer::Request(id), timeout, None)?;
        Ok((id, receiver))
    }
    fn kernel_request(
        &mut self,
        owner: u64,
        kind: PendingKind,
        command: c::Command,
    ) -> Result<Dispatch, Failure> {
        if self.pending.values().any(|p| {
            matches!(
                p.kind,
                PendingKind::Controls | PendingKind::Read(_) | PendingKind::Write
            )
        }) {
            return Err(Failure::InvalidState);
        }
        let (id, receiver) = self.add_pending(owner, kind, 3_250)?;
        if let Err(error) = self.runtime_command(Command::Application(command)) {
            self.finish_pending(id, Err(error.clone()));
            return Err(error);
        }
        self.complete_configuration();
        self.changed()?;
        Ok(Dispatch::Pending(receiver))
    }
    pub(crate) fn finish_pending(&mut self, id: u64, result: Result<f::Completion, Failure>) {
        if let Some(pending) = self.pending.remove(&id) {
            self.cancel_timer(crate::policy::Timer::Request(id));
            let _ = pending.sender.send(result);
        }
    }
    pub(crate) fn complete_ui(&mut self, id: u64, success: bool) {
        if self
            .policy
            .permission_presentation
            .is_none_or(|(active, _, _)| active != id)
        {
            return;
        }
        self.policy.permission_presentation = None;
        if let Some(p) = self.pending.get(&id) {
            let result = if !success {
                Err(Failure::Unavailable)
            } else {
                match p.kind {
                    PendingKind::Permission => {
                        Ok(f::Completion::Permissions(self.permission_rows()))
                    }
                    PendingKind::Bluetooth => Ok(f::Completion::Bluetooth(self.bluetooth_facts())),
                    PendingKind::Location | PendingKind::Settings => {
                        Ok(f::Completion::Location(self.location_facts()))
                    }
                    _ => return,
                }
            };
            self.finish_pending(id, result);
        }
    }
    pub(crate) fn complete_mutation(&mut self, success: bool) {
        if let Some((&id, p)) = self
            .pending
            .iter()
            .find(|(_, p)| matches!(p.kind, PendingKind::Controls | PendingKind::Write))
        {
            let result = if !success {
                Err(Failure::Unavailable)
            } else {
                match p.kind {
                    PendingKind::Controls => self
                        .runtime
                        .snapshot()
                        .application
                        .mutations
                        .controls
                        .as_ref()
                        .map(p::controls)
                        .map(f::Completion::Controls)
                        .ok_or(Failure::InvalidState),
                    _ => self.configuration_result(false),
                }
            };
            self.finish_pending(id, result);
        }
    }
    fn configuration_result(&self, defaults: bool) -> Result<f::Completion, Failure> {
        let state = self.runtime.snapshot().application.configuration;
        let loaded = if defaults {
            state.defaults
        } else {
            state.current
        }
        .ok_or(Failure::Unavailable)?;
        Ok(f::Completion::ConfigurationValues(f::ConfigurationValues {
            revision: loaded.revision,
            source: p::camel(loaded.source),
            values: loaded
                .values
                .into_iter()
                .map(|(k, v)| (k, value(&v)))
                .collect(),
        }))
    }
    pub(crate) fn complete_configuration(&mut self) {
        if self
            .runtime
            .snapshot()
            .application
            .configuration
            .pending
            .is_some()
        {
            return;
        }
        if let Some((&id, p)) = self
            .pending
            .iter()
            .find(|(_, p)| matches!(p.kind, PendingKind::Read(_)))
        {
            let PendingKind::Read(defaults) = p.kind else {
                return;
            };
            self.finish_pending(id, self.configuration_result(defaults));
        }
    }
    pub(crate) fn fail_kernel_requests(&mut self) {
        let ids: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, p)| {
                matches!(
                    p.kind,
                    PendingKind::Controls | PendingKind::Read(_) | PendingKind::Write
                )
            })
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.finish_pending(id, Err(Failure::Unavailable));
        }
    }
    pub fn disconnect_client(&mut self, owner: u64) {
        let ids: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, p)| p.owner == owner)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.finish_pending(id, Err(Failure::Unavailable));
        }
        self.disconnect_file_client(owner);
    }
}
fn identity_store(i: f::ConfigurationIdentity) -> store::TuneIdentity {
    store::TuneIdentity {
        package_name: i.package_name,
        package_version: i.package_version,
        schema_fingerprint: i.schema_fingerprint,
        wire_signature: i.wire_signature,
    }
}
fn store_values(v: BTreeMap<String, f::ConfigurationValue>) -> store::TuneValues {
    v.into_iter()
        .map(|(k, v)| {
            (
                k,
                match v {
                    f::ConfigurationValue::Double(v) => store::TuneValue::Double(v),
                    f::ConfigurationValue::Integer(v) => store::TuneValue::Integer(v),
                    f::ConfigurationValue::Boolean(v) => store::TuneValue::Boolean(v),
                    f::ConfigurationValue::String(v) => store::TuneValue::String(v),
                },
            )
        })
        .collect()
}
fn codec_values(
    v: BTreeMap<String, f::ConfigurationValue>,
) -> c::configuration_codec::ConfigurationValues {
    v.into_iter()
        .map(|(k, v)| {
            (
                k,
                match v {
                    f::ConfigurationValue::Double(v) => {
                        c::configuration_codec::ConfigurationValue::Decimal(v)
                    }
                    f::ConfigurationValue::Integer(v) => {
                        c::configuration_codec::ConfigurationValue::Integer(v)
                    }
                    f::ConfigurationValue::Boolean(v) => {
                        c::configuration_codec::ConfigurationValue::Boolean(v)
                    }
                    f::ConfigurationValue::String(v) => {
                        c::configuration_codec::ConfigurationValue::String(v)
                    }
                },
            )
        })
        .collect()
}
fn value(v: &c::configuration_codec::ConfigurationValue) -> f::ConfigurationValue {
    match v {
        c::configuration_codec::ConfigurationValue::Decimal(v) => f::ConfigurationValue::Double(*v),
        c::configuration_codec::ConfigurationValue::Integer(v) => {
            f::ConfigurationValue::Integer(*v)
        }
        c::configuration_codec::ConfigurationValue::Boolean(v) => {
            f::ConfigurationValue::Boolean(*v)
        }
        c::configuration_codec::ConfigurationValue::String(v) => {
            f::ConfigurationValue::String(v.clone())
        }
    }
}
fn schema(s: &c::configuration_codec::ConfigurationSchema) -> f::ConfigurationSchema {
    f::ConfigurationSchema {
        fingerprint: s.fingerprint.clone(),
        wire_signature: s.wire_signature,
        fields: s
            .fields
            .iter()
            .map(|v| f::ConfigurationField {
                name: v.name.clone(),
                long_name: v.long_name.clone(),
                field_type: v.field_type as u8,
                wire_type: v.wire_type.map(|w| w as u8),
                default_value: value(&v.default_value),
                minimum: v.minimum,
                maximum: v.maximum,
                step: v.step,
                precision: v.precision.and_then(|v| v.try_into().ok()),
                scale: v.scale,
                suffix: v.suffix.clone(),
                choices: v.choices.clone(),
                description: v.description.clone(),
            })
            .collect(),
        groups: s
            .groups
            .iter()
            .map(|g| f::ConfigurationGroup {
                name: g.name.clone(),
                subgroups: g
                    .subgroups
                    .iter()
                    .map(|g| f::ConfigurationSubgroup {
                        name: g.name.clone(),
                        fields: g.fields.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}
fn profile(p: &store::Profile) -> Result<f::Profile, Failure> {
    Ok(f::Profile {
        saved_device_id: p.saved_device_id.clone(),
        revision: p.revision,
        values_json: serde_json::to_string(&p.values).map_err(|_| Failure::Internal)?,
        missing_fields: p.missing_fields().into_iter().map(str::to_owned).collect(),
    })
}
fn tune(t: &store::Tune) -> Result<f::Tune, Failure> {
    Ok(f::Tune {
        configuration_id: t.configuration_id.clone(),
        revision: t.revision,
        name: t.name.clone(),
        description: t.description.clone(),
        identity: f::ConfigurationIdentity {
            package_name: t.identity.package_name.clone(),
            package_version: t.identity.package_version.clone(),
            schema_fingerprint: t.identity.schema_fingerprint.clone(),
            wire_signature: t.identity.wire_signature,
        },
        values_json: serde_json::to_string(&t.values).map_err(|_| Failure::Internal)?,
        created_at: p::date(t.created_at_ms),
        updated_at: p::date(t.updated_at_ms),
    })
}
