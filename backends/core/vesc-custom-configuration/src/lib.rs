//! Strict, bounded VESC custom-configuration command envelopes.
//!
//! Apple and Android select this safe component through separate audited host
//! bridges. It deliberately knows nothing about those bridges, Refloat
//! schema/value semantics, routing, scheduling, or application state.

#![forbid(unsafe_code)]

/// `COMM_GET_CUSTOM_CONFIG_XML`.
pub const GET_SCHEMA_COMMAND: u8 = 92;
/// `COMM_GET_CUSTOM_CONFIG`.
pub const GET_CURRENT_COMMAND: u8 = 93;
/// `COMM_GET_CUSTOM_CONFIG_DEFAULT`.
pub const GET_DEFAULT_COMMAND: u8 = 94;
/// `COMM_SET_CUSTOM_CONFIG`.
pub const SET_CONFIGURATION_COMMAND: u8 = 95;

/// Product chunk limit used for schema retrieval.
pub const MAX_SCHEMA_CHUNK_BYTES: usize = 400;
/// Existing scheduler-level request payload limit shared by both native codecs.
pub const MAX_SCHEDULED_PAYLOAD_BYTES: usize = 65_536;
/// Maximum opaque bytes admitted by a command-95 request before framing.
pub const MAX_WRITE_VALUE_BYTES: usize = MAX_SCHEDULED_PAYLOAD_BYTES - 2;

const SCHEMA_RESPONSE_HEADER_BYTES: usize = 10;
const MINIMUM_VALUES_BYTES: usize = 4;
const KNOWN_NON_CONFIGURATION_COMMANDS: [u8; 8] = [0, 4, 14, 34, 36, 62, 96, 156];

/// Current or default values response identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValuesCommand {
    /// `COMM_GET_CUSTOM_CONFIG`.
    Current,
    /// `COMM_GET_CUSTOM_CONFIG_DEFAULT`.
    Default,
}

impl ValuesCommand {
    /// Wire command byte.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        match self {
            Self::Current => GET_CURRENT_COMMAND,
            Self::Default => GET_DEFAULT_COMMAND,
        }
    }
}

/// One structurally valid VESC custom-configuration response.
///
/// This deliberately does not implement `Debug`: opaque schema and tuning
/// bytes must not be emitted accidentally through routine diagnostics.
#[derive(Eq, PartialEq)]
pub enum CustomConfigurationResponse<'a> {
    /// A bounded slice of the Qt-compressed schema document.
    SchemaChunk {
        /// Configuration index echoed by the controller.
        index: u8,
        /// Complete schema document length.
        total_length: usize,
        /// Offset of `bytes` in the complete document.
        offset: usize,
        /// Opaque caller-owned schema bytes.
        bytes: &'a [u8],
    },
    /// Opaque current or default serialized values.
    Values {
        /// Which values command produced the response.
        command: ValuesCommand,
        /// Configuration index echoed by the controller.
        index: u8,
        /// Opaque caller-owned signature and values bytes.
        bytes: &'a [u8],
    },
    /// Exact command-95 acknowledgement.
    WriteAcknowledgement,
}

/// Stable bounded codec failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CustomConfigurationError {
    /// An input, extent, command, or envelope is structurally invalid.
    InvalidPayload,
    /// A recognized VESC command does not belong to commands 92–95.
    UnsupportedCommand(u8),
}

impl std::fmt::Display for CustomConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPayload => formatter.write_str("invalid custom-configuration payload"),
            Self::UnsupportedCommand(command) => {
                write!(formatter, "unsupported VESC command {command}")
            }
        }
    }
}

impl std::error::Error for CustomConfigurationError {}

/// Encode one command-92 schema request.
pub fn schema_request(
    index: u8,
    length: i64,
    offset: i64,
) -> Result<[u8; 10], CustomConfigurationError> {
    if !(1..=MAX_SCHEMA_CHUNK_BYTES as i64).contains(&length)
        || !(0..=i64::from(i32::MAX)).contains(&offset)
    {
        return Err(CustomConfigurationError::InvalidPayload);
    }
    let length = i32::try_from(length).map_err(|_| CustomConfigurationError::InvalidPayload)?;
    let offset = i32::try_from(offset).map_err(|_| CustomConfigurationError::InvalidPayload)?;
    let length = length.to_be_bytes();
    let offset = offset.to_be_bytes();
    Ok([
        GET_SCHEMA_COMMAND,
        index,
        length[0],
        length[1],
        length[2],
        length[3],
        offset[0],
        offset[1],
        offset[2],
        offset[3],
    ])
}

/// Encode one command-93 current or command-94 default request.
#[must_use]
pub const fn values_request(index: u8, defaults: bool) -> [u8; 2] {
    [
        if defaults {
            GET_DEFAULT_COMMAND
        } else {
            GET_CURRENT_COMMAND
        },
        index,
    ]
}

/// Encode one command-95 write request while leaving values opaque.
pub fn write_request(index: u8, values: &[u8]) -> Result<Vec<u8>, CustomConfigurationError> {
    if values.is_empty() || values.len() > MAX_WRITE_VALUE_BYTES {
        return Err(CustomConfigurationError::InvalidPayload);
    }
    let mut request = Vec::with_capacity(values.len() + 2);
    request.extend_from_slice(&[SET_CONFIGURATION_COMMAND, index]);
    request.extend_from_slice(values);
    Ok(request)
}

/// Decode one bounded commands 92–95 response without interpreting its
/// compressed schema or serialized value bytes.
pub fn decode_response(
    payload: &[u8],
) -> Result<CustomConfigurationResponse<'_>, CustomConfigurationError> {
    if payload.len() > MAX_SCHEDULED_PAYLOAD_BYTES {
        return Err(CustomConfigurationError::InvalidPayload);
    }
    let Some(&command) = payload.first() else {
        return Err(CustomConfigurationError::InvalidPayload);
    };
    match command {
        GET_SCHEMA_COMMAND => decode_schema_chunk(payload),
        GET_CURRENT_COMMAND | GET_DEFAULT_COMMAND => decode_values(payload, command),
        SET_CONFIGURATION_COMMAND => {
            if payload.len() == 1 {
                Ok(CustomConfigurationResponse::WriteAcknowledgement)
            } else {
                Err(CustomConfigurationError::InvalidPayload)
            }
        }
        known if KNOWN_NON_CONFIGURATION_COMMANDS.contains(&known) => {
            Err(CustomConfigurationError::UnsupportedCommand(known))
        }
        _ => Err(CustomConfigurationError::InvalidPayload),
    }
}

fn decode_schema_chunk(
    payload: &[u8],
) -> Result<CustomConfigurationResponse<'_>, CustomConfigurationError> {
    if payload.len() < SCHEMA_RESPONSE_HEADER_BYTES {
        return Err(CustomConfigurationError::InvalidPayload);
    }
    let total_length = positive_i32(payload, 2)?;
    let offset = nonnegative_i32(payload, 6)?;
    let bytes = &payload[SCHEMA_RESPONSE_HEADER_BYTES..];
    if bytes.len() > MAX_SCHEMA_CHUNK_BYTES
        || offset > total_length
        || bytes.len() > total_length - offset
    {
        return Err(CustomConfigurationError::InvalidPayload);
    }
    Ok(CustomConfigurationResponse::SchemaChunk {
        index: payload[1],
        total_length,
        offset,
        bytes,
    })
}

fn decode_values(
    payload: &[u8],
    command: u8,
) -> Result<CustomConfigurationResponse<'_>, CustomConfigurationError> {
    if payload.len() < 2 + MINIMUM_VALUES_BYTES {
        return Err(CustomConfigurationError::InvalidPayload);
    }
    Ok(CustomConfigurationResponse::Values {
        command: if command == GET_CURRENT_COMMAND {
            ValuesCommand::Current
        } else {
            ValuesCommand::Default
        },
        index: payload[1],
        bytes: &payload[2..],
    })
}

fn positive_i32(payload: &[u8], offset: usize) -> Result<usize, CustomConfigurationError> {
    let value = nonnegative_i32(payload, offset)?;
    if value == 0 {
        Err(CustomConfigurationError::InvalidPayload)
    } else {
        Ok(value)
    }
}

fn nonnegative_i32(payload: &[u8], offset: usize) -> Result<usize, CustomConfigurationError> {
    let bytes: [u8; 4] = payload
        .get(offset..offset + 4)
        .ok_or(CustomConfigurationError::InvalidPayload)?
        .try_into()
        .map_err(|_| CustomConfigurationError::InvalidPayload)?;
    let value = i32::from_be_bytes(bytes);
    usize::try_from(value).map_err(|_| CustomConfigurationError::InvalidPayload)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ReleaseFixtures {
        version: &'static str,
        schema: &'static [u8],
        current: &'static [u8],
        defaults: &'static [u8],
        write: &'static [u8],
        readback: &'static [u8],
    }

    macro_rules! release {
        ($version:literal) => {
            ReleaseFixtures {
                version: $version,
                schema: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/schemas/",
                    $version,
                    ".qcompress"
                )),
                current: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-current.bin"
                )),
                defaults: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-default.bin"
                )),
                write: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-write.bin"
                )),
                readback: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-readback.bin"
                )),
            }
        };
    }

    const RELEASES: [ReleaseFixtures; 10] = [
        release!("1.0.0"),
        release!("1.0.1"),
        release!("1.1.0"),
        release!("1.1.1"),
        release!("1.1.2"),
        release!("1.2.0"),
        release!("1.2.1"),
        release!("1.2.2"),
        release!("1.2.3"),
        release!("1.3.0"),
    ];

    #[test]
    fn every_release_preserves_schema_and_value_envelopes() {
        for release in RELEASES {
            assert!(
                release.schema.len() > MAX_SCHEMA_CHUNK_BYTES,
                "{}",
                release.version
            );
            assert!(
                release.current.len() >= MINIMUM_VALUES_BYTES,
                "{}",
                release.version
            );
            assert!(
                release.defaults.len() >= MINIMUM_VALUES_BYTES,
                "{}",
                release.version
            );
            assert!(
                release.readback.len() >= MINIMUM_VALUES_BYTES,
                "{}",
                release.version
            );

            assert_eq!(
                schema_request(0, 400, 0),
                Ok([92, 0, 0, 0, 1, 144, 0, 0, 0, 0]),
                "{}",
                release.version
            );
            let chunk = &release.schema[..MAX_SCHEMA_CHUNK_BYTES];
            let response = schema_response(release.schema.len() as i32, 0, chunk);
            assert_schema(
                decode_response(&response),
                0,
                release.schema.len(),
                0,
                chunk,
                release.version,
            );

            assert_values(
                decode_response(&values_response(GET_CURRENT_COMMAND, 0, release.current)),
                ValuesCommand::Current,
                0,
                release.current,
                release.version,
            );
            assert_values(
                decode_response(&values_response(GET_DEFAULT_COMMAND, 0, release.defaults)),
                ValuesCommand::Default,
                0,
                release.defaults,
                release.version,
            );
            assert_values(
                decode_response(&values_response(GET_CURRENT_COMMAND, 0, release.readback)),
                ValuesCommand::Current,
                0,
                release.readback,
                release.version,
            );

            let write = write_request(0, release.write).expect("fixture write must be bounded");
            assert_eq!(
                write[0..2],
                [SET_CONFIGURATION_COMMAND, 0],
                "{}",
                release.version
            );
            assert_eq!(&write[2..], release.write, "{}", release.version);
            assert!(matches!(
                decode_response(&[SET_CONFIGURATION_COMMAND]),
                Ok(CustomConfigurationResponse::WriteAcknowledgement)
            ));
        }
    }

    #[test]
    fn request_encoding_covers_index_integer_and_payload_boundaries() {
        for index in u8::MIN..=u8::MAX {
            let schema = schema_request(index, 1, i64::from(i32::MAX)).unwrap();
            assert_eq!(schema[0], GET_SCHEMA_COMMAND);
            assert_eq!(schema[1], index);
            assert_eq!(&schema[2..6], &[0, 0, 0, 1]);
            assert_eq!(&schema[6..], &i32::MAX.to_be_bytes());
            assert_eq!(values_request(index, false), [GET_CURRENT_COMMAND, index]);
            assert_eq!(values_request(index, true), [GET_DEFAULT_COMMAND, index]);
        }

        for (length, offset) in [
            (-1, 0),
            (0, 0),
            (401, 0),
            (1, -1),
            (1, i64::from(i32::MAX) + 1),
        ] {
            assert_eq!(
                schema_request(0, length, offset),
                Err(CustomConfigurationError::InvalidPayload)
            );
        }
        assert!(schema_request(255, 400, i64::from(i32::MAX)).is_ok());

        assert_eq!(
            write_request(0, &[]),
            Err(CustomConfigurationError::InvalidPayload)
        );
        let maximum = vec![0x5a; MAX_WRITE_VALUE_BYTES];
        let request = write_request(255, &maximum).unwrap();
        assert_eq!(request.len(), MAX_SCHEDULED_PAYLOAD_BYTES);
        assert_eq!(&request[..2], &[SET_CONFIGURATION_COMMAND, 255]);
        assert_eq!(&request[2..], maximum);
        assert_eq!(
            write_request(0, &vec![0; MAX_WRITE_VALUE_BYTES + 1]),
            Err(CustomConfigurationError::InvalidPayload)
        );

        // Existing native codec admission is scheduler-bounded. The selected
        // frame layer independently rejects a payload above its 4,096 bytes.
        assert_eq!(write_request(0, &vec![0; 4_094]).unwrap().len(), 4_096);
        assert_eq!(write_request(0, &vec![0; 4_095]).unwrap().len(), 4_097);
    }

    #[test]
    fn schema_decoding_covers_truncation_signed_extents_and_chunk_arithmetic() {
        let valid = schema_response(4, 0, &[1, 2, 3, 4]);
        for length in 0..SCHEMA_RESPONSE_HEADER_BYTES {
            assert_invalid(decode_response(&valid[..length]));
        }
        assert_schema(decode_response(&valid), 0, 4, 0, &[1, 2, 3, 4], "valid");
        assert_schema(
            decode_response(&schema_response(4, 4, &[])),
            0,
            4,
            4,
            &[],
            "empty-final",
        );

        for payload in [
            schema_response(0, 0, &[]),
            schema_response(-1, 0, &[]),
            schema_response(4, -1, &[]),
            schema_response(4, 5, &[]),
            schema_response(4, 3, &[1, 2]),
            schema_response(401, 0, &vec![0; 401]),
        ] {
            assert_invalid(decode_response(&payload));
        }
    }

    #[test]
    fn values_and_acknowledgement_decoding_are_exact_and_bounded() {
        for command in [GET_CURRENT_COMMAND, GET_DEFAULT_COMMAND] {
            for length in 1..6 {
                let payload = vec![command; length];
                assert_invalid(decode_response(&payload));
            }
        }
        assert_values(
            decode_response(&[GET_CURRENT_COMMAND, 255, 0, 0, 0, 1]),
            ValuesCommand::Current,
            255,
            &[0, 0, 0, 1],
            "minimum",
        );

        let mut maximum = vec![0x5a; MAX_SCHEDULED_PAYLOAD_BYTES];
        maximum[0] = GET_DEFAULT_COMMAND;
        maximum[1] = 255;
        assert_values(
            decode_response(&maximum),
            ValuesCommand::Default,
            255,
            &maximum[2..],
            "maximum",
        );
        let mut excessive = maximum;
        excessive.push(0);
        assert_invalid(decode_response(&excessive));

        assert!(matches!(
            decode_response(&[SET_CONFIGURATION_COMMAND]),
            Ok(CustomConfigurationResponse::WriteAcknowledgement)
        ));
        assert_invalid(decode_response(&[SET_CONFIGURATION_COMMAND, 0]));
    }

    #[test]
    fn wrong_commands_and_errors_reveal_no_payload_bytes() {
        for command in KNOWN_NON_CONFIGURATION_COMMANDS {
            assert_eq!(
                decode_response(&[command]).err(),
                Some(CustomConfigurationError::UnsupportedCommand(command))
            );
        }
        assert_invalid(decode_response(&[]));
        assert_invalid(decode_response(&[1]));
        assert_invalid(decode_response(&[255]));

        let marker = "SECRET-CONFIGURATION-VALUE";
        let mut malformed = values_response(GET_CURRENT_COMMAND, 0, marker.as_bytes());
        malformed.resize(MAX_SCHEDULED_PAYLOAD_BYTES + 1, 0);
        let error = decode_response(&malformed).unwrap_err_or_invalid();
        let diagnostic = error.to_string();
        assert!(!diagnostic.contains("SECRET"));
        assert_eq!(diagnostic, "invalid custom-configuration payload");
        assert_eq!(
            CustomConfigurationError::UnsupportedCommand(36).to_string(),
            "unsupported VESC command 36"
        );
    }

    fn schema_response(total_length: i32, offset: i32, bytes: &[u8]) -> Vec<u8> {
        let mut response = Vec::with_capacity(SCHEMA_RESPONSE_HEADER_BYTES + bytes.len());
        response.extend_from_slice(&[GET_SCHEMA_COMMAND, 0]);
        response.extend_from_slice(&total_length.to_be_bytes());
        response.extend_from_slice(&offset.to_be_bytes());
        response.extend_from_slice(bytes);
        response
    }

    fn values_response(command: u8, index: u8, bytes: &[u8]) -> Vec<u8> {
        let mut response = Vec::with_capacity(2 + bytes.len());
        response.extend_from_slice(&[command, index]);
        response.extend_from_slice(bytes);
        response
    }

    fn assert_schema(
        result: Result<CustomConfigurationResponse<'_>, CustomConfigurationError>,
        expected_index: u8,
        expected_total: usize,
        expected_offset: usize,
        expected_bytes: &[u8],
        context: &str,
    ) {
        match result {
            Ok(CustomConfigurationResponse::SchemaChunk {
                index,
                total_length,
                offset,
                bytes,
            }) => {
                assert_eq!(index, expected_index, "{context}");
                assert_eq!(total_length, expected_total, "{context}");
                assert_eq!(offset, expected_offset, "{context}");
                assert_eq!(bytes, expected_bytes, "{context}");
            }
            _ => panic!("expected schema chunk for {context}"),
        }
    }

    fn assert_values(
        result: Result<CustomConfigurationResponse<'_>, CustomConfigurationError>,
        expected_command: ValuesCommand,
        expected_index: u8,
        expected_bytes: &[u8],
        context: &str,
    ) {
        match result {
            Ok(CustomConfigurationResponse::Values {
                command,
                index,
                bytes,
            }) => {
                assert_eq!(command, expected_command, "{context}");
                assert_eq!(index, expected_index, "{context}");
                assert_eq!(bytes, expected_bytes, "{context}");
            }
            _ => panic!("expected values for {context}"),
        }
    }

    fn assert_invalid(result: Result<CustomConfigurationResponse<'_>, CustomConfigurationError>) {
        assert!(matches!(
            result,
            Err(CustomConfigurationError::InvalidPayload)
        ));
    }

    trait ErrorResultExt {
        fn unwrap_err_or_invalid(self) -> CustomConfigurationError;
    }

    impl ErrorResultExt for Result<CustomConfigurationResponse<'_>, CustomConfigurationError> {
        fn unwrap_err_or_invalid(self) -> CustomConfigurationError {
            match self {
                Err(error) => error,
                Ok(_) => panic!("expected invalid payload"),
            }
        }
    }
}
