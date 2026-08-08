//! Typed, bounded platform-port vocabulary. No OS implementation lives here.
use domain_types::{OperationId, RuntimeGeneration};

pub const BRIDGE_CODEC_VERSION: u8 = 1;
pub const MAX_BRIDGE_PAYLOAD_BYTES: usize = 64 * 1024;
pub const MAX_COMPLETION_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BLUETOOTH_BYTES: usize = 4096;
pub const MAX_BLUETOOTH_WRITE_BYTES: usize = MAX_BRIDGE_PAYLOAD_BYTES;
pub const MAX_RESOURCE_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_DOCUMENT_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_RANDOM_BYTES: u16 = 256;
pub const MAX_LOGICAL_NAME_BYTES: usize = 128;
pub const MAX_PATH_BYTES: usize = 1024;
pub const MAX_DIAGNOSTIC_BYTES: usize = 64;
pub const MAX_ENDPOINT_BYTES: usize = 256;
pub const MAX_LAUNCH_TOKEN_BYTES: usize = 128;

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
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionKind {
    Bluetooth,
    Location,
    BackgroundLocation,
    Notifications,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceNamespace {
    Frontend,
    Sql,
    SoftwareFixture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageRootKind {
    Durable,
    Cache,
    Temporary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostCommand {
    BluetoothStartScan,
    BluetoothStopScan,
    BluetoothConnect {
        device_token: Vec<u8>,
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
    ReadResource {
        namespace: ResourceNamespace,
        logical_name: String,
        maximum_bytes: u64,
    },
    StageDocument {
        destination: String,
        maximum_bytes: u64,
    },
    ResolveStorageRoot {
        kind: StorageRootKind,
    },
    AvailableSpace {
        root: String,
    },
    MonotonicClock,
    WallClock,
    SecureRandom {
        count: u16,
    },
    Diagnostic {
        stage: String,
        code: String,
    },
    PresentPermission {
        permission: PermissionKind,
    },
    PresentSettings,
    PresentTheme {
        theme: ThemePreference,
    },
    PresentServiceState {
        active: bool,
    },
    PresentExport {
        staged_path: String,
    },
    RuntimeReady {
        endpoint: String,
        launch_token: String,
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
        device_token: Vec<u8>,
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
    LoggingService {
        active: bool,
    },
    Theme(ThemePreference),
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

impl PlatformCommand {
    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn encode(&self) -> Result<Vec<u8>, PortError> {
        validate_command(&self.command)?;
        let mut writer = Writer::new();
        writer.u8(BRIDGE_CODEC_VERSION)?;
        writer.u64(self.generation.0)?;
        writer.u64(self.operation.0)?;
        encode_command(&self.command, &mut writer)?;
        Ok(writer.finish())
    }

    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn decode(bytes: &[u8]) -> Result<Self, PortError> {
        let mut reader = Reader::new(bytes)?;
        reader.version()?;
        let generation = RuntimeGeneration(reader.u64()?);
        let operation = OperationId(reader.u64()?);
        let command = decode_command(&mut reader)?;
        reader.finish()?;
        validate_command(&command)?;
        Ok(Self {
            generation,
            operation,
            command,
        })
    }
}

impl PlatformEvent {
    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn encode(&self) -> Result<Vec<u8>, PortError> {
        validate_event(&self.event)?;
        let mut writer = Writer::new();
        writer.u8(BRIDGE_CODEC_VERSION)?;
        writer.u64(self.generation.0)?;
        writer.u64(self.connection_generation)?;
        writer.u64(self.adapter_order)?;
        encode_event(&self.event, &mut writer)?;
        Ok(writer.finish())
    }

    /// # Errors
    /// Returns `PortError` when validation or bounded codec processing fails.
    pub fn decode(bytes: &[u8]) -> Result<Self, PortError> {
        let mut reader = Reader::new(bytes)?;
        reader.version()?;
        let generation = RuntimeGeneration(reader.u64()?);
        let connection_generation = reader.u64()?;
        let adapter_order = reader.u64()?;
        let event = decode_event(&mut reader)?;
        reader.finish()?;
        validate_event(&event)?;
        Ok(Self {
            generation,
            connection_generation,
            adapter_order,
            event,
        })
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
        HostCommand::BluetoothConnect { device_token } => bounded_bytes(device_token, 256),
        HostCommand::BluetoothWrite { bytes, .. } => {
            bounded_bytes(bytes, MAX_BLUETOOTH_WRITE_BYTES)
        }
        HostCommand::ReadResource {
            logical_name,
            maximum_bytes,
            ..
        } => {
            logical_name_valid(logical_name)?;
            if *maximum_bytes == 0 || *maximum_bytes > MAX_RESOURCE_BYTES {
                return Err(PortError::InvalidValue);
            }
            Ok(())
        }
        HostCommand::StageDocument {
            destination,
            maximum_bytes,
        } => {
            bounded_string(destination, MAX_PATH_BYTES)?;
            if *maximum_bytes == 0 || *maximum_bytes > MAX_DOCUMENT_BYTES {
                return Err(PortError::InvalidValue);
            }
            Ok(())
        }
        HostCommand::AvailableSpace { root } => bounded_string(root, MAX_PATH_BYTES),
        HostCommand::SecureRandom { count } if *count == 0 || *count > MAX_RANDOM_BYTES => {
            Err(PortError::InvalidValue)
        }
        HostCommand::Diagnostic { stage, code } => {
            diagnostic_value(stage)?;
            diagnostic_value(code)
        }
        HostCommand::PresentExport { staged_path } => bounded_string(staged_path, MAX_PATH_BYTES),
        HostCommand::RuntimeReady {
            endpoint,
            launch_token,
        } => {
            bounded_string(endpoint, MAX_ENDPOINT_BYTES)?;
            bounded_string(launch_token, MAX_LAUNCH_TOKEN_BYTES)
        }
        _ => Ok(()),
    }
}

fn validate_event(event: &AdapterEvent) -> Result<(), PortError> {
    match event {
        AdapterEvent::BluetoothDevice {
            device_token, name, ..
        } => {
            bounded_bytes(device_token, 256)?;
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

fn logical_name_valid(value: &str) -> Result<(), PortError> {
    bounded_string(value, MAX_LOGICAL_NAME_BYTES)?;
    if value.starts_with('/')
        || value.starts_with('\\')
        || value.contains("..")
        || value.contains(':')
        || value.split('/').any(|part| part.is_empty() || part == ".")
    {
        Err(PortError::TraversalPath)
    } else if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"-_./".contains(&byte))
    {
        Ok(())
    } else {
        Err(PortError::InvalidValue)
    }
}

fn diagnostic_value(value: &str) -> Result<(), PortError> {
    bounded_string(value, MAX_DIAGNOSTIC_BYTES)?;
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        Ok(())
    } else {
        Err(PortError::InvalidValue)
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

fn encode_command(command: &HostCommand, writer: &mut Writer) -> Result<(), PortError> {
    match command {
        HostCommand::BluetoothStartScan => writer.u8(1),
        HostCommand::BluetoothStopScan => writer.u8(2),
        HostCommand::BluetoothConnect { device_token } => {
            writer.u8(3)?;
            writer.bytes(device_token)
        }
        HostCommand::BluetoothDisconnect {
            connection_generation,
        } => {
            writer.u8(4)?;
            writer.u64(*connection_generation)
        }
        HostCommand::BluetoothWrite {
            connection_generation,
            bytes,
        } => {
            writer.u8(5)?;
            writer.u64(*connection_generation)?;
            writer.bytes(bytes)
        }
        HostCommand::LocationStart { background } => {
            writer.u8(6)?;
            writer.boolean(*background)
        }
        HostCommand::LocationStop => writer.u8(7),
        HostCommand::SetBackgroundLogging { enabled } => {
            writer.u8(8)?;
            writer.boolean(*enabled)
        }
        HostCommand::ReadResource {
            namespace,
            logical_name,
            maximum_bytes,
        } => {
            writer.u8(9)?;
            writer.u8(resource_tag(*namespace))?;
            writer.string(logical_name)?;
            writer.u64(*maximum_bytes)
        }
        HostCommand::StageDocument {
            destination,
            maximum_bytes,
        } => {
            writer.u8(10)?;
            writer.string(destination)?;
            writer.u64(*maximum_bytes)
        }
        HostCommand::ResolveStorageRoot { kind } => {
            writer.u8(11)?;
            writer.u8(storage_tag(*kind))
        }
        HostCommand::AvailableSpace { root } => {
            writer.u8(12)?;
            writer.string(root)
        }
        HostCommand::MonotonicClock => writer.u8(13),
        HostCommand::WallClock => writer.u8(14),
        HostCommand::SecureRandom { count } => {
            writer.u8(15)?;
            writer.u16(*count)
        }
        HostCommand::Diagnostic { stage, code } => {
            writer.u8(16)?;
            writer.string(stage)?;
            writer.string(code)
        }
        HostCommand::PresentPermission { permission } => {
            writer.u8(17)?;
            writer.u8(permission_tag(*permission))
        }
        HostCommand::PresentSettings => writer.u8(18),
        HostCommand::PresentTheme { theme } => {
            writer.u8(19)?;
            writer.u8(theme_tag(*theme))
        }
        HostCommand::PresentServiceState { active } => {
            writer.u8(20)?;
            writer.boolean(*active)
        }
        HostCommand::PresentExport { staged_path } => {
            writer.u8(21)?;
            writer.string(staged_path)
        }
        HostCommand::RuntimeReady {
            endpoint,
            launch_token,
        } => {
            writer.u8(22)?;
            writer.string(endpoint)?;
            writer.string(launch_token)
        }
    }
}

fn decode_command(reader: &mut Reader<'_>) -> Result<HostCommand, PortError> {
    Ok(match reader.u8()? {
        1 => HostCommand::BluetoothStartScan,
        2 => HostCommand::BluetoothStopScan,
        3 => HostCommand::BluetoothConnect {
            device_token: reader.bytes(256)?,
        },
        4 => HostCommand::BluetoothDisconnect {
            connection_generation: reader.u64()?,
        },
        5 => HostCommand::BluetoothWrite {
            connection_generation: reader.u64()?,
            bytes: reader.bytes(MAX_BLUETOOTH_WRITE_BYTES)?,
        },
        6 => HostCommand::LocationStart {
            background: reader.boolean()?,
        },
        7 => HostCommand::LocationStop,
        8 => HostCommand::SetBackgroundLogging {
            enabled: reader.boolean()?,
        },
        9 => HostCommand::ReadResource {
            namespace: decode_resource(reader.u8()?)?,
            logical_name: reader.string(MAX_LOGICAL_NAME_BYTES)?,
            maximum_bytes: reader.u64()?,
        },
        10 => HostCommand::StageDocument {
            destination: reader.string(MAX_PATH_BYTES)?,
            maximum_bytes: reader.u64()?,
        },
        11 => HostCommand::ResolveStorageRoot {
            kind: decode_storage(reader.u8()?)?,
        },
        12 => HostCommand::AvailableSpace {
            root: reader.string(MAX_PATH_BYTES)?,
        },
        13 => HostCommand::MonotonicClock,
        14 => HostCommand::WallClock,
        15 => HostCommand::SecureRandom {
            count: reader.u16()?,
        },
        16 => HostCommand::Diagnostic {
            stage: reader.string(MAX_DIAGNOSTIC_BYTES)?,
            code: reader.string(MAX_DIAGNOSTIC_BYTES)?,
        },
        17 => HostCommand::PresentPermission {
            permission: decode_permission(reader.u8()?)?,
        },
        18 => HostCommand::PresentSettings,
        19 => HostCommand::PresentTheme {
            theme: decode_theme(reader.u8()?)?,
        },
        20 => HostCommand::PresentServiceState {
            active: reader.boolean()?,
        },
        21 => HostCommand::PresentExport {
            staged_path: reader.string(MAX_PATH_BYTES)?,
        },
        22 => HostCommand::RuntimeReady {
            endpoint: reader.string(MAX_ENDPOINT_BYTES)?,
            launch_token: reader.string(MAX_LAUNCH_TOKEN_BYTES)?,
        },
        _ => return Err(PortError::InvalidTag),
    })
}

fn encode_event(event: &AdapterEvent, writer: &mut Writer) -> Result<(), PortError> {
    match event {
        AdapterEvent::BluetoothState {
            radio,
            authorization,
        } => {
            writer.u8(1)?;
            writer.u8(radio_tag(*radio))?;
            writer.u8(bluetooth_authorization_tag(*authorization))
        }
        AdapterEvent::BluetoothDevice {
            device_token,
            name,
            rssi,
        } => {
            writer.u8(2)?;
            writer.bytes(device_token)?;
            writer.optional_string(name.as_deref())?;
            writer.i16(*rssi)
        }
        AdapterEvent::BluetoothConnection {
            connection_generation,
            state,
        } => {
            writer.u8(3)?;
            writer.u64(*connection_generation)?;
            writer.u8(connection_tag(*state))
        }
        AdapterEvent::LocationAuthorization(value) => {
            writer.u8(4)?;
            writer.u8(location_authorization_tag(*value))
        }
        AdapterEvent::Location {
            latitude,
            longitude,
            accuracy_m,
        } => {
            writer.u8(5)?;
            writer.f64(*latitude)?;
            writer.f64(*longitude)?;
            writer.optional_f64(*accuracy_m)
        }
        AdapterEvent::Lifecycle {
            foreground,
            background_eligible,
        } => {
            writer.u8(6)?;
            writer.boolean(*foreground)?;
            writer.boolean(*background_eligible)
        }
        AdapterEvent::LoggingService { active } => {
            writer.u8(7)?;
            writer.boolean(*active)
        }
        AdapterEvent::Theme(theme) => {
            writer.u8(8)?;
            writer.u8(theme_tag(*theme))
        }
    }
}

fn decode_event(reader: &mut Reader<'_>) -> Result<AdapterEvent, PortError> {
    Ok(match reader.u8()? {
        1 => AdapterEvent::BluetoothState {
            radio: decode_radio(reader.u8()?)?,
            authorization: decode_bluetooth_authorization(reader.u8()?)?,
        },
        2 => AdapterEvent::BluetoothDevice {
            device_token: reader.bytes(256)?,
            name: reader.optional_string(MAX_LOGICAL_NAME_BYTES)?,
            rssi: reader.i16()?,
        },
        3 => AdapterEvent::BluetoothConnection {
            connection_generation: reader.u64()?,
            state: decode_connection(reader.u8()?)?,
        },
        4 => AdapterEvent::LocationAuthorization(decode_location_authorization(reader.u8()?)?),
        5 => AdapterEvent::Location {
            latitude: reader.f64()?,
            longitude: reader.f64()?,
            accuracy_m: reader.optional_f64()?,
        },
        6 => AdapterEvent::Lifecycle {
            foreground: reader.boolean()?,
            background_eligible: reader.boolean()?,
        },
        7 => AdapterEvent::LoggingService {
            active: reader.boolean()?,
        },
        8 => AdapterEvent::Theme(decode_theme(reader.u8()?)?),
        _ => return Err(PortError::InvalidTag),
    })
}

macro_rules! tagged_enum {
    ($encode:ident, $decode:ident, $ty:ty, {$($variant:path => $tag:literal),+ $(,)?}) => {
        const fn $encode(value: $ty) -> u8 { match value { $($variant => $tag),+ } }
        fn $decode(value: u8) -> Result<$ty, PortError> { match value { $($tag => Ok($variant),)+ _ => Err(PortError::InvalidTag) } }
    };
}

tagged_enum!(resource_tag, decode_resource, ResourceNamespace, {ResourceNamespace::Frontend => 1, ResourceNamespace::Sql => 2, ResourceNamespace::SoftwareFixture => 3});
tagged_enum!(storage_tag, decode_storage, StorageRootKind, {StorageRootKind::Durable => 1, StorageRootKind::Cache => 2, StorageRootKind::Temporary => 3});
tagged_enum!(permission_tag, decode_permission, PermissionKind, {PermissionKind::Bluetooth => 1, PermissionKind::Location => 2, PermissionKind::BackgroundLocation => 3, PermissionKind::Notifications => 4});
tagged_enum!(theme_tag, decode_theme, ThemePreference, {ThemePreference::System => 1, ThemePreference::Light => 2, ThemePreference::Dark => 3});
tagged_enum!(radio_tag, decode_radio, BluetoothRadio, {BluetoothRadio::Unknown => 1, BluetoothRadio::PoweredOff => 2, BluetoothRadio::PoweredOn => 3});
tagged_enum!(bluetooth_authorization_tag, decode_bluetooth_authorization, BluetoothAuthorization, {BluetoothAuthorization::Unknown => 1, BluetoothAuthorization::Denied => 2, BluetoothAuthorization::Granted => 3});
tagged_enum!(connection_tag, decode_connection, BluetoothConnectionState, {BluetoothConnectionState::Connecting => 1, BluetoothConnectionState::Ready => 2, BluetoothConnectionState::Disconnected => 3, BluetoothConnectionState::Failed => 4});
tagged_enum!(location_authorization_tag, decode_location_authorization, LocationAuthorization, {LocationAuthorization::Unknown => 1, LocationAuthorization::Denied => 2, LocationAuthorization::Foreground => 3, LocationAuthorization::Background => 4});
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
    fn u16(&mut self, value: u16) -> Result<(), PortError> {
        self.raw(&value.to_be_bytes())
    }
    fn i16(&mut self, value: i16) -> Result<(), PortError> {
        self.raw(&value.to_be_bytes())
    }
    fn u64(&mut self, value: u64) -> Result<(), PortError> {
        self.raw(&value.to_be_bytes())
    }
    fn f64(&mut self, value: f64) -> Result<(), PortError> {
        self.u64(value.to_bits())
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
    fn optional_string(&mut self, value: Option<&str>) -> Result<(), PortError> {
        self.boolean(value.is_some())?;
        if let Some(value) = value {
            self.string(value)?;
        }
        Ok(())
    }
    fn optional_f64(&mut self, value: Option<f64>) -> Result<(), PortError> {
        self.boolean(value.is_some())?;
        if let Some(value) = value {
            self.f64(value)?;
        }
        Ok(())
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
    fn u16(&mut self) -> Result<u16, PortError> {
        let value = self.take(2)?;
        Ok(u16::from_be_bytes([value[0], value[1]]))
    }
    fn i16(&mut self) -> Result<i16, PortError> {
        let value = self.take(2)?;
        Ok(i16::from_be_bytes([value[0], value[1]]))
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
    fn f64(&mut self) -> Result<f64, PortError> {
        Ok(f64::from_bits(self.u64()?))
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
    fn optional_string(&mut self, maximum: usize) -> Result<Option<String>, PortError> {
        if self.boolean()? {
            Ok(Some(self.string(maximum)?))
        } else {
            Ok(None)
        }
    }
    fn optional_f64(&mut self) -> Result<Option<f64>, PortError> {
        if self.boolean()? {
            Ok(Some(self.f64()?))
        } else {
            Ok(None)
        }
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
                device_token: vec![1, 2, 3],
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
            HostCommand::ReadResource {
                namespace: ResourceNamespace::Frontend,
                logical_name: "assets/app.js".into(),
                maximum_bytes: 1024,
            },
            HostCommand::StageDocument {
                destination: "/staging/import.fv".into(),
                maximum_bytes: 4096,
            },
            HostCommand::ResolveStorageRoot {
                kind: StorageRootKind::Durable,
            },
            HostCommand::AvailableSpace {
                root: "/durable".into(),
            },
            HostCommand::MonotonicClock,
            HostCommand::WallClock,
            HostCommand::SecureRandom { count: 32 },
            HostCommand::Diagnostic {
                stage: "bluetooth_connect".into(),
                code: "timeout".into(),
            },
            HostCommand::PresentPermission {
                permission: PermissionKind::Bluetooth,
            },
            HostCommand::PresentSettings,
            HostCommand::PresentTheme {
                theme: ThemePreference::Dark,
            },
            HostCommand::PresentServiceState { active: true },
            HostCommand::PresentExport {
                staged_path: "/staging/export.fv".into(),
            },
            HostCommand::RuntimeReady {
                endpoint: "http://127.0.0.1:1234".into(),
                launch_token: "opaque-token".into(),
            },
        ];
        for command in commands {
            let value = PlatformCommand {
                generation: RuntimeGeneration(7),
                operation: OperationId(11),
                command,
            };
            assert_eq!(
                PlatformCommand::decode(&value.encode().expect("encode")),
                Ok(value)
            );
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
                device_token: vec![9],
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
            AdapterEvent::LoggingService { active: true },
            AdapterEvent::Theme(ThemePreference::System),
        ];
        for event in events {
            let value = PlatformEvent {
                generation: RuntimeGeneration(2),
                connection_generation: 3,
                adapter_order: 8,
                event,
            };
            assert_eq!(
                PlatformEvent::decode(&value.encode().expect("encode")),
                Ok(value)
            );
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
    fn bounds_traversal_stale_context_fields_and_malformed_codecs_are_explicit() {
        let traversal = PlatformCommand {
            generation: RuntimeGeneration(1),
            operation: OperationId(1),
            command: HostCommand::ReadResource {
                namespace: ResourceNamespace::Frontend,
                logical_name: "../secret".into(),
                maximum_bytes: 10,
            },
        };
        assert_eq!(traversal.encode(), Err(PortError::TraversalPath));
        let too_random = PlatformCommand {
            generation: RuntimeGeneration(1),
            operation: OperationId(1),
            command: HostCommand::SecureRandom {
                count: MAX_RANDOM_BYTES + 1,
            },
        };
        assert_eq!(too_random.encode(), Err(PortError::InvalidValue));
        assert_eq!(PlatformCommand::decode(&[]), Err(PortError::Truncated));
        assert_eq!(
            PlatformCommand::decode(&[2]),
            Err(PortError::InvalidVersion)
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
    }
}
