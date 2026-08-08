use domain_types::RideState;

use crate::{
    ControlProfile, FootpadState, LightingProfile, PackageMode, PackageState, RefloatCommand,
    request,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanControl {
    Package,
    Flywheel,
    Leds,
    Headlights,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LightingState {
    pub leds_enabled: bool,
    pub headlights_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlError {
    InvalidFrame,
    Unsupported,
    InvalidState,
}

#[must_use]
pub fn package_request(enabled: bool) -> Vec<u8> {
    request(RefloatCommand::PackageLock, &[u8::from(!enabled)])
}

#[must_use]
pub fn flywheel_request(enabled: bool) -> Vec<u8> {
    request(
        RefloatCommand::Flywheel,
        &[if enabled { 0x81 } else { 0x80 }, 0, 0, 0, 0, 1],
    )
}

/// Builds one exact lighting request while preserving the other light bit.
///
/// # Errors
///
/// Rejects non-lighting controls.
pub fn lighting_request(
    profile: ControlProfile,
    control: Option<BooleanControl>,
    enabled: bool,
    current: LightingState,
) -> Result<Vec<u8>, ControlError> {
    if matches!(
        control,
        Some(BooleanControl::Package | BooleanControl::Flywheel)
    ) {
        return Err(ControlError::InvalidFrame);
    }
    let mask = match control {
        Some(BooleanControl::Leds) => 1,
        Some(BooleanControl::Headlights) => 2,
        None => 0,
        _ => unreachable!(),
    };
    let mut flags = u8::from(current.leds_enabled) | (u8::from(current.headlights_enabled) << 1);
    if let Some(control) = control {
        let bit = if control == BooleanControl::Leds {
            1
        } else {
            2
        };
        if enabled {
            flags |= bit;
        } else {
            flags &= !bit;
        }
    }
    Ok(match profile.lighting {
        LightingProfile::LegacyCommand202 => request(RefloatCommand::LegacyLights, &[mask, flags]),
        LightingProfile::StableCommand20 => {
            request(RefloatCommand::StableLights, &[0, 0, 0, mask, flags])
        }
    })
}

/// Decodes the exact four-byte lighting response for a selected profile.
///
/// # Errors
///
/// Rejects the wrong command, length, package, and flag bits.
pub fn decode_lighting(
    payload: &[u8],
    profile: ControlProfile,
) -> Result<LightingState, ControlError> {
    let command = match profile.lighting {
        LightingProfile::LegacyCommand202 => RefloatCommand::LegacyLights,
        LightingProfile::StableCommand20 => RefloatCommand::StableLights,
    };
    if payload.len() != 4
        || payload[..3] != [36, crate::PACKAGE_ID, command as u8]
        || payload[3] & !3 != 0
    {
        return Err(ControlError::InvalidFrame);
    }
    Ok(LightingState {
        leds_enabled: payload[3] & 1 != 0,
        headlights_enabled: payload[3] & 2 != 0,
    })
}

/// Builds the stable fatal-latch clear request.
///
/// # Errors
///
/// Rejects profiles that do not source this command.
pub fn fatal_clear_request(profile: ControlProfile) -> Result<Vec<u8>, ControlError> {
    profile
        .fatal_clear_available
        .then(|| request(RefloatCommand::AlertsControl, &[1]))
        .ok_or(ControlError::Unsupported)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingAction {
    Package,
    Flywheel,
    Leds,
    Headlights,
    ClearFatal,
    Shutdown,
    Configuration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlState {
    pub writable: bool,
    pub package_enabled: Option<bool>,
    pub flywheel_enabled: Option<bool>,
    pub lighting: Option<LightingState>,
    pub fatal_latched: Option<bool>,
    pub fatal_active: Option<bool>,
    pub pending: Option<PendingAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MutationContext {
    pub generation: u64,
    pub profile: ControlProfile,
    pub connection_ready: bool,
    pub writable: bool,
    pub ride_state: RideState,
    pub package_state: PackageState,
    pub package_mode: PackageMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigurationMutationContext {
    pub generation: u64,
    pub connection_ready: bool,
    pub writable: bool,
    pub ride_state: RideState,
    pub footpad_state: FootpadState,
    pub package_state: PackageState,
    pub package_mode: PackageMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Authorization {
    Idempotent,
    Mutation {
        action: PendingAction,
        payload: Vec<u8>,
        one_way: bool,
    },
}

pub struct MutationCoordinator {
    generation: u64,
    pub state: ControlState,
    expected: Option<bool>,
}

impl MutationCoordinator {
    #[must_use]
    pub const fn new(generation: u64, writable: bool) -> Self {
        Self {
            generation,
            state: ControlState {
                writable,
                package_enabled: None,
                flywheel_enabled: None,
                lighting: None,
                fatal_latched: None,
                fatal_active: None,
                pending: None,
            },
            expected: None,
        }
    }

    pub fn observe(
        &mut self,
        package_state: PackageState,
        package_mode: PackageMode,
        lighting: Option<LightingState>,
        fatal_latched: Option<bool>,
        fatal_active: Option<bool>,
    ) {
        self.state.package_enabled = Some(package_state != PackageState::Disabled);
        self.state.flywheel_enabled = Some(package_mode == PackageMode::Flywheel);
        if let Some(lighting) = lighting {
            self.state.lighting = Some(lighting);
        }
        if fatal_latched.is_some() {
            self.state.fatal_latched = fatal_latched;
        }
        if fatal_active.is_some() {
            self.state.fatal_active = fatal_active;
        }
        let confirmed = match self.state.pending {
            Some(PendingAction::Package) => self.state.package_enabled == self.expected,
            Some(PendingAction::Flywheel) => self.state.flywheel_enabled == self.expected,
            Some(PendingAction::Leds) => {
                self.state.lighting.map(|value| value.leds_enabled) == self.expected
            }
            Some(PendingAction::Headlights) => {
                self.state.lighting.map(|value| value.headlights_enabled) == self.expected
            }
            Some(PendingAction::ClearFatal) => {
                self.state.fatal_latched == Some(false) && self.state.fatal_active == Some(false)
            }
            _ => false,
        };
        if confirmed {
            self.state.pending = None;
            self.expected = None;
        }
    }

    /// Authorizes and serializes one semantic control mutation.
    ///
    /// # Errors
    ///
    /// Fails closed for stale/read-only/not-ready state, another pending
    /// mutation, motion, unsafe flywheel transitions, and unknown lighting.
    pub fn authorize_set(
        &mut self,
        control: BooleanControl,
        enabled: bool,
        context: MutationContext,
    ) -> Result<Authorization, ControlError> {
        self.validate(context)?;
        if self.state.pending.is_some() {
            return Err(ControlError::InvalidState);
        }
        if self.state.flywheel_enabled == Some(true)
            && (control != BooleanControl::Flywheel || enabled)
        {
            return Err(ControlError::InvalidState);
        }
        let active_disable = control == BooleanControl::Flywheel
            && !enabled
            && context.package_mode == PackageMode::Flywheel;
        if matches!(control, BooleanControl::Package | BooleanControl::Flywheel)
            && !active_disable
            && context.ride_state != RideState::Idle
        {
            return Err(ControlError::InvalidState);
        }
        if control == BooleanControl::Flywheel
            && enabled
            && (context.package_state != PackageState::Ready
                || context.package_mode != PackageMode::Normal)
        {
            return Err(ControlError::InvalidState);
        }
        if control == BooleanControl::Flywheel
            && !enabled
            && context.package_mode != PackageMode::Flywheel
        {
            return Err(ControlError::InvalidState);
        }
        let confirmed = match control {
            BooleanControl::Package => self.state.package_enabled,
            BooleanControl::Flywheel => self.state.flywheel_enabled,
            BooleanControl::Leds => self.state.lighting.map(|v| v.leds_enabled),
            BooleanControl::Headlights => self.state.lighting.map(|v| v.headlights_enabled),
        };
        if confirmed == Some(enabled) {
            return Ok(Authorization::Idempotent);
        }
        let (action, payload, one_way) = match control {
            BooleanControl::Package => (PendingAction::Package, package_request(enabled), true),
            BooleanControl::Flywheel => (PendingAction::Flywheel, flywheel_request(enabled), true),
            BooleanControl::Leds => (
                PendingAction::Leds,
                lighting_request(
                    context.profile,
                    Some(control),
                    enabled,
                    self.state.lighting.ok_or(ControlError::InvalidState)?,
                )?,
                false,
            ),
            BooleanControl::Headlights => (
                PendingAction::Headlights,
                lighting_request(
                    context.profile,
                    Some(control),
                    enabled,
                    self.state.lighting.ok_or(ControlError::InvalidState)?,
                )?,
                false,
            ),
        };
        self.state.pending = Some(action);
        self.expected = Some(enabled);
        Ok(Authorization::Mutation {
            action,
            payload,
            one_way,
        })
    }

    /// Authorizes shutdown through the same sole-pending gate.
    ///
    /// # Errors
    ///
    /// Rejects stale, disconnected, unsupported, moving, flywheel, or pending state.
    pub fn authorize_shutdown(
        &mut self,
        generation: u64,
        connection_ready: bool,
        supported: bool,
        ride_state: RideState,
        mode: PackageMode,
    ) -> Result<Vec<u8>, ControlError> {
        if generation != self.generation
            || !connection_ready
            || !supported
            || ride_state != RideState::Idle
            || mode == PackageMode::Flywheel
            || self.state.pending.is_some()
        {
            return Err(ControlError::InvalidState);
        }
        self.state.pending = Some(PendingAction::Shutdown);
        Ok(vec![156, 0, 0])
    }

    /// Authorizes clearing an inactive, latched fatal error.
    ///
    /// # Errors
    ///
    /// Fails closed for stale/read-only/moving/flywheel/pending state or when
    /// the audited profile and observed telemetry do not prove clearability.
    pub fn authorize_fatal_clear(
        &mut self,
        context: MutationContext,
    ) -> Result<Vec<u8>, ControlError> {
        self.validate(context)?;
        if self.state.pending.is_some()
            || self.state.flywheel_enabled == Some(true)
            || context.ride_state != RideState::Idle
            || !context.profile.fatal_clear_available
            || self.state.fatal_latched != Some(true)
            || self.state.fatal_active != Some(false)
        {
            return Err(ControlError::InvalidState);
        }
        let payload = fatal_clear_request(context.profile)?;
        self.state.pending = Some(PendingAction::ClearFatal);
        self.expected = Some(false);
        Ok(payload)
    }

    /// Authorizes configuration through the same connection-scoped mutation gate.
    ///
    /// # Errors
    ///
    /// Fails closed unless the generation is current and the connected board is
    /// writable, idle, unengaged, ready, in normal mode, and mutation-free.
    pub fn authorize_configuration(
        &mut self,
        context: ConfigurationMutationContext,
    ) -> Result<(), ControlError> {
        if context.generation != self.generation
            || !context.connection_ready
            || !context.writable
            || !self.state.writable
            || context.ride_state != RideState::Idle
            || context.footpad_state != FootpadState::None
            || context.package_state != PackageState::Ready
            || context.package_mode != PackageMode::Normal
            || self.state.pending.is_some()
        {
            return Err(ControlError::InvalidState);
        }
        self.state.pending = Some(PendingAction::Configuration);
        Ok(())
    }

    pub fn finish_configuration(&mut self, generation: u64) -> bool {
        if generation != self.generation || self.state.pending != Some(PendingAction::Configuration)
        {
            return false;
        }
        self.state.pending = None;
        true
    }

    pub fn clear_shutdown(&mut self, generation: u64) -> bool {
        if generation != self.generation || self.state.pending != Some(PendingAction::Shutdown) {
            return false;
        }
        self.state.pending = None;
        true
    }

    pub fn cancel(&mut self) {
        self.state.pending = None;
        self.expected = None;
    }

    fn validate(&self, context: MutationContext) -> Result<(), ControlError> {
        if context.generation != self.generation
            || !context.connection_ready
            || !context.writable
            || !self.state.writable
            || self.state.package_enabled.is_none()
            || self.state.flywheel_enabled.is_none()
        {
            Err(ControlError::InvalidState)
        } else {
            Ok(())
        }
    }
}
