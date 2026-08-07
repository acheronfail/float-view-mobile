//! Refloat configuration schema, value, transport, and Tune codecs.
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::format_collect,
    clippy::implicit_hasher,
    clippy::missing_errors_doc,
    clippy::too_many_lines
)]

use std::collections::{HashMap, HashSet};
use std::io::Read;

use flate2::read::ZlibDecoder;
use quick_xml::Reader;
use quick_xml::events::Event;
use sha2::{Digest, Sha256};

pub const CONFIG_SCHEMA_COMMAND: u8 = 92;
pub const CONFIG_CURRENT_COMMAND: u8 = 93;
pub const CONFIG_DEFAULT_COMMAND: u8 = 94;
pub const CONFIG_WRITE_COMMAND: u8 = 95;

pub const MAX_COMPRESSED_BYTES: usize = 32 * 1024;
pub const MAX_EXPANDED_BYTES: usize = 384 * 1024;
pub const MAX_CONFIGURATION_FIELDS: usize = 512;
pub const MAX_GROUPS: usize = 16;
pub const MAX_SUBGROUPS: usize = 64;
pub const MAX_ENUM_CHOICES: usize = 128;
pub const MAX_NAME_BYTES: usize = 128;
pub const MAX_DESCRIPTION_BYTES: usize = 4096;
pub const MAX_STRING_VALUE_BYTES: usize = 4096;
pub const MAX_XML_DEPTH: usize = 32;
pub const MAX_XML_ELEMENTS: usize = 12_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationError {
    TruncatedEnvelope,
    CompressedTooLarge,
    ExpandedTooLarge,
    ExpandedLengthMismatch,
    DecompressionFailed,
    ProhibitedXml,
    MalformedXml,
    StructuralLimit,
    DuplicateField,
    DuplicateOrder,
    MissingReference,
    UnsupportedType,
    UnsupportedEncoding,
    InvalidMetadata,
    InvalidNumber,
    SignatureMismatch,
    TruncatedValues,
    Remainder,
    MissingValue,
    ExtraValue,
    InvalidValue,
    UnexpectedCommand,
    UnexpectedIndex,
    UnexpectedOffset,
    UnexpectedLength,
    ConflictingChunk,
    IdentityMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ConfigurationFieldType {
    Double = 1,
    Integer = 2,
    String = 3,
    Enumeration = 4,
    Boolean = 5,
    Bitfield = 6,
}

impl TryFrom<u8> for ConfigurationFieldType {
    type Error = ConfigurationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Double),
            2 => Ok(Self::Integer),
            3 => Ok(Self::String),
            4 => Ok(Self::Enumeration),
            5 => Ok(Self::Boolean),
            6 => Ok(Self::Bitfield),
            _ => Err(ConfigurationError::UnsupportedType),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ConfigurationWireType {
    Uint8 = 1,
    Int8 = 2,
    Uint16 = 3,
    Int16 = 4,
    Uint32 = 5,
    Int32 = 6,
    Double16 = 7,
    Double32 = 8,
    Double32Auto = 9,
}

impl TryFrom<u8> for ConfigurationWireType {
    type Error = ConfigurationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Uint8),
            2 => Ok(Self::Int8),
            3 => Ok(Self::Uint16),
            4 => Ok(Self::Int16),
            5 => Ok(Self::Uint32),
            6 => Ok(Self::Int32),
            7 => Ok(Self::Double16),
            8 => Ok(Self::Double32),
            9 => Ok(Self::Double32Auto),
            _ => Err(ConfigurationError::UnsupportedEncoding),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigurationValue {
    Double(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigurationField {
    pub name: String,
    pub long_name: String,
    pub field_type: ConfigurationFieldType,
    pub wire_type: Option<ConfigurationWireType>,
    pub default_value: ConfigurationValue,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub step: Option<f64>,
    pub precision: Option<i32>,
    pub scale: Option<f64>,
    pub suffix: String,
    pub choices: Vec<String>,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationSubgroup {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationGroup {
    pub name: String,
    pub subgroups: Vec<ConfigurationSubgroup>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigurationSchema {
    pub fingerprint: String,
    pub wire_signature: u32,
    pub fields: Vec<ConfigurationField>,
    pub groups: Vec<ConfigurationGroup>,
}

#[derive(Default)]
struct RawField {
    values: HashMap<String, Vec<String>>,
}

impl RawField {
    fn first(&self, name: &str) -> &str {
        self.values
            .get(name)
            .and_then(|values| values.first())
            .map_or("", String::as_str)
    }
}

#[derive(Default)]
struct ParsedXml {
    fields: Vec<(String, RawField)>,
    order: Vec<String>,
    groups: Vec<ConfigurationGroup>,
}

pub fn decode_qt_compressed(input: &[u8]) -> Result<ConfigurationSchema, ConfigurationError> {
    if input.len() < 4 {
        return Err(ConfigurationError::TruncatedEnvelope);
    }
    if input.len() > MAX_COMPRESSED_BYTES {
        return Err(ConfigurationError::CompressedTooLarge);
    }
    let expected = u32::from_be_bytes([input[0], input[1], input[2], input[3]]) as usize;
    if expected > MAX_EXPANDED_BYTES {
        return Err(ConfigurationError::ExpandedTooLarge);
    }

    let mut decoder = ZlibDecoder::new(&input[4..]);
    let mut expanded = Vec::with_capacity(expected.min(MAX_EXPANDED_BYTES));
    decoder
        .by_ref()
        .take((MAX_EXPANDED_BYTES + 1) as u64)
        .read_to_end(&mut expanded)
        .map_err(|_| ConfigurationError::DecompressionFailed)?;
    if expanded.len() > MAX_EXPANDED_BYTES {
        return Err(ConfigurationError::ExpandedTooLarge);
    }
    if expanded.len() != expected || decoder.total_in() as usize != input.len() - 4 {
        return Err(ConfigurationError::ExpandedLengthMismatch);
    }
    parse_configuration_xml(&expanded)
}

pub fn parse_configuration_xml(xml: &[u8]) -> Result<ConfigurationSchema, ConfigurationError> {
    if xml.len() > MAX_EXPANDED_BYTES {
        return Err(ConfigurationError::ExpandedTooLarge);
    }
    let lowered = String::from_utf8_lossy(xml).to_ascii_lowercase();
    if lowered.contains("<!doctype") || lowered.contains("<!entity") {
        return Err(ConfigurationError::ProhibitedXml);
    }

    let parsed = parse_xml_structure(xml)?;
    if parsed.fields.len() > MAX_CONFIGURATION_FIELDS
        || parsed.groups.len() > MAX_GROUPS
        || parsed
            .groups
            .iter()
            .map(|group| group.subgroups.len())
            .sum::<usize>()
            > MAX_SUBGROUPS
    {
        return Err(ConfigurationError::StructuralLimit);
    }

    let mut raw_by_name = HashMap::with_capacity(parsed.fields.len());
    for (name, raw) in parsed.fields {
        if raw_by_name.insert(name, raw).is_some() {
            return Err(ConfigurationError::DuplicateField);
        }
    }
    let mut seen = HashSet::with_capacity(parsed.order.len());
    if parsed.order.iter().any(|name| !seen.insert(name)) {
        return Err(ConfigurationError::DuplicateOrder);
    }

    let mut fields = Vec::with_capacity(parsed.order.len());
    for name in parsed.order {
        let raw = raw_by_name
            .get(&name)
            .ok_or(ConfigurationError::MissingReference)?;
        fields.push(normalize_field(&name, raw)?);
    }
    let wire_signature = configuration_signature(&fields);
    let fingerprint = Sha256::digest(xml)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(ConfigurationSchema {
        fingerprint,
        wire_signature,
        fields,
        groups: parsed.groups,
    })
}

fn parse_xml_structure(xml: &[u8]) -> Result<ParsedXml, ConfigurationError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<String> = Vec::new();
    let mut text_stack: Vec<String> = Vec::new();
    let mut element_count = 0usize;
    let mut output = ParsedXml::default();
    let mut current_field: Option<(String, RawField)> = None;
    let mut current_group: Option<ConfigurationGroup> = None;
    let mut current_subgroup: Option<ConfigurationSubgroup> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                element_count += 1;
                let name = std::str::from_utf8(event.name().as_ref())
                    .map_err(|_| ConfigurationError::MalformedXml)?
                    .to_owned();
                stack.push(name.clone());
                text_stack.push(String::new());
                if stack.len() > MAX_XML_DEPTH || element_count > MAX_XML_ELEMENTS {
                    return Err(ConfigurationError::StructuralLimit);
                }
                if stack.len() == 3 && stack[0] == "ConfigParams" && stack[1] == "Params" {
                    current_field = Some((name, RawField::default()));
                } else if stack.ends_with(&["Grouping".into(), "group".into()]) {
                    current_group = Some(ConfigurationGroup {
                        name: String::new(),
                        subgroups: Vec::new(),
                    });
                } else if stack.ends_with(&["group".into(), "subgroup".into()]) {
                    current_subgroup = Some(ConfigurationSubgroup {
                        name: String::new(),
                        fields: Vec::new(),
                    });
                }
            }
            Ok(Event::Empty(event)) => {
                element_count += 1;
                if stack.len() + 1 > MAX_XML_DEPTH || element_count > MAX_XML_ELEMENTS {
                    return Err(ConfigurationError::StructuralLimit);
                }
                let name = std::str::from_utf8(event.name().as_ref())
                    .map_err(|_| ConfigurationError::MalformedXml)?
                    .to_owned();
                if stack.len() == 2 && stack[0] == "ConfigParams" && stack[1] == "Params" {
                    output.fields.push((name, RawField::default()));
                }
            }
            Ok(Event::Text(event)) => {
                let decoded = event
                    .decode()
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                let decoded = quick_xml::escape::unescape(&decoded)
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                let Some(current) = text_stack.last_mut() else {
                    if decoded.trim().is_empty() {
                        continue;
                    }
                    return Err(ConfigurationError::MalformedXml);
                };
                if current.len() + decoded.len() > MAX_DESCRIPTION_BYTES * 2 {
                    return Err(ConfigurationError::StructuralLimit);
                }
                current.push_str(&decoded);
            }
            Ok(Event::CData(event)) => {
                let decoded = event
                    .decode()
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                let Some(current) = text_stack.last_mut() else {
                    if decoded.trim().is_empty() {
                        continue;
                    }
                    return Err(ConfigurationError::MalformedXml);
                };
                if current.len() + decoded.len() > MAX_DESCRIPTION_BYTES * 2 {
                    return Err(ConfigurationError::StructuralLimit);
                }
                current.push_str(&decoded);
            }
            Ok(Event::End(event)) => {
                let end_name = std::str::from_utf8(event.name().as_ref())
                    .map_err(|_| ConfigurationError::MalformedXml)?
                    .to_owned();
                if stack.last() != Some(&end_name) {
                    return Err(ConfigurationError::MalformedXml);
                }
                let value = text_stack.pop().ok_or(ConfigurationError::MalformedXml)?;

                if stack.len() == 4 && stack[0] == "ConfigParams" && stack[1] == "Params" {
                    if let Some((_, raw)) = current_field.as_mut() {
                        raw.values.entry(end_name.clone()).or_default().push(value);
                    }
                } else if stack.as_slice() == ["ConfigParams", "SerOrder", "ser"] {
                    output.order.push(value);
                } else if stack.ends_with(&[
                    "group".into(),
                    "subgroup".into(),
                    "subgroupName".into(),
                ]) {
                    if let Some(subgroup) = current_subgroup.as_mut() {
                        subgroup.name = value;
                    }
                } else if stack.ends_with(&["subgroupParams".into(), "param".into()]) {
                    if let Some(subgroup) = current_subgroup.as_mut() {
                        subgroup.fields.push(value);
                    }
                } else if stack.ends_with(&["group".into(), "groupName".into()])
                    && let Some(group) = current_group.as_mut()
                {
                    group.name = value;
                }

                if stack.len() == 3 && stack[0] == "ConfigParams" && stack[1] == "Params" {
                    let field = current_field
                        .take()
                        .ok_or(ConfigurationError::MalformedXml)?;
                    output.fields.push(field);
                } else if stack.ends_with(&["group".into(), "subgroup".into()]) {
                    let subgroup = current_subgroup
                        .take()
                        .ok_or(ConfigurationError::MalformedXml)?;
                    let group = current_group
                        .as_mut()
                        .ok_or(ConfigurationError::MalformedXml)?;
                    group.subgroups.push(subgroup);
                } else if stack.ends_with(&["Grouping".into(), "group".into()]) {
                    let group = current_group
                        .take()
                        .ok_or(ConfigurationError::MalformedXml)?;
                    output.groups.push(group);
                }
                stack.pop();
            }
            Ok(Event::DocType(_)) => return Err(ConfigurationError::ProhibitedXml),
            Ok(Event::GeneralRef(reference)) => {
                let name = reference
                    .decode()
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                let replacement = match name.as_ref() {
                    "lt" => '<',
                    "gt" => '>',
                    "amp" => '&',
                    "apos" => '\'',
                    "quot" => '"',
                    _ => return Err(ConfigurationError::MalformedXml),
                };
                let current = text_stack
                    .last_mut()
                    .ok_or(ConfigurationError::MalformedXml)?;
                current.push(replacement);
            }
            Ok(Event::Eof) => break,
            Ok(Event::Decl(_) | Event::PI(_) | Event::Comment(_)) => {}
            Err(_) => return Err(ConfigurationError::MalformedXml),
        }
    }
    if !stack.is_empty() || !text_stack.is_empty() {
        return Err(ConfigurationError::MalformedXml);
    }
    Ok(output)
}

fn normalize_field(name: &str, raw: &RawField) -> Result<ConfigurationField, ConfigurationError> {
    bounded_nonempty(name, MAX_NAME_BYTES)?;
    let field_type = raw
        .first("type")
        .parse::<u8>()
        .map_err(|_| ConfigurationError::UnsupportedType)?
        .try_into()?;
    let wire_type = if raw.first("vTx").is_empty() {
        None
    } else {
        Some(
            raw.first("vTx")
                .parse::<u8>()
                .map_err(|_| ConfigurationError::UnsupportedEncoding)?
                .try_into()?,
        )
    };
    if matches!(
        field_type,
        ConfigurationFieldType::Double | ConfigurationFieldType::Integer
    ) && wire_type.is_none()
    {
        return Err(ConfigurationError::UnsupportedEncoding);
    }
    let choices = raw.values.get("enumNames").cloned().unwrap_or_default();
    if choices.len() > MAX_ENUM_CHOICES {
        return Err(ConfigurationError::StructuralLimit);
    }
    let default_value = match field_type {
        ConfigurationFieldType::Double => {
            ConfigurationValue::Double(required_finite(raw.first("valDouble"))?)
        }
        ConfigurationFieldType::Integer
        | ConfigurationFieldType::Enumeration
        | ConfigurationFieldType::Bitfield => ConfigurationValue::Integer(
            raw.first("valInt")
                .parse()
                .map_err(|_| ConfigurationError::InvalidNumber)?,
        ),
        ConfigurationFieldType::Boolean => match raw.first("valInt") {
            "0" => ConfigurationValue::Boolean(false),
            "1" => ConfigurationValue::Boolean(true),
            _ => return Err(ConfigurationError::InvalidNumber),
        },
        ConfigurationFieldType::String => {
            ConfigurationValue::String(raw.first("valString").to_owned())
        }
    };
    let description = sanitize_description(raw.first("description"));
    if description.len() > MAX_DESCRIPTION_BYTES {
        return Err(ConfigurationError::InvalidMetadata);
    }
    Ok(ConfigurationField {
        name: name.to_owned(),
        long_name: raw.first("longName").to_owned(),
        field_type,
        wire_type,
        default_value,
        minimum: optional_finite(match field_type {
            ConfigurationFieldType::Double => raw.first("minDouble"),
            _ => raw.first("minInt"),
        })?,
        maximum: optional_finite(match field_type {
            ConfigurationFieldType::Double => raw.first("maxDouble"),
            _ => raw.first("maxInt"),
        })?,
        step: optional_finite(match field_type {
            ConfigurationFieldType::Double => raw.first("stepDouble"),
            _ => raw.first("stepInt"),
        })?,
        precision: raw.first("editorDecimalsDouble").parse().ok(),
        scale: optional_finite(raw.first("vTxDoubleScale"))?,
        suffix: raw.first("suffix").to_owned(),
        choices,
        description,
    })
}

fn bounded_nonempty(value: &str, maximum: usize) -> Result<(), ConfigurationError> {
    if value.is_empty() || value.len() > maximum {
        Err(ConfigurationError::InvalidMetadata)
    } else {
        Ok(())
    }
}

fn required_finite(value: &str) -> Result<f64, ConfigurationError> {
    optional_finite(value)?.ok_or(ConfigurationError::InvalidNumber)
}

fn optional_finite(value: &str) -> Result<Option<f64>, ConfigurationError> {
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|_| ConfigurationError::InvalidNumber)?;
    if !parsed.is_finite() {
        return Err(ConfigurationError::InvalidNumber);
    }
    Ok(Some(parsed))
}

fn sanitize_description(value: &str) -> String {
    let mut result = String::new();
    let mut inside_tag = false;
    for character in value.chars() {
        match character {
            '<' => {
                inside_tag = true;
                result.push(' ');
            }
            '>' => {
                inside_tag = false;
                result.push(' ');
            }
            _ if !inside_tag => result.push(character),
            _ => {}
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[must_use]
pub fn configuration_signature(fields: &[ConfigurationField]) -> u32 {
    let mut input = String::new();
    for field in fields {
        input.push_str(&field.name);
        input.push_str(&(field.field_type as u8).to_string());
        input.push_str(&field.wire_type.map_or(0, |wire| wire as u8).to_string());
        for choice in &field.choices {
            input.push_str(choice);
        }
    }
    crc32c::crc32c(input.as_bytes())
}

struct ValueReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ValueReader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], ConfigurationError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ConfigurationError::TruncatedValues)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ConfigurationError::TruncatedValues)?;
        self.offset = end;
        Ok(value)
    }

    fn unsigned(&mut self, count: usize) -> Result<u64, ConfigurationError> {
        Ok(self
            .take(count)?
            .iter()
            .fold(0, |result, byte| (result << 8) | u64::from(*byte)))
    }

    fn signed(&mut self, count: usize) -> Result<i64, ConfigurationError> {
        let value = self.unsigned(count)?;
        let shift = 64 - count * 8;
        Ok(((value << shift) as i64) >> shift)
    }
}

pub fn decode_configuration_values(
    data: &[u8],
    schema: &ConfigurationSchema,
) -> Result<HashMap<String, ConfigurationValue>, ConfigurationError> {
    let mut reader = ValueReader {
        bytes: data,
        offset: 0,
    };
    if reader.unsigned(4)? != u64::from(schema.wire_signature) {
        return Err(ConfigurationError::SignatureMismatch);
    }
    let mut values = HashMap::with_capacity(schema.fields.len());
    for field in &schema.fields {
        values.insert(field.name.clone(), decode_value(field, &mut reader)?);
    }
    if reader.offset != data.len() {
        return Err(ConfigurationError::Remainder);
    }
    Ok(values)
}

fn decode_value(
    field: &ConfigurationField,
    reader: &mut ValueReader<'_>,
) -> Result<ConfigurationValue, ConfigurationError> {
    match field.field_type {
        ConfigurationFieldType::Double => {
            let scale = field
                .scale
                .filter(|scale| *scale > 0.0)
                .ok_or(ConfigurationError::InvalidMetadata)?;
            let raw = match field.wire_type {
                Some(ConfigurationWireType::Double16) => reader.signed(2)?,
                Some(ConfigurationWireType::Double32) => reader.signed(4)?,
                _ => return Err(ConfigurationError::UnsupportedEncoding),
            };
            Ok(ConfigurationValue::Double(raw as f64 / scale))
        }
        ConfigurationFieldType::Integer => {
            let raw = match field.wire_type {
                Some(ConfigurationWireType::Uint8) => reader.unsigned(1)? as i64,
                Some(ConfigurationWireType::Int8) => reader.signed(1)?,
                Some(ConfigurationWireType::Uint16) => reader.unsigned(2)? as i64,
                Some(ConfigurationWireType::Int16) => reader.signed(2)?,
                Some(ConfigurationWireType::Uint32) => reader.unsigned(4)? as i64,
                Some(ConfigurationWireType::Int32) => reader.signed(4)?,
                _ => return Err(ConfigurationError::UnsupportedEncoding),
            };
            Ok(ConfigurationValue::Integer(raw))
        }
        ConfigurationFieldType::Enumeration => Ok(ConfigurationValue::Integer(reader.signed(1)?)),
        ConfigurationFieldType::Bitfield => {
            Ok(ConfigurationValue::Integer(reader.unsigned(1)? as i64))
        }
        ConfigurationFieldType::Boolean => match reader.signed(1)? {
            0 => Ok(ConfigurationValue::Boolean(false)),
            1 => Ok(ConfigurationValue::Boolean(true)),
            _ => Err(ConfigurationError::InvalidValue),
        },
        ConfigurationFieldType::String => {
            let remainder = &reader.bytes[reader.offset..];
            let length = remainder
                .iter()
                .position(|byte| *byte == 0)
                .ok_or(ConfigurationError::TruncatedValues)?;
            if length > MAX_STRING_VALUE_BYTES {
                return Err(ConfigurationError::TruncatedValues);
            }
            let bytes = reader.take(length + 1)?;
            let value = std::str::from_utf8(&bytes[..length])
                .map_err(|_| ConfigurationError::InvalidValue)?;
            Ok(ConfigurationValue::String(value.to_owned()))
        }
    }
}

pub fn encode_configuration_values(
    values: &HashMap<String, ConfigurationValue>,
    schema: &ConfigurationSchema,
) -> Result<Vec<u8>, ConfigurationError> {
    let expected: HashSet<&str> = schema
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    if expected.iter().any(|name| !values.contains_key(*name)) {
        return Err(ConfigurationError::MissingValue);
    }
    if values.keys().any(|name| !expected.contains(name.as_str())) {
        return Err(ConfigurationError::ExtraValue);
    }
    let mut output = schema.wire_signature.to_be_bytes().to_vec();
    for field in &schema.fields {
        encode_value(
            values
                .get(&field.name)
                .ok_or(ConfigurationError::MissingValue)?,
            field,
            &mut output,
        )?;
    }
    Ok(output)
}

fn encode_value(
    value: &ConfigurationValue,
    field: &ConfigurationField,
    output: &mut Vec<u8>,
) -> Result<(), ConfigurationError> {
    match (field.field_type, value) {
        (ConfigurationFieldType::Double, ConfigurationValue::Double(number)) => {
            let scale = field
                .scale
                .filter(|scale| *scale > 0.0)
                .ok_or(ConfigurationError::InvalidMetadata)?;
            if !number.is_finite() || !within(*number, field) {
                return Err(ConfigurationError::InvalidValue);
            }
            let scaled = number * scale;
            if scaled.round() != scaled || scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
                return Err(ConfigurationError::InvalidValue);
            }
            let raw = scaled as i64;
            match field.wire_type {
                Some(ConfigurationWireType::Double16) if i16::try_from(raw).is_ok() => {
                    append_signed(raw, 2, output);
                }
                Some(ConfigurationWireType::Double32) if i32::try_from(raw).is_ok() => {
                    append_signed(raw, 4, output);
                }
                _ => return Err(ConfigurationError::InvalidValue),
            }
        }
        (ConfigurationFieldType::Integer, ConfigurationValue::Integer(number)) => {
            if !within(*number as f64, field) {
                return Err(ConfigurationError::InvalidValue);
            }
            let (minimum, maximum, bytes) = match field.wire_type {
                Some(ConfigurationWireType::Uint8) => (0, u8::MAX as i64, 1),
                Some(ConfigurationWireType::Int8) => (i8::MIN as i64, i8::MAX as i64, 1),
                Some(ConfigurationWireType::Uint16) => (0, u16::MAX as i64, 2),
                Some(ConfigurationWireType::Int16) => (i16::MIN as i64, i16::MAX as i64, 2),
                Some(ConfigurationWireType::Uint32) => (0, u32::MAX as i64, 4),
                Some(ConfigurationWireType::Int32) => (i32::MIN as i64, i32::MAX as i64, 4),
                _ => return Err(ConfigurationError::UnsupportedEncoding),
            };
            if *number < minimum || *number > maximum {
                return Err(ConfigurationError::InvalidValue);
            }
            append_signed(*number, bytes, output);
        }
        (ConfigurationFieldType::Enumeration, ConfigurationValue::Integer(number)) => {
            if *number < 0 || *number >= field.choices.len() as i64 || *number > i8::MAX as i64 {
                return Err(ConfigurationError::InvalidValue);
            }
            output.push(*number as u8);
        }
        (ConfigurationFieldType::Bitfield, ConfigurationValue::Integer(number)) => {
            output.push(u8::try_from(*number).map_err(|_| ConfigurationError::InvalidValue)?);
        }
        (ConfigurationFieldType::Boolean, ConfigurationValue::Boolean(value)) => {
            output.push(u8::from(*value));
        }
        (ConfigurationFieldType::String, ConfigurationValue::String(value)) => {
            if value.len() > MAX_STRING_VALUE_BYTES || value.as_bytes().contains(&0) {
                return Err(ConfigurationError::InvalidValue);
            }
            output.extend_from_slice(value.as_bytes());
            output.push(0);
        }
        _ => return Err(ConfigurationError::InvalidValue),
    }
    Ok(())
}

fn append_signed(value: i64, count: usize, output: &mut Vec<u8>) {
    let bytes = value.to_be_bytes();
    output.extend_from_slice(&bytes[bytes.len() - count..]);
}

fn within(value: f64, field: &ConfigurationField) -> bool {
    field.minimum.is_none_or(|minimum| value >= minimum)
        && field.maximum.is_none_or(|maximum| value <= maximum)
}

#[must_use]
pub fn schema_request(length: u32, offset: u32) -> Vec<u8> {
    let mut request = vec![CONFIG_SCHEMA_COMMAND, 0];
    request.extend_from_slice(&length.to_be_bytes());
    request.extend_from_slice(&offset.to_be_bytes());
    request
}

#[must_use]
pub fn current_values_request() -> [u8; 2] {
    [CONFIG_CURRENT_COMMAND, 0]
}

#[must_use]
pub fn default_values_request() -> [u8; 2] {
    [CONFIG_DEFAULT_COMMAND, 0]
}

#[must_use]
pub fn write_values_request(encoded_values: &[u8]) -> Vec<u8> {
    let mut request = vec![CONFIG_WRITE_COMMAND, 0];
    request.extend_from_slice(encoded_values);
    request
}

#[derive(Debug)]
pub struct SchemaChunkAssembler {
    total_length: usize,
    bytes: Vec<u8>,
    next_offset: usize,
}

impl SchemaChunkAssembler {
    pub fn new(total_length: usize) -> Result<Self, ConfigurationError> {
        if total_length > MAX_COMPRESSED_BYTES {
            return Err(ConfigurationError::CompressedTooLarge);
        }
        Ok(Self {
            total_length,
            bytes: Vec::with_capacity(total_length),
            next_offset: 0,
        })
    }

    pub fn accept_response(&mut self, response: &[u8]) -> Result<bool, ConfigurationError> {
        if response.len() < 10 {
            return Err(ConfigurationError::TruncatedEnvelope);
        }
        if response[0] != CONFIG_SCHEMA_COMMAND {
            return Err(ConfigurationError::UnexpectedCommand);
        }
        if response[1] != 0 {
            return Err(ConfigurationError::UnexpectedIndex);
        }
        let total =
            u32::from_be_bytes([response[2], response[3], response[4], response[5]]) as usize;
        let offset =
            u32::from_be_bytes([response[6], response[7], response[8], response[9]]) as usize;
        if total != self.total_length {
            return Err(ConfigurationError::UnexpectedLength);
        }
        if offset != self.next_offset {
            return Err(ConfigurationError::UnexpectedOffset);
        }
        let chunk = &response[10..];
        if chunk.len() > 400 || self.next_offset + chunk.len() > self.total_length {
            return Err(ConfigurationError::UnexpectedLength);
        }
        self.bytes.extend_from_slice(chunk);
        self.next_offset += chunk.len();
        Ok(self.next_offset == self.total_length)
    }

    pub fn finish(self) -> Result<Vec<u8>, ConfigurationError> {
        if self.next_offset != self.total_length {
            return Err(ConfigurationError::UnexpectedLength);
        }
        Ok(self.bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuneIdentity {
    pub package_version: String,
    pub schema_fingerprint: String,
    pub wire_signature: u32,
}

impl TuneIdentity {
    #[must_use]
    pub fn from_schema(package_version: impl Into<String>, schema: &ConfigurationSchema) -> Self {
        Self {
            package_version: package_version.into(),
            schema_fingerprint: schema.fingerprint.clone(),
            wire_signature: schema.wire_signature,
        }
    }

    pub fn validate(
        &self,
        package_version: &str,
        schema: &ConfigurationSchema,
    ) -> Result<(), ConfigurationError> {
        if self.package_version == package_version
            && self.schema_fingerprint == schema.fingerprint
            && self.wire_signature == schema.wire_signature
        {
            Ok(())
        } else {
            Err(ConfigurationError::IdentityMismatch)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tune {
    pub identity: TuneIdentity,
    pub values: HashMap<String, ConfigurationValue>,
}

impl Tune {
    pub fn encode_for(
        &self,
        package_version: &str,
        schema: &ConfigurationSchema,
    ) -> Result<Vec<u8>, ConfigurationError> {
        self.identity.validate(package_version, schema)?;
        encode_configuration_values(&self.values, schema)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteReadbackResult {
    Success,
    Conflict,
}

#[must_use]
pub fn verify_write_readback(
    acknowledged: bool,
    intended: &HashMap<String, ConfigurationValue>,
    readback: &HashMap<String, ConfigurationValue>,
) -> WriteReadbackResult {
    if acknowledged && intended == readback {
        WriteReadbackResult::Success
    } else {
        WriteReadbackResult::Conflict
    }
}

#[cfg(test)]
#[allow(clippy::match_same_arms, clippy::needless_raw_string_hashes)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Manifest {
        releases: Vec<Release>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Release {
        version: String,
        schema_fingerprint_sha256: String,
        wire_signature: u32,
        serialized_field_count: usize,
        group_count: usize,
    }

    fn schema_fixture(version: &str) -> &'static [u8] {
        match version {
            "1.0.0" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.0.0.qcompress")
            }
            "1.0.1" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.0.1.qcompress")
            }
            "1.1.0" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.1.0.qcompress")
            }
            "1.1.1" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.1.1.qcompress")
            }
            "1.1.2" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.1.2.qcompress")
            }
            "1.2.0" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.2.0.qcompress")
            }
            "1.2.1" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.2.1.qcompress")
            }
            "1.2.2" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.2.2.qcompress")
            }
            "1.2.3" => {
                include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.2.3.qcompress")
            }
            _ => panic!("unknown fixture version"),
        }
    }

    fn default_fixture(version: &str) -> &'static [u8] {
        match version {
            "1.0.0" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.0.0-default.bin"
            ),
            "1.0.1" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.0.1-default.bin"
            ),
            "1.1.0" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.1.0-default.bin"
            ),
            "1.1.1" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.1.1-default.bin"
            ),
            "1.1.2" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.1.2-default.bin"
            ),
            "1.2.0" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.2.0-default.bin"
            ),
            "1.2.1" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.2.1-default.bin"
            ),
            "1.2.2" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.2.2-default.bin"
            ),
            "1.2.3" => include_bytes!(
                "../../../../fixtures/refloat-configuration/values/1.2.3-default.bin"
            ),
            _ => panic!("unknown fixture version"),
        }
    }

    #[test]
    fn every_audited_schema_and_default_value_set_round_trips() {
        let manifest: Manifest = serde_json::from_str(crate::fixtures::CONFIGURATION_MANIFEST)
            .expect("configuration manifest");
        assert_eq!(manifest.releases.len(), 9);
        for release in manifest.releases {
            let schema = decode_qt_compressed(schema_fixture(&release.version)).expect("schema");
            assert_eq!(schema.fingerprint, release.schema_fingerprint_sha256);
            assert_eq!(schema.wire_signature, release.wire_signature);
            assert_eq!(schema.fields.len(), release.serialized_field_count);
            assert_eq!(schema.groups.len(), release.group_count);

            let encoded = default_fixture(&release.version);
            let values = decode_configuration_values(encoded, &schema).expect("default values");
            assert_eq!(values.len(), release.serialized_field_count);
            assert_eq!(
                encode_configuration_values(&values, &schema).expect("encoded defaults"),
                encoded
            );

            let identity = TuneIdentity::from_schema(release.version.clone(), &schema);
            assert_eq!(identity.validate(&release.version, &schema), Ok(()));
            assert_eq!(
                identity.validate("different-version", &schema),
                Err(ConfigurationError::IdentityMismatch)
            );
        }
    }

    #[test]
    fn schema_parser_rejects_adversarial_inputs() {
        assert_eq!(
            decode_qt_compressed(&[0, 0]),
            Err(ConfigurationError::TruncatedEnvelope)
        );
        let prohibited = br#"<?xml version="1.0"?><!DOCTYPE x [<!ENTITY e SYSTEM "file:///etc/passwd">]><ConfigParams>&e;</ConfigParams>"#;
        assert_eq!(
            parse_configuration_xml(prohibited),
            Err(ConfigurationError::ProhibitedXml)
        );
        let duplicate_field =
            br#"<ConfigParams><Params><x/><x/></Params><SerOrder/><Grouping/></ConfigParams>"#;
        assert_eq!(
            parse_configuration_xml(duplicate_field),
            Err(ConfigurationError::DuplicateField)
        );
        let duplicate_order = br#"<ConfigParams><Params><x><type>5</type></x></Params><SerOrder><ser>x</ser><ser>x</ser></SerOrder><Grouping/></ConfigParams>"#;
        assert_eq!(
            parse_configuration_xml(duplicate_order),
            Err(ConfigurationError::DuplicateOrder)
        );
        let missing = br#"<ConfigParams><Params/><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>"#;
        assert_eq!(
            parse_configuration_xml(missing),
            Err(ConfigurationError::MissingReference)
        );
        let unsupported_type = br#"<ConfigParams><Params><x><type>99</type></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>"#;
        assert_eq!(
            parse_configuration_xml(unsupported_type),
            Err(ConfigurationError::UnsupportedType)
        );
        let unsupported_encoding = br#"<ConfigParams><Params><x><type>1</type><vTx>99</vTx></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>"#;
        assert_eq!(
            parse_configuration_xml(unsupported_encoding),
            Err(ConfigurationError::UnsupportedEncoding)
        );
        let invalid_number = br#"<ConfigParams><Params><x><type>1</type><vTx>7</vTx><valDouble>NaN</valDouble></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>"#;
        assert_eq!(
            parse_configuration_xml(invalid_number),
            Err(ConfigurationError::InvalidNumber)
        );
        let nested = format!(
            "<ConfigParams>{}{}</ConfigParams>",
            "<x>".repeat(40),
            "</x>".repeat(40)
        );
        assert_eq!(
            parse_configuration_xml(nested.as_bytes()),
            Err(ConfigurationError::StructuralLimit)
        );
    }

    #[test]
    fn configuration_routes_chunks_and_write_readback_are_exact() {
        assert_eq!(schema_request(400, 0), hex("5c000000019000000000"));
        assert_eq!(current_values_request(), [0x5d, 0]);
        assert_eq!(default_values_request(), [0x5e, 0]);

        let schema = decode_qt_compressed(schema_fixture("1.2.3")).expect("schema");
        let write =
            include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-write.bin");
        let readback =
            include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-readback.bin");
        assert_eq!(&write_values_request(write)[..6], hex("5f0090b7a9ba"));
        let intended = decode_configuration_values(write, &schema).expect("write");
        let accepted = decode_configuration_values(readback, &schema).expect("readback");
        let current = decode_configuration_values(
            include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-current.bin"),
            &schema,
        )
        .expect("current");
        assert_eq!(
            verify_write_readback(true, &accepted, &accepted),
            WriteReadbackResult::Success
        );
        assert_eq!(
            verify_write_readback(true, &intended, &current),
            WriteReadbackResult::Conflict
        );

        let compressed = schema_fixture("1.2.3");
        let mut assembler = SchemaChunkAssembler::new(compressed.len()).expect("assembler");
        let mut offset = 0usize;
        for chunk in compressed.chunks(400) {
            let mut response = vec![CONFIG_SCHEMA_COMMAND, 0];
            response.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
            response.extend_from_slice(&(offset as u32).to_be_bytes());
            response.extend_from_slice(chunk);
            offset += chunk.len();
            assert_eq!(
                assembler.accept_response(&response),
                Ok(offset == compressed.len())
            );
        }
        assert_eq!(assembler.finish().expect("assembled"), compressed);

        let mut wrong_index = vec![CONFIG_SCHEMA_COMMAND, 1];
        wrong_index.extend_from_slice(&1_u32.to_be_bytes());
        wrong_index.extend_from_slice(&0_u32.to_be_bytes());
        wrong_index.push(0);
        assert_eq!(
            SchemaChunkAssembler::new(1)
                .expect("assembler")
                .accept_response(&wrong_index),
            Err(ConfigurationError::UnexpectedIndex)
        );
    }

    #[test]
    fn values_reject_signature_remainder_and_non_exact_tune_identity() {
        let schema = decode_qt_compressed(schema_fixture("1.2.3")).expect("schema");
        let mut mismatch = default_fixture("1.2.3").to_vec();
        mismatch[0] ^= 0xff;
        assert_eq!(
            decode_configuration_values(&mismatch, &schema),
            Err(ConfigurationError::SignatureMismatch)
        );
        let mut remainder = default_fixture("1.2.3").to_vec();
        remainder.push(0);
        assert_eq!(
            decode_configuration_values(&remainder, &schema),
            Err(ConfigurationError::Remainder)
        );
        let values =
            decode_configuration_values(default_fixture("1.2.3"), &schema).expect("values");
        let mut tune = Tune {
            identity: TuneIdentity::from_schema("1.2.3", &schema),
            values,
        };
        assert_eq!(
            tune.encode_for("1.2.3", &schema).expect("encoded tune"),
            default_fixture("1.2.3")
        );
        tune.identity.schema_fingerprint.push('0');
        assert_eq!(
            tune.encode_for("1.2.3", &schema),
            Err(ConfigurationError::IdentityMismatch)
        );
    }

    fn hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("hex"), 16).expect("hex")
            })
            .collect()
    }
}
