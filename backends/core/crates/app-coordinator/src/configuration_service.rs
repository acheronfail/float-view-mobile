use std::collections::HashMap;

use domain_types::FirmwareRecord;
use protocol_refloat::{
    CONFIG_CURRENT_COMMAND, CONFIG_DEFAULT_COMMAND, CONFIG_WRITE_COMMAND, ConfigurationError,
    ConfigurationSchema, ConfigurationValue, MAX_COMPRESSED_BYTES, SchemaChunkAssembler,
    TuneIdentity, WriteReadbackResult, current_values_request, decode_configuration_values,
    decode_qt_compressed, default_values_request, encode_configuration_values, schema_request,
    verify_write_readback, write_values_request,
};
use protocol_vesc::ControllerRoute;

pub const CONFIGURATION_INDEX: u8 = 0;
pub const CONFIGURATION_CHUNK_BYTES: u32 = 400;
pub const CONFIGURATION_REQUEST_TIMEOUT_MILLIS: u64 = 3_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationAvailability {
    Unavailable,
    Loading,
    Ready,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationOperation {
    Schema,
    ReadCurrent,
    ReadDefault,
    Write,
    Verify,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigurationEffect {
    None,
    Request {
        route: ControllerRoute,
        payload: Vec<u8>,
        operation: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationSummary {
    pub availability: ConfigurationAvailability,
    pub writable: bool,
    pub identity: Option<TuneIdentity>,
    pub current_loaded: bool,
    pub default_loaded: bool,
    pub pending: Option<ConfigurationOperation>,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationServiceError {
    InvalidState,
    StaleResponse,
    UnauditedSchema,
    MalformedResponse,
    Conflict,
}

#[derive(Clone, Copy)]
pub struct ConfigurationBegin<'a> {
    pub route: ControllerRoute,
    pub generation: u64,
    pub firmware: &'a FirmwareRecord,
    pub reported_version: &'a str,
    pub writable: bool,
    pub flywheel: bool,
    pub cached_schema: Option<&'a [u8]>,
}

#[derive(Clone, Debug)]
pub struct ConfigurationContext {
    pub route: ControllerRoute,
    pub generation: u64,
    pub firmware: FirmwareRecord,
    pub reported_version: String,
    pub writable: bool,
    pub flywheel: bool,
    pub cached_schema: Option<Vec<u8>>,
}

impl From<ConfigurationError> for ConfigurationServiceError {
    fn from(_: ConfigurationError) -> Self {
        Self::MalformedResponse
    }
}

#[derive(Clone, Copy)]
struct CatalogEntry {
    version: &'static str,
    fingerprint: &'static str,
    signature: u32,
}

const CATALOG: [CatalogEntry; 9] = [
    CatalogEntry {
        version: "1.0.0",
        fingerprint: "324e7f5ef1ea1b2038478af7e6f43c91d63ec06a6caf2907c5b7e6c812382ad9",
        signature: 4_090_797_430,
    },
    CatalogEntry {
        version: "1.0.1",
        fingerprint: "887e7b03c03304a2f8455e71b369b89830e8d6446a73aa598ab067ac81944526",
        signature: 4_090_797_430,
    },
    CatalogEntry {
        version: "1.1.0",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        signature: 15_843_867,
    },
    CatalogEntry {
        version: "1.1.1",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        signature: 15_843_867,
    },
    CatalogEntry {
        version: "1.1.2",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        signature: 15_843_867,
    },
    CatalogEntry {
        version: "1.2.0",
        fingerprint: "2561e00a5b6694de0ac272bae077e06050cfc00cf4ba371bb1d5d9616e137f7e",
        signature: 2_427_955_642,
    },
    CatalogEntry {
        version: "1.2.1",
        fingerprint: "9b8979a36759810a7fa9c19d01534ed52ec9ed8ac4a2d3ab3aefc594ba3b93cc",
        signature: 2_427_955_642,
    },
    CatalogEntry {
        version: "1.2.2",
        fingerprint: "d7ca6a2c06ce5701858eebfaaf224e97e08a212d1dab60b664d268a90763eb39",
        signature: 2_427_955_642,
    },
    CatalogEntry {
        version: "1.2.3",
        fingerprint: "fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c",
        signature: 2_427_955_642,
    },
];

pub struct ConfigurationService {
    route: Option<ControllerRoute>,
    generation: u64,
    reported_version: String,
    can_write: bool,
    flywheel: bool,
    next_operation: u64,
    active_operation: Option<u64>,
    requested_offset: usize,
    assembler: Option<SchemaChunkAssembler>,
    schema: Option<ConfigurationSchema>,
    schema_bytes: Option<Vec<u8>>,
    identity: Option<TuneIdentity>,
    current: Option<HashMap<String, ConfigurationValue>>,
    defaults: Option<HashMap<String, ConfigurationValue>>,
    expected_readback: Option<HashMap<String, ConfigurationValue>>,
    revision: u64,
    availability: ConfigurationAvailability,
    pending: Option<ConfigurationOperation>,
}

impl Default for ConfigurationService {
    fn default() -> Self {
        Self {
            route: None,
            generation: 0,
            reported_version: String::new(),
            can_write: false,
            flywheel: false,
            next_operation: 0,
            active_operation: None,
            requested_offset: 0,
            assembler: None,
            schema: None,
            schema_bytes: None,
            identity: None,
            current: None,
            defaults: None,
            expected_readback: None,
            revision: 0,
            availability: ConfigurationAvailability::Unavailable,
            pending: None,
        }
    }
}

impl ConfigurationService {
    pub fn begin_owned(&mut self, context: &ConfigurationContext) -> ConfigurationEffect {
        self.begin(ConfigurationBegin {
            route: context.route,
            generation: context.generation,
            firmware: &context.firmware,
            reported_version: &context.reported_version,
            writable: context.writable,
            flywheel: context.flywheel,
            cached_schema: context.cached_schema.as_deref(),
        })
    }

    pub fn begin(&mut self, context: ConfigurationBegin<'_>) -> ConfigurationEffect {
        *self = Self::default();
        self.route = Some(context.route);
        self.generation = context.generation;
        context
            .reported_version
            .clone_into(&mut self.reported_version);
        self.can_write = context.writable;
        self.flywheel = context.flywheel;
        if context.firmware.custom_configuration_count == CONFIGURATION_INDEX {
            return ConfigurationEffect::None;
        }
        self.availability = ConfigurationAvailability::Loading;
        if let Some(bytes) = context.cached_schema
            && self.install_schema(bytes).is_ok()
        {
            return if context.flywheel {
                ConfigurationEffect::None
            } else {
                self.request_values(false)
                    .unwrap_or(ConfigurationEffect::None)
            };
        }
        self.pending = Some(ConfigurationOperation::Schema);
        self.requested_offset = 0;
        self.effect(schema_request(CONFIGURATION_CHUNK_BYTES, 0))
    }

    #[must_use]
    pub fn summary(&self) -> ConfigurationSummary {
        let availability = if self.availability == ConfigurationAvailability::Ready
            && (self.flywheel
                || (self.current.is_some() && self.defaults.is_some() && self.pending.is_none()))
        {
            ConfigurationAvailability::Ready
        } else if self.availability == ConfigurationAvailability::Ready {
            ConfigurationAvailability::Loading
        } else {
            self.availability
        };
        ConfigurationSummary {
            availability,
            writable: availability == ConfigurationAvailability::Ready
                && self.can_write
                && !self.flywheel
                && self.current.is_some(),
            identity: self.identity.clone(),
            current_loaded: self.current.is_some() && !self.flywheel,
            default_loaded: self.defaults.is_some(),
            pending: self.pending,
            revision: self.revision,
        }
    }

    #[must_use]
    pub fn values(&self, defaults: bool) -> Option<&HashMap<String, ConfigurationValue>> {
        if defaults {
            self.defaults.as_ref()
        } else {
            self.current.as_ref()
        }
    }

    #[must_use]
    pub const fn schema(&self) -> Option<&ConfigurationSchema> {
        self.schema.as_ref()
    }

    #[must_use]
    pub fn schema_bytes(&self) -> Option<&[u8]> {
        self.schema_bytes.as_deref()
    }

    /// # Errors
    /// Returns a configuration-service error when state or response validation fails.
    pub fn accept_response(
        &mut self,
        generation: u64,
        operation: u64,
        response: &[u8],
    ) -> Result<ConfigurationEffect, ConfigurationServiceError> {
        match self.pending {
            Some(ConfigurationOperation::Schema) => {
                self.accept_schema_chunk(generation, operation, response)
            }
            Some(
                ConfigurationOperation::ReadCurrent
                | ConfigurationOperation::ReadDefault
                | ConfigurationOperation::Verify,
            ) => self.accept_values(generation, operation, response),
            Some(ConfigurationOperation::Write) => {
                self.accept_write(generation, operation, response)
            }
            None => Err(ConfigurationServiceError::StaleResponse),
        }
    }

    pub fn set_flywheel(&mut self, active: bool) -> ConfigurationEffect {
        let was_active = self.flywheel;
        self.flywheel = active;
        if active {
            self.current = None;
            if self.pending == Some(ConfigurationOperation::ReadCurrent) {
                self.clear_pending();
            }
            return ConfigurationEffect::None;
        }
        if was_active && self.schema.is_some() && self.pending.is_none() {
            return self
                .request_values(false)
                .unwrap_or(ConfigurationEffect::None);
        }
        ConfigurationEffect::None
    }

    /// # Errors
    /// Returns a configuration-service error when state or response validation fails.
    pub fn request_values(
        &mut self,
        defaults: bool,
    ) -> Result<ConfigurationEffect, ConfigurationServiceError> {
        if self.schema.is_none() || self.pending.is_some() || (!defaults && self.flywheel) {
            return Err(ConfigurationServiceError::InvalidState);
        }
        self.pending = Some(if defaults {
            ConfigurationOperation::ReadDefault
        } else {
            ConfigurationOperation::ReadCurrent
        });
        Ok(self.effect(if defaults {
            default_values_request().to_vec()
        } else {
            current_values_request().to_vec()
        }))
    }

    /// # Errors
    /// Returns a configuration-service error when state or response validation fails.
    pub fn request_write(
        &mut self,
        identity: &TuneIdentity,
        base_revision: u64,
        values: HashMap<String, ConfigurationValue>,
    ) -> Result<ConfigurationEffect, ConfigurationServiceError> {
        let Some(schema) = self.schema.as_ref() else {
            return Err(ConfigurationServiceError::InvalidState);
        };
        if self.pending.is_some()
            || !self.can_write
            || self.flywheel
            || self.current.is_none()
            || base_revision != self.revision
            || self.identity.as_ref() != Some(identity)
        {
            return Err(ConfigurationServiceError::InvalidState);
        }
        let encoded = encode_configuration_values(&values, schema)?;
        self.expected_readback = Some(values);
        self.pending = Some(ConfigurationOperation::Write);
        Ok(self.effect(write_values_request(&encoded)))
    }

    /// # Errors
    /// Returns a configuration-service error when state or response validation fails.
    pub fn accept_schema_chunk(
        &mut self,
        generation: u64,
        operation: u64,
        response: &[u8],
    ) -> Result<ConfigurationEffect, ConfigurationServiceError> {
        self.validate_response(generation, operation, ConfigurationOperation::Schema)?;
        if response.len() < 10 || response[1] != CONFIGURATION_INDEX {
            return Err(ConfigurationServiceError::MalformedResponse);
        }
        let total =
            u32::from_be_bytes([response[2], response[3], response[4], response[5]]) as usize;
        let offset =
            u32::from_be_bytes([response[6], response[7], response[8], response[9]]) as usize;
        if total > MAX_COMPRESSED_BYTES || offset != self.requested_offset {
            return Err(ConfigurationServiceError::StaleResponse);
        }
        if self.assembler.is_none() {
            self.assembler = Some(SchemaChunkAssembler::new(total)?);
        }
        let complete = self
            .assembler
            .as_mut()
            .ok_or(ConfigurationServiceError::MalformedResponse)?
            .accept_response(response)?;
        self.requested_offset = self.requested_offset.saturating_add(response.len() - 10);
        if !complete {
            return Ok(ConfigurationEffect::Request {
                route: self.route.ok_or(ConfigurationServiceError::InvalidState)?,
                payload: schema_request(
                    u32::try_from(
                        (total - self.requested_offset).min(CONFIGURATION_CHUNK_BYTES as usize),
                    )
                    .map_err(|_| ConfigurationServiceError::MalformedResponse)?,
                    u32::try_from(self.requested_offset)
                        .map_err(|_| ConfigurationServiceError::MalformedResponse)?,
                ),
                operation,
            });
        }
        let compressed = self
            .assembler
            .take()
            .ok_or(ConfigurationServiceError::MalformedResponse)?
            .finish()?;
        self.install_schema(&compressed)?;
        self.clear_pending();
        if self.flywheel {
            Ok(ConfigurationEffect::None)
        } else {
            self.request_values(false)
        }
    }

    /// # Errors
    /// Returns a configuration-service error when state or response validation fails.
    pub fn accept_values(
        &mut self,
        generation: u64,
        operation: u64,
        response: &[u8],
    ) -> Result<ConfigurationEffect, ConfigurationServiceError> {
        let pending = self
            .pending
            .ok_or(ConfigurationServiceError::StaleResponse)?;
        if !matches!(
            pending,
            ConfigurationOperation::ReadCurrent
                | ConfigurationOperation::ReadDefault
                | ConfigurationOperation::Verify
        ) {
            return Err(ConfigurationServiceError::StaleResponse);
        }
        self.validate_response(generation, operation, pending)?;
        let expected_command = if pending == ConfigurationOperation::ReadDefault {
            CONFIG_DEFAULT_COMMAND
        } else {
            CONFIG_CURRENT_COMMAND
        };
        if response.len() < 2
            || response[0] != expected_command
            || response[1] != CONFIGURATION_INDEX
        {
            return Err(ConfigurationServiceError::MalformedResponse);
        }
        let values = decode_configuration_values(
            &response[2..],
            self.schema
                .as_ref()
                .ok_or(ConfigurationServiceError::InvalidState)?,
        )?;
        let loaded_current = pending == ConfigurationOperation::ReadCurrent;
        if pending == ConfigurationOperation::ReadDefault {
            self.defaults = Some(values);
        } else if pending == ConfigurationOperation::Verify {
            let intended = self
                .expected_readback
                .take()
                .ok_or(ConfigurationServiceError::InvalidState)?;
            if verify_write_readback(true, &intended, &values) == WriteReadbackResult::Conflict {
                self.clear_pending();
                return Err(ConfigurationServiceError::Conflict);
            }
            self.current = Some(values);
            self.revision = self.revision.saturating_add(1);
        } else {
            self.current = Some(values);
            self.revision = self.revision.saturating_add(1);
        }
        self.clear_pending();
        if loaded_current && self.defaults.is_none() {
            self.request_values(true)
        } else {
            Ok(ConfigurationEffect::None)
        }
    }

    /// # Errors
    /// Returns a configuration-service error when state or response validation fails.
    pub fn accept_write(
        &mut self,
        generation: u64,
        operation: u64,
        response: &[u8],
    ) -> Result<ConfigurationEffect, ConfigurationServiceError> {
        self.validate_response(generation, operation, ConfigurationOperation::Write)?;
        if response != [CONFIG_WRITE_COMMAND, CONFIGURATION_INDEX, 1] {
            self.clear_pending();
            return Err(ConfigurationServiceError::Conflict);
        }
        self.clear_pending();
        self.pending = Some(ConfigurationOperation::Verify);
        Ok(self.effect(current_values_request().to_vec()))
    }

    fn install_schema(&mut self, bytes: &[u8]) -> Result<(), ConfigurationServiceError> {
        let schema = decode_qt_compressed(bytes)?;
        let entry = CATALOG.iter().find(|entry| {
            (entry.version == self.reported_version
                || (self.reported_version == "1.0" && entry.version.starts_with("1.0.")))
                && entry.fingerprint == schema.fingerprint
                && entry.signature == schema.wire_signature
        });
        let entry = entry.ok_or(ConfigurationServiceError::UnauditedSchema)?;
        self.identity = Some(TuneIdentity::from_schema(entry.version, &schema));
        self.schema = Some(schema);
        self.schema_bytes = Some(bytes.to_vec());
        self.availability = ConfigurationAvailability::Ready;
        Ok(())
    }

    fn validate_response(
        &self,
        generation: u64,
        operation: u64,
        expected: ConfigurationOperation,
    ) -> Result<(), ConfigurationServiceError> {
        if generation == self.generation
            && self.active_operation == Some(operation)
            && self.pending == Some(expected)
        {
            Ok(())
        } else {
            Err(ConfigurationServiceError::StaleResponse)
        }
    }

    fn effect(&mut self, payload: Vec<u8>) -> ConfigurationEffect {
        self.next_operation = self.next_operation.saturating_add(1);
        self.active_operation = Some(self.next_operation);
        ConfigurationEffect::Request {
            route: self.route.unwrap_or(ControllerRoute::Direct),
            payload,
            operation: self.next_operation,
        }
    }

    fn clear_pending(&mut self) {
        self.pending = None;
        self.active_operation = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn firmware() -> FirmwareRecord {
        FirmwareRecord {
            major: 6,
            minor: 6,
            test_version: 0,
            hardware_name: Some("75_300_R2".into()),
            firmware_name: None,
            hardware_type: 0,
            can_id: None,
            controller_uuid: None,
            custom_configuration_count: 1,
            hardware_configuration_crc: Some(1),
        }
    }

    #[test]
    fn audited_cached_schema_loads_then_reads_and_flywheel_hides_current_values() {
        let mut service = ConfigurationService::default();
        let firmware = firmware();
        let effect = service.begin(ConfigurationBegin {
            route: ControllerRoute::Direct,
            generation: 4,
            firmware: &firmware,
            reported_version: "1.2.3",
            writable: true,
            flywheel: false,
            cached_schema: Some(protocol_refloat::fixtures::CONFIGURATION_SCHEMA_123),
        });
        let ConfigurationEffect::Request {
            payload, operation, ..
        } = effect
        else {
            panic!("current request")
        };
        assert_eq!(payload, current_values_request());
        assert_eq!(
            service.summary().availability,
            ConfigurationAvailability::Loading
        );
        let mut response = vec![CONFIG_CURRENT_COMMAND, CONFIGURATION_INDEX];
        response.extend_from_slice(protocol_refloat::fixtures::CONFIGURATION_VALUES_123);
        let ConfigurationEffect::Request {
            operation: default_operation,
            ..
        } = service.accept_values(4, operation, &response).unwrap()
        else {
            panic!("default request")
        };
        let mut default_response = vec![CONFIG_DEFAULT_COMMAND, CONFIGURATION_INDEX];
        default_response.extend_from_slice(protocol_refloat::fixtures::CONFIGURATION_VALUES_123);
        assert_eq!(
            service
                .accept_values(4, default_operation, &default_response)
                .unwrap(),
            ConfigurationEffect::None
        );
        let summary = service.summary();
        assert_eq!(summary.availability, ConfigurationAvailability::Ready);
        assert!(summary.writable);
        let identity = summary.identity.expect("identity");
        let values = service.values(false).expect("values").clone();
        let ConfigurationEffect::Request {
            operation: write_operation,
            ..
        } = service
            .request_write(&identity, summary.revision, values)
            .expect("write")
        else {
            panic!("write request")
        };
        let ConfigurationEffect::Request {
            operation: verify_operation,
            payload: verify_payload,
            ..
        } = service
            .accept_write(
                4,
                write_operation,
                &[CONFIG_WRITE_COMMAND, CONFIGURATION_INDEX, 1],
            )
            .expect("ack")
        else {
            panic!("verification request")
        };
        assert_eq!(verify_payload, current_values_request());
        assert_eq!(
            service
                .accept_values(4, verify_operation, &response)
                .expect("readback"),
            ConfigurationEffect::None
        );
        assert_eq!(service.set_flywheel(true), ConfigurationEffect::None);
        assert!(!service.summary().current_loaded);
    }

    #[test]
    fn stale_operations_and_unaudited_schema_fail_closed() {
        let mut service = ConfigurationService::default();
        let firmware = firmware();
        let effect = service.begin(ConfigurationBegin {
            route: ControllerRoute::Direct,
            generation: 8,
            firmware: &firmware,
            reported_version: "9.9.9",
            writable: true,
            flywheel: false,
            cached_schema: None,
        });
        let ConfigurationEffect::Request { operation, .. } = effect else {
            panic!("request")
        };
        assert_eq!(
            service.accept_schema_chunk(7, operation, &[0; 10]),
            Err(ConfigurationServiceError::StaleResponse)
        );
    }
}
