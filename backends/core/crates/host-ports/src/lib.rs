//! Typed, bounded platform-port vocabulary. No OS implementation lives here.
use domain_types::{OperationId, PlatformAssociation, RuntimeGeneration};

pub const BRIDGE_CODEC_VERSION: u8 = 1;
pub const MAX_BRIDGE_PAYLOAD_BYTES: usize = 64 * 1024;
pub const MAX_COMPLETION_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BLUETOOTH_BYTES: usize = 4096;
pub const MAX_BLUETOOTH_WRITE_BYTES: usize = MAX_BRIDGE_PAYLOAD_BYTES;
pub const MAX_LOGICAL_NAME_BYTES: usize = 128;
pub const MAX_ENDPOINT_BYTES: usize = 256;
pub const MAX_LAUNCH_TOKEN_BYTES: usize = 128;
pub const MAX_DOCUMENT_PATH_BYTES: usize = 4096;
pub const MAX_DOCUMENT_NAME_BYTES: usize = 255;
pub const MAX_CONTENT_TYPE_BYTES: usize = 128;
pub const MAX_DOCUMENT_CONTENT_TYPES: usize = 8;

pub const COMMAND_BLUETOOTH_START_SCAN: u32 = 1;
pub const COMMAND_BLUETOOTH_STOP_SCAN: u32 = 2;
pub const COMMAND_BLUETOOTH_CONNECT: u32 = 3;
pub const COMMAND_BLUETOOTH_DISCONNECT: u32 = 4;
pub const COMMAND_BLUETOOTH_WRITE: u32 = 5;
pub const COMMAND_LOCATION_START: u32 = 6;
pub const COMMAND_LOCATION_STOP: u32 = 7;
pub const COMMAND_SET_BACKGROUND_LOGGING: u32 = 8;
pub const COMMAND_PRESENT_PERMISSION: u32 = 9;
pub const COMMAND_PRESENT_SETTINGS: u32 = 10;
pub const COMMAND_RUNTIME_READY: u32 = 11;
pub const COMMAND_PRESENT_IMPORT: u32 = 12;
pub const COMMAND_PRESENT_EXPORT: u32 = 13;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortError {
    InvalidVersion,
    InvalidTag,
    InvalidValue,
    InvalidUtf8,
    Truncated,
    PayloadTooLarge,
    TraversalPath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BluetoothAuthorization {
    Unknown,
    Denied,
    Granted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BluetoothRadio {
    Unknown,
    PoweredOff,
    PoweredOn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BluetoothConnectionState {
    Connecting,
    Ready,
    Disconnected,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocationAuthorization {
    Unknown,
    Denied,
    Foreground,
    Background,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionKind {
    Bluetooth,
    Location,
    BackgroundLocation,
    Notifications,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostCommand {
    BluetoothStartScan,
    BluetoothStopScan,
    BluetoothConnect {
        connection_generation: u64,
        association: PlatformAssociation,
    },
    BluetoothDisconnect {
        connection_generation: u64,
    },
    BluetoothWrite {
        connection_generation: u64,
        bytes: Vec<u8>,
    },
    LocationStart {
        background: bool,
    },
    LocationStop,
    SetBackgroundLogging {
        enabled: bool,
    },
    PresentPermission {
        permission: PermissionKind,
    },
    PresentSettings,
    RuntimeReady {
        endpoint: String,
        launch_token: String,
    },
    PresentImport {
        staging_path: String,
        allowed_content_types: Vec<String>,
        maximum_bytes: u64,
    },
    PresentExport {
        staging_path: String,
        display_name: String,
        content_type: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformCommand {
    pub generation: RuntimeGeneration,
    pub operation: OperationId,
    pub command: HostCommand,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AdapterEvent {
    BluetoothState {
        radio: BluetoothRadio,
        authorization: BluetoothAuthorization,
    },
    BluetoothDevice {
        association: PlatformAssociation,
        name: Option<String>,
        rssi: i16,
    },
    BluetoothConnection {
        connection_generation: u64,
        state: BluetoothConnectionState,
    },
    LocationAuthorization(LocationAuthorization),
    Location {
        latitude: f64,
        longitude: f64,
        accuracy_m: Option<f64>,
    },
    Lifecycle {
        foreground: bool,
        background_eligible: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformEvent {
    pub generation: RuntimeGeneration,
    pub connection_generation: u64,
    pub adapter_order: u64,
    pub event: AdapterEvent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionStatus {
    Success,
    Cancelled,
    PermissionDenied,
    Unavailable,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCompletion {
    pub generation: RuntimeGeneration,
    pub operation: OperationId,
    pub status: CompletionStatus,
    pub payload: Vec<u8>,
}

/// Focused raw-byte ingress for the high-frequency BLE notification path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BluetoothNotification {
    pub generation: RuntimeGeneration,
    pub connection_generation: u64,
    pub adapter_order: u64,
    pub bytes: Vec<u8>,
}

impl BluetoothNotification {
    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn validate(&self) -> Result<(), PortError> {
        if self.bytes.is_empty() || self.bytes.len() > MAX_BLUETOOTH_BYTES {
            Err(PortError::PayloadTooLarge)
        } else {
            Ok(())
        }
    }
}

impl HostCommand {
    #[must_use]
    pub const fn kind(&self) -> u32 {
        match self {
            Self::BluetoothStartScan => COMMAND_BLUETOOTH_START_SCAN,
            Self::BluetoothStopScan => COMMAND_BLUETOOTH_STOP_SCAN,
            Self::BluetoothConnect { .. } => COMMAND_BLUETOOTH_CONNECT,
            Self::BluetoothDisconnect { .. } => COMMAND_BLUETOOTH_DISCONNECT,
            Self::BluetoothWrite { .. } => COMMAND_BLUETOOTH_WRITE,
            Self::LocationStart { .. } => COMMAND_LOCATION_START,
            Self::LocationStop => COMMAND_LOCATION_STOP,
            Self::SetBackgroundLogging { .. } => COMMAND_SET_BACKGROUND_LOGGING,
            Self::PresentPermission { .. } => COMMAND_PRESENT_PERMISSION,
            Self::PresentSettings => COMMAND_PRESENT_SETTINGS,
            Self::RuntimeReady { .. } => COMMAND_RUNTIME_READY,
            Self::PresentImport { .. } => COMMAND_PRESENT_IMPORT,
            Self::PresentExport { .. } => COMMAND_PRESENT_EXPORT,
        }
    }

    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn encode_payload(&self) -> Result<Vec<u8>, PortError> {
        validate_command(self)?;
        let mut writer = Writer::new();
        encode_command_payload(self, &mut writer)?;
        Ok(writer.finish())
    }

    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn decode_payload(kind: u32, bytes: &[u8]) -> Result<Self, PortError> {
        let mut reader = Reader::new(bytes)?;
        let command = decode_command_payload(kind, &mut reader)?;
        reader.finish()?;
        validate_command(&command)?;
        Ok(command)
    }
}

impl PlatformEvent {
    /// Validates bounded fields before a focused event crosses into the actor.
    ///
    /// # Errors
    /// Returns `PortError` when an event field is invalid or exceeds its bound.
    pub fn validate(&self) -> Result<(), PortError> {
        validate_event(&self.event)
    }
}

impl OperationCompletion {
    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn encode(&self) -> Result<Vec<u8>, PortError> {
        if self.payload.len() > MAX_COMPLETION_PAYLOAD_BYTES {
            return Err(PortError::PayloadTooLarge);
        }
        let mut writer = Writer::new();
        writer.u8(BRIDGE_CODEC_VERSION)?;
        writer.u64(self.generation.0)?;
        writer.u64(self.operation.0)?;
        writer.u8(completion_tag(self.status))?;
        writer.bytes(&self.payload)?;
        Ok(writer.finish())
    }

    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn decode(bytes: &[u8]) -> Result<Self, PortError> {
        let mut reader = Reader::new(bytes)?;
        reader.version()?;
        let generation = RuntimeGeneration(reader.u64()?);
        let operation = OperationId(reader.u64()?);
        let status = decode_completion(reader.u8()?)?;
        let payload = reader.bytes(MAX_COMPLETION_PAYLOAD_BYTES)?;
        reader.finish()?;
        Ok(Self {
            generation,
            operation,
            status,
            payload,
        })
    }
}

fn validate_command(command: &HostCommand) -> Result<(), PortError> {
    match command {
        HostCommand::BluetoothConnect { association, .. } => bounded_bytes(
            association.as_bytes(),
            domain_types::MAX_PLATFORM_ASSOCIATION_BYTES,
        ),
        HostCommand::BluetoothWrite { bytes, .. } => {
            bounded_bytes(bytes, MAX_BLUETOOTH_WRITE_BYTES)
        }
        HostCommand::RuntimeReady {
            endpoint,
            launch_token,
        } => {
            bounded_string(endpoint, MAX_ENDPOINT_BYTES)?;
            bounded_string(launch_token, MAX_LAUNCH_TOKEN_BYTES)
        }
        HostCommand::PresentImport {
            staging_path,
            allowed_content_types,
            maximum_bytes,
        } => {
            validate_document_path(staging_path)?;
            if *maximum_bytes == 0
                || allowed_content_types.is_empty()
                || allowed_content_types.len() > MAX_DOCUMENT_CONTENT_TYPES
            {
                return Err(PortError::InvalidValue);
            }
            for content_type in allowed_content_types {
                bounded_string(content_type, MAX_CONTENT_TYPE_BYTES)?;
            }
            Ok(())
        }
        HostCommand::PresentExport {
            staging_path,
            display_name,
            content_type,
        } => {
            validate_document_path(staging_path)?;
            bounded_string(display_name, MAX_DOCUMENT_NAME_BYTES)?;
            if display_name.contains('/') || display_name.contains('\\') {
                return Err(PortError::TraversalPath);
            }
            bounded_string(content_type, MAX_CONTENT_TYPE_BYTES)
        }
        _ => Ok(()),
    }
}

fn validate_document_path(value: &str) -> Result<(), PortError> {
    bounded_string(value, MAX_DOCUMENT_PATH_BYTES)?;
    let path = std::path::Path::new(value);
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        Err(PortError::TraversalPath)
    } else {
        Ok(())
    }
}

fn validate_event(event: &AdapterEvent) -> Result<(), PortError> {
    match event {
        AdapterEvent::BluetoothDevice {
            association, name, ..
        } => {
            bounded_bytes(
                association.as_bytes(),
                domain_types::MAX_PLATFORM_ASSOCIATION_BYTES,
            )?;
            if let Some(name) = name {
                bounded_string(name, MAX_LOGICAL_NAME_BYTES)?;
            }
            Ok(())
        }
        AdapterEvent::Location {
            latitude,
            longitude,
            accuracy_m,
        } => {
            if !latitude.is_finite()
                || !longitude.is_finite()
                || !(-90.0..=90.0).contains(latitude)
                || !(-180.0..=180.0).contains(longitude)
                || accuracy_m.is_some_and(|accuracy| !accuracy.is_finite() || accuracy < 0.0)
            {
                return Err(PortError::InvalidValue);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn bounded_string(value: &str, maximum: usize) -> Result<(), PortError> {
    if value.is_empty() || value.len() > maximum || value.as_bytes().contains(&0) {
        Err(PortError::InvalidValue)
    } else {
        Ok(())
    }
}

fn bounded_bytes(value: &[u8], maximum: usize) -> Result<(), PortError> {
    if value.is_empty() || value.len() > maximum {
        Err(PortError::PayloadTooLarge)
    } else {
        Ok(())
    }
}

fn encode_command_payload(command: &HostCommand, writer: &mut Writer) -> Result<(), PortError> {
    match command {
        HostCommand::BluetoothStartScan
        | HostCommand::BluetoothStopScan
        | HostCommand::LocationStop
        | HostCommand::PresentSettings => Ok(()),
        HostCommand::BluetoothConnect {
            connection_generation,
            association,
        } => {
            writer.u64(*connection_generation)?;
            writer.bytes(association.as_bytes())
        }
        HostCommand::BluetoothDisconnect {
            connection_generation,
        } => writer.u64(*connection_generation),
        HostCommand::BluetoothWrite {
            connection_generation,
            bytes,
        } => {
            writer.u64(*connection_generation)?;
            writer.bytes(bytes)
        }
        HostCommand::LocationStart { background } => writer.boolean(*background),
        HostCommand::SetBackgroundLogging { enabled } => writer.boolean(*enabled),
        HostCommand::PresentPermission { permission } => writer.u8(permission_tag(*permission)),
        HostCommand::RuntimeReady {
            endpoint,
            launch_token,
        } => {
            writer.string(endpoint)?;
            writer.string(launch_token)
        }
        HostCommand::PresentImport {
            staging_path,
            allowed_content_types,
            maximum_bytes,
        } => {
            writer.string(staging_path)?;
            writer
                .u8(u8::try_from(allowed_content_types.len())
                    .map_err(|_| PortError::InvalidValue)?)?;
            for content_type in allowed_content_types {
                writer.string(content_type)?;
            }
            writer.u64(*maximum_bytes)
        }
        HostCommand::PresentExport {
            staging_path,
            display_name,
            content_type,
        } => {
            writer.string(staging_path)?;
            writer.string(display_name)?;
            writer.string(content_type)
        }
    }
}

fn decode_command_payload(kind: u32, reader: &mut Reader<'_>) -> Result<HostCommand, PortError> {
    Ok(match kind {
        COMMAND_BLUETOOTH_START_SCAN => HostCommand::BluetoothStartScan,
        COMMAND_BLUETOOTH_STOP_SCAN => HostCommand::BluetoothStopScan,
        COMMAND_BLUETOOTH_CONNECT => HostCommand::BluetoothConnect {
            connection_generation: reader.u64()?,
            association: PlatformAssociation::new(
                reader.bytes(domain_types::MAX_PLATFORM_ASSOCIATION_BYTES)?,
            )
            .map_err(|_| PortError::InvalidValue)?,
        },
        COMMAND_BLUETOOTH_DISCONNECT => HostCommand::BluetoothDisconnect {
            connection_generation: reader.u64()?,
        },
        COMMAND_BLUETOOTH_WRITE => HostCommand::BluetoothWrite {
            connection_generation: reader.u64()?,
            bytes: reader.bytes(MAX_BLUETOOTH_WRITE_BYTES)?,
        },
        COMMAND_LOCATION_START => HostCommand::LocationStart {
            background: reader.boolean()?,
        },
        COMMAND_LOCATION_STOP => HostCommand::LocationStop,
        COMMAND_SET_BACKGROUND_LOGGING => HostCommand::SetBackgroundLogging {
            enabled: reader.boolean()?,
        },
        COMMAND_PRESENT_PERMISSION => HostCommand::PresentPermission {
            permission: decode_permission(reader.u8()?)?,
        },
        COMMAND_PRESENT_SETTINGS => HostCommand::PresentSettings,
        COMMAND_RUNTIME_READY => HostCommand::RuntimeReady {
            endpoint: reader.string(MAX_ENDPOINT_BYTES)?,
            launch_token: reader.string(MAX_LAUNCH_TOKEN_BYTES)?,
        },
        COMMAND_PRESENT_IMPORT => {
            let staging_path = reader.string(MAX_DOCUMENT_PATH_BYTES)?;
            let count = usize::from(reader.u8()?);
            if count == 0 || count > MAX_DOCUMENT_CONTENT_TYPES {
                return Err(PortError::InvalidValue);
            }
            let mut allowed_content_types = Vec::with_capacity(count);
            for _ in 0..count {
                allowed_content_types.push(reader.string(MAX_CONTENT_TYPE_BYTES)?);
            }
            HostCommand::PresentImport {
                staging_path,
                allowed_content_types,
                maximum_bytes: reader.u64()?,
            }
        }
        COMMAND_PRESENT_EXPORT => HostCommand::PresentExport {
            staging_path: reader.string(MAX_DOCUMENT_PATH_BYTES)?,
            display_name: reader.string(MAX_DOCUMENT_NAME_BYTES)?,
            content_type: reader.string(MAX_CONTENT_TYPE_BYTES)?,
        },
        _ => return Err(PortError::InvalidTag),
    })
}

macro_rules! tagged_enum {
    ($encode:ident, $decode:ident, $ty:ty, {$($variant:path => $tag:literal),+ $(,)?}) => {
        const fn $encode(value: $ty) -> u8 { match value { $($variant => $tag),+ } }
        fn $decode(value: u8) -> Result<$ty, PortError> { match value { $($tag => Ok($variant),)+ _ => Err(PortError::InvalidTag) } }
    };
}

tagged_enum!(permission_tag, decode_permission, PermissionKind, {PermissionKind::Bluetooth => 1, PermissionKind::Location => 2, PermissionKind::BackgroundLocation => 3, PermissionKind::Notifications => 4});
tagged_enum!(completion_tag, decode_completion, CompletionStatus, {CompletionStatus::Success => 1, CompletionStatus::Cancelled => 2, CompletionStatus::PermissionDenied => 3, CompletionStatus::Unavailable => 4, CompletionStatus::Failed => 5});

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }
    fn reserve(&self, count: usize) -> Result<(), PortError> {
        if self.bytes.len().saturating_add(count) > MAX_BRIDGE_PAYLOAD_BYTES {
            Err(PortError::PayloadTooLarge)
        } else {
            Ok(())
        }
    }
    fn u8(&mut self, value: u8) -> Result<(), PortError> {
        self.reserve(1)?;
        self.bytes.push(value);
        Ok(())
    }
    fn u64(&mut self, value: u64) -> Result<(), PortError> {
        self.raw(&value.to_be_bytes())
    }
    fn boolean(&mut self, value: bool) -> Result<(), PortError> {
        self.u8(u8::from(value))
    }
    fn bytes(&mut self, value: &[u8]) -> Result<(), PortError> {
        let length = u32::try_from(value.len()).map_err(|_| PortError::PayloadTooLarge)?;
        self.raw(&length.to_be_bytes())?;
        self.raw(value)
    }
    fn string(&mut self, value: &str) -> Result<(), PortError> {
        self.bytes(value.as_bytes())
    }
    fn raw(&mut self, value: &[u8]) -> Result<(), PortError> {
        self.reserve(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, PortError> {
        if bytes.len() > MAX_BRIDGE_PAYLOAD_BYTES {
            Err(PortError::PayloadTooLarge)
        } else {
            Ok(Self { bytes, offset: 0 })
        }
    }
    fn version(&mut self) -> Result<(), PortError> {
        if self.u8()? == BRIDGE_CODEC_VERSION {
            Ok(())
        } else {
            Err(PortError::InvalidVersion)
        }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], PortError> {
        let end = self.offset.checked_add(count).ok_or(PortError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(PortError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, PortError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, PortError> {
        let value = self.take(4)?;
        Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
    }
    fn u64(&mut self) -> Result<u64, PortError> {
        let value = self.take(8)?;
        Ok(u64::from_be_bytes([
            value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
        ]))
    }
    fn boolean(&mut self) -> Result<bool, PortError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(PortError::InvalidValue),
        }
    }
    fn bytes(&mut self, maximum: usize) -> Result<Vec<u8>, PortError> {
        let count = usize::try_from(self.u32()?).map_err(|_| PortError::PayloadTooLarge)?;
        if count > maximum {
            return Err(PortError::PayloadTooLarge);
        }
        Ok(self.take(count)?.to_vec())
    }
    fn string(&mut self, maximum: usize) -> Result<String, PortError> {
        String::from_utf8(self.bytes(maximum)?).map_err(|_| PortError::InvalidUtf8)
    }
    fn finish(self) -> Result<(), PortError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PortError::InvalidValue)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_family_round_trips_with_fixed_context() {
        let commands = vec![
            HostCommand::BluetoothStartScan,
            HostCommand::BluetoothStopScan,
            HostCommand::BluetoothConnect {
                connection_generation: 9,
                association: PlatformAssociation::new(vec![1, 2, 3]).unwrap(),
            },
            HostCommand::BluetoothDisconnect {
                connection_generation: 9,
            },
            HostCommand::BluetoothWrite {
                connection_generation: 9,
                bytes: vec![2, 3, 4],
            },
            HostCommand::LocationStart { background: true },
            HostCommand::LocationStop,
            HostCommand::SetBackgroundLogging { enabled: true },
            HostCommand::PresentPermission {
                permission: PermissionKind::Bluetooth,
            },
            HostCommand::PresentSettings,
            HostCommand::RuntimeReady {
                endpoint: "http://127.0.0.1:1234".into(),
                launch_token: "opaque-token".into(),
            },
            HostCommand::PresentImport {
                staging_path: "/tmp/float-view/import.floatview.tmp".into(),
                allowed_content_types: vec!["application/octet-stream".into()],
                maximum_bytes: 128 * 1024 * 1024,
            },
            HostCommand::PresentExport {
                staging_path: "/tmp/float-view/export.floatview".into(),
                display_name: "ride.floatview".into(),
                content_type: "application/octet-stream".into(),
            },
        ];
        for command in commands {
            let kind = command.kind();
            let payload = command.encode_payload().expect("encode");
            assert_eq!(HostCommand::decode_payload(kind, &payload), Ok(command));
        }
    }

    #[test]
    fn adapter_events_completion_and_raw_bluetooth_round_trip() {
        let events = vec![
            AdapterEvent::BluetoothState {
                radio: BluetoothRadio::PoweredOn,
                authorization: BluetoothAuthorization::Granted,
            },
            AdapterEvent::BluetoothDevice {
                association: PlatformAssociation::new(vec![9]).unwrap(),
                name: Some("Board".into()),
                rssi: -42,
            },
            AdapterEvent::BluetoothConnection {
                connection_generation: 4,
                state: BluetoothConnectionState::Ready,
            },
            AdapterEvent::LocationAuthorization(LocationAuthorization::Background),
            AdapterEvent::Location {
                latitude: -33.8,
                longitude: 151.2,
                accuracy_m: Some(3.0),
            },
            AdapterEvent::Lifecycle {
                foreground: false,
                background_eligible: true,
            },
        ];
        for event in events {
            let value = PlatformEvent {
                generation: RuntimeGeneration(2),
                connection_generation: 3,
                adapter_order: 8,
                event,
            };
            assert_eq!(value.validate(), Ok(()));
        }
        let completion = OperationCompletion {
            generation: RuntimeGeneration(2),
            operation: OperationId(3),
            status: CompletionStatus::Success,
            payload: vec![1, 2],
        };
        assert_eq!(
            OperationCompletion::decode(&completion.encode().expect("encode")),
            Ok(completion)
        );
        assert_eq!(
            BluetoothNotification {
                generation: RuntimeGeneration(1),
                connection_generation: 2,
                adapter_order: 3,
                bytes: vec![1]
            }
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn bounds_stale_context_fields_and_malformed_codecs_are_explicit() {
        assert_eq!(
            HostCommand::decode_payload(0, &[]),
            Err(PortError::InvalidTag)
        );
        assert_eq!(
            HostCommand::decode_payload(COMMAND_BLUETOOTH_START_SCAN, &[2]),
            Err(PortError::InvalidValue)
        );
        assert_eq!(
            BluetoothNotification {
                generation: RuntimeGeneration(1),
                connection_generation: 1,
                adapter_order: 1,
                bytes: vec![0; MAX_BLUETOOTH_BYTES + 1]
            }
            .validate(),
            Err(PortError::PayloadTooLarge)
        );
        assert_eq!(
            HostCommand::PresentImport {
                staging_path: "../import.floatview".into(),
                allowed_content_types: vec!["application/octet-stream".into()],
                maximum_bytes: 1,
            }
            .encode_payload(),
            Err(PortError::TraversalPath)
        );
        assert_eq!(
            HostCommand::PresentExport {
                staging_path: "/tmp/export.csv".into(),
                display_name: "../export.csv".into(),
                content_type: "text/csv".into(),
            }
            .encode_payload(),
            Err(PortError::TraversalPath)
        );
        assert_eq!(
            HostCommand::PresentImport {
                staging_path: "/tmp/import.floatview".into(),
                allowed_content_types: Vec::new(),
                maximum_bytes: 1,
            }
            .encode_payload(),
            Err(PortError::InvalidValue)
        );
    }
}
