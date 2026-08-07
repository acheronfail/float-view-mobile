use float_view_domain::CanId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum VescCommand {
    GetFirmwareVersion = 0,
    GetRealtimeValues = 4,
    GetMotorConfiguration = 14,
    ForwardCan = 34,
    CustomAppData = 36,
    PingCan = 62,
    GetCustomConfigurationXml = 92,
    GetCustomConfiguration = 93,
    GetCustomConfigurationDefault = 94,
    SetCustomConfiguration = 95,
    BmsGetValues = 96,
    Shutdown = 156,
}

impl TryFrom<u8> for VescCommand {
    type Error = super::ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::GetFirmwareVersion),
            4 => Ok(Self::GetRealtimeValues),
            14 => Ok(Self::GetMotorConfiguration),
            34 => Ok(Self::ForwardCan),
            36 => Ok(Self::CustomAppData),
            62 => Ok(Self::PingCan),
            92 => Ok(Self::GetCustomConfigurationXml),
            93 => Ok(Self::GetCustomConfiguration),
            94 => Ok(Self::GetCustomConfigurationDefault),
            95 => Ok(Self::SetCustomConfiguration),
            96 => Ok(Self::BmsGetValues),
            156 => Ok(Self::Shutdown),
            command => Err(super::ProtocolError::UnsupportedCommand(command)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerRoute {
    Direct,
    Can(CanId),
}

#[must_use]
pub fn route_command(command: &[u8], route: ControllerRoute) -> Vec<u8> {
    match route {
        ControllerRoute::Direct => command.to_vec(),
        ControllerRoute::Can(can_id) => {
            let mut routed = Vec::with_capacity(command.len() + 2);
            routed.extend_from_slice(&[VescCommand::ForwardCan as u8, can_id.get()]);
            routed.extend_from_slice(command);
            routed
        }
    }
}

#[must_use]
pub fn shutdown_command(route: ControllerRoute) -> Vec<u8> {
    route_command(&[VescCommand::Shutdown as u8, 0, 0], route)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpectedResponse {
    pub command: VescCommand,
    pub refloat_command: Option<u8>,
}

impl ExpectedResponse {
    #[must_use]
    pub const fn new(command: VescCommand, refloat_command: Option<u8>) -> Self {
        Self {
            command,
            refloat_command,
        }
    }
}

/// Extracts only protocol response identity. Route attribution remains owned
/// by the connection-wide scheduler and its in-flight request context.
#[must_use]
pub fn response_key(payload: &[u8]) -> Option<ExpectedResponse> {
    let command = VescCommand::try_from(*payload.first()?).ok()?;
    let refloat_command = if command == VescCommand::CustomAppData {
        payload.get(2).copied()
    } else {
        None
    };
    Some(ExpectedResponse::new(command, refloat_command))
}
