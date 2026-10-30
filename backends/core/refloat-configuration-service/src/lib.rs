//! Connection-scoped Refloat configuration lifecycle decisions.
//!
//! This safe candidate emits work for a host to execute. It owns no transport,
//! cache storage, scheduler, application publication, host ABI, or OS behavior,
//! and remains unselected until explicit host-cutover changes.

#![forbid(unsafe_code)]

use float_view_refloat_configuration_codec::{
    ConfigurationError, ConfigurationSchema, ConfigurationValue, ConfigurationValues,
    MAX_COMPRESSED_BYTES, decode_qt_compressed, decode_values, encode_values,
};

/// VESC custom-configuration package index used by Refloat.
pub const REFLOAT_INDEX: u8 = 0;
/// Maximum bytes requested in one command-92 schema page.
pub const MAX_SCHEMA_CHUNK_BYTES: usize = 400;
/// Native-equivalent scheduler timeout for one commands 92–95 request.
pub const REQUEST_TIMEOUT_MS: u64 = 3_000;

/// An opaque host route identity. Rust compares it but does not interpret it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Route(pub u32);

/// A connection generation assigned by the host.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Generation(pub u64);

/// One state-machine operation identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Operation(pub u64);

/// Firmware facts relevant to Refloat configuration admission and caching.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareFacts {
    pub major: u8,
    pub minor: u8,
    pub hardware_name: String,
    pub custom_configuration_count: u8,
    pub hardware_configuration_crc: Option<u32>,
}

/// Exact host-owned cache key.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct CacheIdentity {
    pub firmware_version: String,
    pub hardware_name: String,
    pub package_version: String,
    pub custom_index: u8,
    pub hardware_configuration_crc: u32,
}

impl std::fmt::Debug for CacheIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CacheIdentity")
            .field("firmware_version", &self.firmware_version)
            .field("hardware_name", &"[redacted]")
            .field("package_version", &self.package_version)
            .field("custom_index", &self.custom_index)
            .field("hardware_configuration_crc", &"[redacted]")
            .finish()
    }
}

/// Immutable facts used to start one connection generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeginFacts {
    pub route: Route,
    pub generation: Generation,
    pub firmware: FirmwareFacts,
    pub reported_version: String,
    pub writable: bool,
    pub flywheel: bool,
    /// Host-loaded bytes for the exact derived cache identity, if present.
    pub cached_schema: Option<Vec<u8>>,
}

/// Derive the exact host-owned schema cache identity, when firmware supplies
/// the nonzero hardware-configuration CRC required to make it stable.
#[must_use]
pub fn cache_identity(firmware: &FirmwareFacts, reported_version: &str) -> Option<CacheIdentity> {
    firmware
        .hardware_configuration_crc
        .filter(|crc| *crc != 0)
        .map(|crc| CacheIdentity {
            firmware_version: format!("{}.{}", firmware.major, firmware.minor),
            hardware_name: firmware.hardware_name.clone(),
            package_version: reported_version.to_owned(),
            custom_index: REFLOAT_INDEX,
            hardware_configuration_crc: crc,
        })
}

/// Audited package identity selected from the complete schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaIdentity {
    pub package_name: &'static str,
    pub package_version: &'static str,
    pub fingerprint: &'static str,
    pub wire_signature: u32,
}

/// One immutable audited catalogue entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogEntry {
    pub version: &'static str,
    pub fingerprint: &'static str,
    pub wire_signature: u32,
}

/// Exact source-derived Refloat configuration releases.
pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        version: "1.0.0",
        fingerprint: "324e7f5ef1ea1b2038478af7e6f43c91d63ec06a6caf2907c5b7e6c812382ad9",
        wire_signature: 4_090_797_430,
    },
    CatalogEntry {
        version: "1.0.1",
        fingerprint: "887e7b03c03304a2f8455e71b369b89830e8d6446a73aa598ab067ac81944526",
        wire_signature: 4_090_797_430,
    },
    CatalogEntry {
        version: "1.1.0",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        wire_signature: 15_843_867,
    },
    CatalogEntry {
        version: "1.1.1",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        wire_signature: 15_843_867,
    },
    CatalogEntry {
        version: "1.1.2",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        wire_signature: 15_843_867,
    },
    CatalogEntry {
        version: "1.2.0",
        fingerprint: "2561e00a5b6694de0ac272bae077e06050cfc00cf4ba371bb1d5d9616e137f7e",
        wire_signature: 2_427_955_642,
    },
    CatalogEntry {
        version: "1.2.1",
        fingerprint: "9b8979a36759810a7fa9c19d01534ed52ec9ed8ac4a2d3ab3aefc594ba3b93cc",
        wire_signature: 2_427_955_642,
    },
    CatalogEntry {
        version: "1.2.2",
        fingerprint: "d7ca6a2c06ce5701858eebfaaf224e97e08a212d1dab60b664d268a90763eb39",
        wire_signature: 2_427_955_642,
    },
    CatalogEntry {
        version: "1.2.3",
        fingerprint: "fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c",
        wire_signature: 2_427_955_642,
    },
    CatalogEntry {
        version: "1.3.0",
        fingerprint: "62681208dc609eb19420c89df65105f018499aa9f5741f295f00d62ea5a1bceb",
        wire_signature: 421_162_011,
    },
    CatalogEntry {
        version: "1.3.1-beta2",
        fingerprint: "6bc6bfa433f8205519bc931723e39252aa71c9f538a0ed7c200d688486c63e73",
        wire_signature: 421_162_011,
    },
];

/// Resolve a reported release only when its complete audited identity matches.
#[must_use]
pub fn resolve_catalog(
    reported_version: &str,
    schema: &ConfigurationSchema,
) -> Option<CatalogEntry> {
    CATALOG.iter().copied().find(|entry| {
        (entry.version == reported_version
            || (reported_version == "1.0" && entry.version.starts_with("1.0.")))
            && entry.fingerprint == schema.fingerprint
            && entry.wire_signature == schema.wire_signature
    })
}

/// Published configuration availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Availability {
    Unavailable,
    Loading,
    Ready,
}

/// Active lifecycle operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingOperation {
    Schema,
    ReadCurrent,
    ReadDefault,
    Write,
}

/// One complete decoded value revision.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedValues {
    pub revision: u64,
    pub source: ValueSource,
    pub values: ConfigurationValues,
}

/// Authoritative source of a decoded value set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueSource {
    Current,
    Default,
}

/// Deterministic publishable state.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub availability: Availability,
    pub writable: bool,
    pub identity: Option<SchemaIdentity>,
    pub current: Option<LoadedValues>,
    pub defaults: Option<LoadedValues>,
    pub pending: Option<PendingOperation>,
}

/// Work the host must submit through its existing scheduler and transport.
#[derive(Clone, Debug, PartialEq)]
pub enum RequestEffect {
    SchemaChunk {
        route: Route,
        length: usize,
        offset: usize,
        operation: Operation,
    },
    Values {
        route: Route,
        defaults: bool,
        operation: Operation,
    },
    Write {
        route: Route,
        payload: Vec<u8>,
        operation: Operation,
    },
    Verification {
        route: Route,
        operation: Operation,
    },
}

/// Cache work the host applies before the request from the same transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CacheEffect {
    Store {
        identity: CacheIdentity,
        bytes: Vec<u8>,
    },
    Remove {
        identity: CacheIdentity,
    },
}

/// Atomic bounded result of one state transition.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub request: Option<RequestEffect>,
    pub cache: Option<CacheEffect>,
}

impl Transition {
    const NONE: Self = Self {
        request: None,
        cache: None,
    };

    fn request(request: RequestEffect) -> Self {
        Self {
            request: Some(request),
            cache: None,
        }
    }
}

/// Stable, payload-free lifecycle failure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ServiceError {
    Unavailable,
    InvalidState,
    StaleResponse,
    UnauditedSchema,
    MalformedResponse,
    Conflict,
    Codec(ConfigurationError),
}

impl From<ConfigurationError> for ServiceError {
    fn from(value: ConfigurationError) -> Self {
        Self::Codec(value)
    }
}

/// Command represented by a values response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValuesCommand {
    Current,
    Default,
}

/// Decoded command-92–95 response facts supplied by the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    SchemaChunk {
        index: u8,
        total: usize,
        offset: usize,
        bytes: Vec<u8>,
    },
    Values {
        command: ValuesCommand,
        index: u8,
        bytes: Vec<u8>,
    },
    WriteAcknowledgement,
}

/// Scheduler context returned alongside a response or failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestContext {
    Schema {
        route: Route,
        index: u8,
        offset: usize,
        operation: Operation,
    },
    Values {
        route: Route,
        index: u8,
        defaults: bool,
        operation: Operation,
    },
    Write {
        route: Route,
        index: u8,
        operation: Operation,
    },
    Verification {
        route: Route,
        index: u8,
        operation: Operation,
    },
}

impl RequestContext {
    fn operation(self) -> Operation {
        match self {
            Self::Schema { operation, .. }
            | Self::Values { operation, .. }
            | Self::Write { operation, .. }
            | Self::Verification { operation, .. } => operation,
        }
    }
}

/// Stateful, connection-scoped configuration decision owner.
#[derive(Debug)]
pub struct ConfigurationService {
    route: Option<Route>,
    generation: Generation,
    reported_version: Option<String>,
    can_write: bool,
    flywheel: bool,
    next_operation: u64,
    operation: Option<Operation>,
    assembly: Vec<u8>,
    total_length: Option<usize>,
    requested_offset: usize,
    schema: Option<ConfigurationSchema>,
    identity: Option<SchemaIdentity>,
    current: Option<LoadedValues>,
    defaults: Option<LoadedValues>,
    expected_readback: Option<ConfigurationValues>,
    revision: u64,
    availability: Availability,
    pending: Option<PendingOperation>,
    cache_identity: Option<CacheIdentity>,
}

impl Default for ConfigurationService {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationService {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            route: None,
            generation: Generation(0),
            reported_version: None,
            can_write: false,
            flywheel: false,
            next_operation: 0,
            operation: None,
            assembly: Vec::new(),
            total_length: None,
            requested_offset: 0,
            schema: None,
            identity: None,
            current: None,
            defaults: None,
            expected_readback: None,
            revision: 0,
            availability: Availability::Unavailable,
            pending: None,
            cache_identity: None,
        }
    }

    /// Start a new generation and return its first bounded host action.
    pub fn begin(&mut self, facts: BeginFacts) -> Transition {
        self.reset_connection();
        self.route = Some(facts.route);
        self.generation = facts.generation;
        self.reported_version = Some(facts.reported_version.clone());
        self.can_write = facts.writable;
        self.flywheel = facts.flywheel;
        if facts.firmware.custom_configuration_count == REFLOAT_INDEX {
            return Transition::NONE;
        }
        self.availability = Availability::Loading;

        let cache_identity = cache_identity(&facts.firmware, &facts.reported_version);
        self.cache_identity = cache_identity.clone();
        let mut cache = None;
        if let (Some(identity), Some(bytes)) = (cache_identity, facts.cached_schema) {
            if let Ok(decoded) = decode_qt_compressed(&bytes)
                && let Some(entry) =
                    resolve_catalog(self.reported_version.as_deref().unwrap_or(""), &decoded)
            {
                self.schema = Some(decoded);
                self.identity = Some(schema_identity(entry));
                self.availability = Availability::Ready;
                if self.flywheel {
                    return Transition::NONE;
                }
                self.pending = Some(PendingOperation::ReadCurrent);
                let operation = self.start_operation();
                return Transition::request(RequestEffect::Values {
                    route: facts.route,
                    defaults: false,
                    operation,
                });
            }
            cache = Some(CacheEffect::Remove { identity });
        }
        self.pending = Some(PendingOperation::Schema);
        let operation = self.start_operation();
        Transition {
            request: Some(RequestEffect::SchemaChunk {
                route: facts.route,
                length: MAX_SCHEMA_CHUNK_BYTES,
                offset: 0,
                operation,
            }),
            cache,
        }
    }

    /// Clear all connection-scoped state while retaining the operation counter.
    pub fn reset(&mut self) {
        self.reset_connection();
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            availability: self.availability,
            writable: self.availability == Availability::Ready
                && self.can_write
                && !self.flywheel
                && self.current.is_some(),
            identity: self.identity.clone(),
            current: if self.flywheel {
                None
            } else {
                self.current.clone()
            },
            defaults: self.defaults.clone(),
            pending: self.pending,
        }
    }

    #[must_use]
    pub fn loaded_schema(&self) -> Option<&ConfigurationSchema> {
        self.schema.as_ref()
    }

    /// Enter or leave flywheel mode and emit at most one refresh request.
    pub fn set_flywheel(&mut self, active: bool) -> Transition {
        let was_active = self.flywheel;
        self.flywheel = active;
        if active {
            self.current = None;
            if self.pending == Some(PendingOperation::ReadCurrent) {
                self.pending = None;
                self.operation = None;
            }
            return Transition::NONE;
        }
        let Some(route) = self.route else {
            return Transition::NONE;
        };
        if !was_active || self.schema.is_none() || self.pending.is_some() {
            return Transition::NONE;
        }
        self.pending = Some(PendingOperation::ReadCurrent);
        let operation = self.start_operation();
        Transition::request(RequestEffect::Values {
            route,
            defaults: false,
            operation,
        })
    }

    pub fn invalidate_current(&mut self) {
        self.current = None;
    }

    pub fn request_values(&mut self, defaults: bool) -> Result<Transition, ServiceError> {
        let route = self.route.ok_or(ServiceError::InvalidState)?;
        if self.schema.is_none() || self.pending.is_some() || (!defaults && self.flywheel) {
            return Err(ServiceError::InvalidState);
        }
        self.pending = Some(if defaults {
            PendingOperation::ReadDefault
        } else {
            PendingOperation::ReadCurrent
        });
        let operation = self.start_operation();
        Ok(Transition::request(RequestEffect::Values {
            route,
            defaults,
            operation,
        }))
    }

    pub fn request_write(
        &mut self,
        package_version: &str,
        fingerprint: &str,
        wire_signature: u32,
        base_revision: u64,
        values: &ConfigurationValues,
    ) -> Result<Transition, ServiceError> {
        let route = self.route.ok_or(ServiceError::InvalidState)?;
        let schema = self.schema.as_ref().ok_or(ServiceError::InvalidState)?;
        let identity = self.identity.as_ref().ok_or(ServiceError::InvalidState)?;
        let current = self.current.as_ref().ok_or(ServiceError::InvalidState)?;
        if self.availability != Availability::Ready
            || !self.can_write
            || self.flywheel
            || self.pending.is_some()
            || identity.package_version != package_version
            || identity.fingerprint != fingerprint
            || identity.wire_signature != wire_signature
            || current.revision != base_revision
        {
            return Err(ServiceError::InvalidState);
        }
        let payload = encode_values(values, schema)?;
        let canonical = decode_values(&payload, schema)?;
        self.expected_readback = Some(normalized(&canonical, identity.package_version));
        self.pending = Some(PendingOperation::Write);
        let operation = self.start_operation();
        Ok(Transition::request(RequestEffect::Write {
            route,
            payload,
            operation,
        }))
    }

    pub fn accept(
        &mut self,
        response: Response,
        context: RequestContext,
        generation: Generation,
    ) -> Result<Transition, ServiceError> {
        if generation != self.generation || self.route.is_none() || self.operation.is_none() {
            return Err(ServiceError::StaleResponse);
        }
        match (response, context) {
            (
                Response::SchemaChunk {
                    index,
                    total,
                    offset,
                    bytes,
                },
                RequestContext::Schema {
                    route,
                    index: expected_index,
                    offset: expected_offset,
                    operation,
                },
            ) => self.accept_schema(
                route,
                index,
                expected_index,
                total,
                offset,
                expected_offset,
                operation,
                bytes,
            ),
            (
                Response::Values {
                    command,
                    index,
                    bytes,
                },
                RequestContext::Values {
                    route,
                    index: expected_index,
                    defaults,
                    operation,
                },
            ) => self.accept_values(
                route,
                index,
                expected_index,
                command,
                defaults,
                operation,
                bytes,
            ),
            (
                Response::WriteAcknowledgement,
                RequestContext::Write {
                    route,
                    index,
                    operation,
                },
            ) => {
                if Some(route) != self.route
                    || index != REFLOAT_INDEX
                    || Some(operation) != self.operation
                    || self.pending != Some(PendingOperation::Write)
                    || self.expected_readback.is_none()
                {
                    return Err(ServiceError::StaleResponse);
                }
                Ok(Transition::request(RequestEffect::Verification {
                    route,
                    operation,
                }))
            }
            (
                Response::Values {
                    command,
                    index,
                    bytes,
                },
                RequestContext::Verification {
                    route,
                    index: expected_index,
                    operation,
                },
            ) => self.accept_verification(route, index, expected_index, command, operation, bytes),
            _ => Err(ServiceError::StaleResponse),
        }
    }

    pub fn request_failed(&mut self, context: RequestContext, generation: Generation) {
        if generation == self.generation && Some(context.operation()) == self.operation {
            self.fail_active();
        }
    }

    pub fn abort_active_operation(&mut self, generation: Generation) {
        if generation == self.generation && self.pending.is_some() {
            self.fail_active();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn accept_schema(
        &mut self,
        route: Route,
        index: u8,
        expected_index: u8,
        total: usize,
        offset: usize,
        expected_offset: usize,
        operation: Operation,
        bytes: Vec<u8>,
    ) -> Result<Transition, ServiceError> {
        if self.pending != Some(PendingOperation::Schema)
            || Some(route) != self.route
            || index != expected_index
            || index != REFLOAT_INDEX
            || offset != expected_offset
            || offset != self.requested_offset
            || Some(operation) != self.operation
            || bytes.is_empty()
            || total > MAX_COMPRESSED_BYTES
            || self.total_length.is_some_and(|known| known != total)
        {
            return Err(ServiceError::StaleResponse);
        }
        self.total_length = Some(total);
        self.assembly.extend_from_slice(&bytes);
        if self.assembly.len() > total {
            return Err(ServiceError::MalformedResponse);
        }
        self.requested_offset = self.assembly.len();
        if self.assembly.len() < total {
            return Ok(Transition::request(RequestEffect::SchemaChunk {
                route,
                length: MAX_SCHEMA_CHUNK_BYTES.min(total - self.assembly.len()),
                offset: self.assembly.len(),
                operation,
            }));
        }
        let decoded = decode_qt_compressed(&self.assembly)?;
        let Some(entry) = resolve_catalog(self.reported_version.as_deref().unwrap_or(""), &decoded)
        else {
            self.fail_active();
            return Err(ServiceError::UnauditedSchema);
        };
        self.schema = Some(decoded);
        self.identity = Some(schema_identity(entry));
        self.pending = None;
        self.operation = None;
        self.availability = Availability::Ready;
        let cache = self
            .cache_identity
            .clone()
            .map(|identity| CacheEffect::Store {
                identity,
                bytes: self.assembly.clone(),
            });
        let request = if self.flywheel {
            None
        } else {
            self.pending = Some(PendingOperation::ReadCurrent);
            let next = self.start_operation();
            Some(RequestEffect::Values {
                route,
                defaults: false,
                operation: next,
            })
        };
        Ok(Transition { request, cache })
    }

    #[allow(clippy::too_many_arguments)]
    fn accept_values(
        &mut self,
        route: Route,
        index: u8,
        expected_index: u8,
        command: ValuesCommand,
        defaults: bool,
        operation: Operation,
        bytes: Vec<u8>,
    ) -> Result<Transition, ServiceError> {
        let expected_command = if defaults {
            ValuesCommand::Default
        } else {
            ValuesCommand::Current
        };
        let expected_pending = if defaults {
            PendingOperation::ReadDefault
        } else {
            PendingOperation::ReadCurrent
        };
        if Some(route) != self.route
            || index != expected_index
            || index != REFLOAT_INDEX
            || Some(operation) != self.operation
            || command != expected_command
            || self.pending != Some(expected_pending)
        {
            return Err(ServiceError::StaleResponse);
        }
        if !defaults && self.flywheel {
            return Err(ServiceError::InvalidState);
        }
        let schema = self.schema.as_ref().ok_or(ServiceError::InvalidState)?;
        let decoded = decode_values(&bytes, schema)?;
        self.revision = self.revision.wrapping_add(1);
        let loaded = LoadedValues {
            revision: self.revision,
            source: if defaults {
                ValueSource::Default
            } else {
                ValueSource::Current
            },
            values: decoded,
        };
        if defaults {
            self.defaults = Some(loaded)
        } else {
            self.current = Some(loaded)
        }
        self.pending = None;
        self.operation = None;
        Ok(Transition::NONE)
    }

    fn accept_verification(
        &mut self,
        route: Route,
        index: u8,
        expected_index: u8,
        command: ValuesCommand,
        operation: Operation,
        bytes: Vec<u8>,
    ) -> Result<Transition, ServiceError> {
        if Some(route) != self.route
            || index != expected_index
            || index != REFLOAT_INDEX
            || Some(operation) != self.operation
            || self.pending != Some(PendingOperation::Write)
            || command != ValuesCommand::Current
            || self.flywheel
            || self.schema.is_none()
            || self.expected_readback.is_none()
        {
            return Err(ServiceError::StaleResponse);
        }
        let decoded = decode_values(&bytes, self.schema.as_ref().expect("checked schema"))?;
        self.revision = self.revision.wrapping_add(1);
        self.current = Some(LoadedValues {
            revision: self.revision,
            source: ValueSource::Current,
            values: decoded.clone(),
        });
        self.pending = None;
        self.operation = None;
        let expected = self.expected_readback.take().expect("checked readback");
        if decoded != expected {
            return Err(ServiceError::Conflict);
        }
        Ok(Transition::NONE)
    }

    fn start_operation(&mut self) -> Operation {
        self.next_operation = self.next_operation.wrapping_add(1);
        let operation = Operation(self.next_operation);
        self.operation = Some(operation);
        operation
    }

    fn fail_active(&mut self) {
        self.pending = None;
        self.cache_identity = None;
        self.operation = None;
        self.current = None;
        self.expected_readback = None;
        self.availability = if self.schema.is_some() {
            Availability::Ready
        } else {
            Availability::Unavailable
        };
    }

    fn reset_connection(&mut self) {
        self.route = None;
        self.reported_version = None;
        self.can_write = false;
        self.flywheel = false;
        self.operation = None;
        self.assembly.clear();
        self.total_length = None;
        self.requested_offset = 0;
        self.schema = None;
        self.identity = None;
        self.current = None;
        self.defaults = None;
        self.expected_readback = None;
        self.revision = 0;
        self.availability = Availability::Unavailable;
        self.pending = None;
        self.cache_identity = None;
    }
}

fn schema_identity(entry: CatalogEntry) -> SchemaIdentity {
    SchemaIdentity {
        package_name: "refloat",
        package_version: entry.version,
        fingerprint: entry.fingerprint,
        wire_signature: entry.wire_signature,
    }
}

fn normalized(values: &ConfigurationValues, version: &str) -> ConfigurationValues {
    let mut result = values.clone();
    if !version.starts_with("1.0.")
        && let Some((_, value)) = result
            .iter_mut()
            .find(|(name, _)| name == "meta.is_default")
    {
        *value = ConfigurationValue::Boolean(false);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_and_revision_counters_wrap_exactly() {
        let mut service = ConfigurationService::new();
        service.next_operation = u64::MAX;
        assert_eq!(service.start_operation(), Operation(0));
        service.revision = u64::MAX;
        let schema = ConfigurationSchema {
            fingerprint: String::new(),
            wire_signature: 0,
            fields: vec![],
            groups: vec![],
        };
        let payload = encode_values(&[], &schema).expect("empty schema payload");
        service.schema = Some(schema);
        service.route = Some(Route(1));
        service.operation = Some(Operation(0));
        service.pending = Some(PendingOperation::ReadCurrent);
        service.generation = Generation(1);
        let result = service.accept_values(
            Route(1),
            0,
            0,
            ValuesCommand::Current,
            false,
            Operation(0),
            payload,
        );
        assert_eq!(result, Ok(Transition::NONE));
        assert_eq!(service.current.expect("current").revision, 0);
    }
}
