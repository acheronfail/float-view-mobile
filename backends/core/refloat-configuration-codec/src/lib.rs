//! Bounded Refloat `ConfigParams` schema and signed-value semantics.
//!
//! This safe candidate deliberately owns no command transport, schema chunk
//! assembly, compatibility catalogue, configuration lifecycle, persistence,
//! host ABI, or OS behavior. Swift and Kotlin remain selected until separate
//! host-cutover changes.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use flate2::{Decompress, FlushDecompress, Status};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use sha2::{Digest, Sha256};

mod records;

pub use records::{
    MAX_SCHEMA_RECORD_BYTES, MAX_VALUE_PAYLOAD_BYTES, MAX_VALUE_RECORD_BYTES, RecordError,
    decode_qt_compressed_to_record, decode_schema_record, decode_value_record,
    decode_values_to_record, encode_schema_record, encode_value_record, encode_values_from_records,
};

/// Maximum complete Qt-compressed schema bytes, including its length prefix.
pub const MAX_COMPRESSED_BYTES: usize = 32 * 1024;
/// Maximum expanded `ConfigParams` XML bytes.
pub const MAX_EXPANDED_BYTES: usize = 384 * 1024;
/// Maximum parameter definitions in one schema.
pub const MAX_FIELDS: usize = 512;
/// Maximum presentation groups in one schema.
pub const MAX_GROUPS: usize = 16;
/// Maximum presentation subgroups across all groups.
pub const MAX_SUBGROUPS: usize = 64;
/// Maximum choices for one enumeration.
pub const MAX_ENUM_CHOICES: usize = 128;
/// Maximum UTF-8 bytes in a serialized field name.
pub const MAX_NAME_BYTES: usize = 128;
/// Maximum UTF-8 bytes in a normalized field description.
pub const MAX_DESCRIPTION_BYTES: usize = 4096;
/// Maximum UTF-8 bytes in one serialized string value.
pub const MAX_STRING_VALUE_BYTES: usize = 4096;
/// Maximum XML element nesting depth.
pub const MAX_XML_DEPTH: usize = 32;
/// Maximum XML start/empty elements.
pub const MAX_XML_ELEMENTS: usize = 12_000;
/// Maximum accumulated decoded text for one XML element.
const MAX_ELEMENT_TEXT_BYTES: usize = MAX_DESCRIPTION_BYTES * 2;

/// Stable, payload-free configuration codec failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationError {
    /// The Qt compression envelope lacks its four-byte length.
    TruncatedEnvelope,
    /// The complete compressed input exceeds its bound.
    CompressedTooLarge,
    /// The declared or direct expanded input exceeds its bound.
    ExpandedTooLarge,
    /// The zlib output or input consumption differs from the Qt envelope.
    ExpandedLengthMismatch,
    /// The enclosed zlib stream is invalid or incomplete.
    DecompressionFailed,
    /// The document contains a DTD or entity declaration.
    ProhibitedXml,
    /// XML is malformed, non-UTF-8, or structurally unlike `ConfigParams`.
    MalformedXml,
    /// A bounded collection, depth, element, or text extent was exceeded.
    StructuralLimit,
    /// A parameter name was declared more than once.
    DuplicateField,
    /// A serialization-order name was listed more than once.
    DuplicateOrder,
    /// A serialization-order entry has no parameter definition.
    MissingReference,
    /// A parameter has an unsupported semantic type.
    UnsupportedType,
    /// A numeric parameter has an unsupported or missing wire encoding.
    UnsupportedEncoding,
    /// Required metadata is empty or outside its byte bound.
    InvalidMetadata,
    /// Numeric metadata is absent, malformed, or non-finite.
    InvalidNumber,
    /// Serialized values carry another schema's signature.
    SignatureMismatch,
    /// Serialized values end before the complete schema order.
    TruncatedValues,
    /// Serialized values contain bytes after the complete schema order.
    Remainder,
    /// An encoded value set omits an expected field.
    MissingValue,
    /// An encoded value set contains an unknown field.
    ExtraValue,
    /// A decoded or encoded value is invalid for its field.
    InvalidValue,
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::TruncatedEnvelope => "truncated compression envelope",
            Self::CompressedTooLarge => "compressed schema exceeds bound",
            Self::ExpandedTooLarge => "expanded schema exceeds bound",
            Self::ExpandedLengthMismatch => "expanded schema length mismatch",
            Self::DecompressionFailed => "schema decompression failed",
            Self::ProhibitedXml => "prohibited XML construct",
            Self::MalformedXml => "malformed configuration XML",
            Self::StructuralLimit => "configuration structural limit exceeded",
            Self::DuplicateField => "duplicate configuration field",
            Self::DuplicateOrder => "duplicate serialization order entry",
            Self::MissingReference => "missing serialization field reference",
            Self::UnsupportedType => "unsupported configuration field type",
            Self::UnsupportedEncoding => "unsupported configuration wire encoding",
            Self::InvalidMetadata => "invalid configuration metadata",
            Self::InvalidNumber => "invalid configuration number",
            Self::SignatureMismatch => "configuration signature mismatch",
            Self::TruncatedValues => "truncated configuration values",
            Self::Remainder => "configuration values contain a remainder",
            Self::MissingValue => "configuration value is missing",
            Self::ExtraValue => "configuration value is extra",
            Self::InvalidValue => "invalid configuration value",
        })
    }
}

impl std::error::Error for ConfigurationError {}

/// Supported source `ConfigParam` semantic types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FieldType {
    /// Scaled finite decimal.
    Decimal = 1,
    /// Signed or unsigned integral value.
    Integer = 2,
    /// NUL-terminated UTF-8 string.
    String = 3,
    /// Signed-byte choice index.
    Enumeration = 4,
    /// Strict byte `0` or `1`.
    Boolean = 5,
    /// Unsigned-byte bit mask.
    Bitfield = 6,
}

impl FieldType {
    fn from_wire(value: i64) -> Option<Self> {
        Some(match value {
            1 => Self::Decimal,
            2 => Self::Integer,
            3 => Self::String,
            4 => Self::Enumeration,
            5 => Self::Boolean,
            6 => Self::Bitfield,
            _ => return None,
        })
    }
}

/// Supported VESC configuration wire encodings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WireType {
    /// One-byte unsigned integer.
    Uint8 = 1,
    /// One-byte signed integer.
    Int8 = 2,
    /// Two-byte unsigned integer.
    Uint16 = 3,
    /// Two-byte signed integer.
    Int16 = 4,
    /// Four-byte unsigned integer.
    Uint32 = 5,
    /// Four-byte signed integer.
    Int32 = 6,
    /// Two-byte signed scaled decimal.
    Decimal16 = 7,
    /// Four-byte signed scaled decimal.
    Decimal32 = 8,
    /// Source-declared automatic decimal encoding; not serialized here.
    Decimal32Auto = 9,
}

impl WireType {
    fn from_wire(value: i64) -> Option<Self> {
        Some(match value {
            1 => Self::Uint8,
            2 => Self::Int8,
            3 => Self::Uint16,
            4 => Self::Int16,
            5 => Self::Uint32,
            6 => Self::Int32,
            7 => Self::Decimal16,
            8 => Self::Decimal32,
            9 => Self::Decimal32Auto,
            _ => return None,
        })
    }
}

/// One normalized configuration value.
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigurationValue {
    /// Finite decimal value.
    Decimal(f64),
    /// Integral, enumeration, or bitfield value.
    Integer(i64),
    /// Strict boolean value.
    Boolean(bool),
    /// Bounded UTF-8 string value.
    String(String),
}

/// One normalized serialized field definition.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigurationField {
    /// Exact serialized identity.
    pub name: String,
    /// Human-readable label.
    pub long_name: String,
    /// Closed semantic type.
    pub field_type: FieldType,
    /// Numeric wire encoding, when declared.
    pub wire_type: Option<WireType>,
    /// Schema default value.
    pub default_value: ConfigurationValue,
    /// Optional inclusive minimum.
    pub minimum: Option<f64>,
    /// Optional inclusive maximum.
    pub maximum: Option<f64>,
    /// Optional editor step.
    pub step: Option<f64>,
    /// Optional editor precision.
    pub precision: Option<i32>,
    /// Optional fixed-point wire scale.
    pub scale: Option<f64>,
    /// Display suffix.
    pub suffix: String,
    /// Ordered enumeration labels.
    pub choices: Vec<String>,
    /// Sanitized human-readable description.
    pub description: String,
}

/// One normalized presentation subgroup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationSubgroup {
    /// Display name.
    pub name: String,
    /// Ordered field or separator identities.
    pub fields: Vec<String>,
}

/// One normalized presentation group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationGroup {
    /// Display name.
    pub name: String,
    /// Ordered subgroups.
    pub subgroups: Vec<ConfigurationSubgroup>,
}

/// One complete normalized schema.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigurationSchema {
    /// Lowercase SHA-256 of complete validated expanded XML bytes.
    pub fingerprint: String,
    /// CRC32C of the ordered wire schema identity.
    pub wire_signature: u32,
    /// Serialized fields in exact wire order.
    pub fields: Vec<ConfigurationField>,
    /// Presentation groups in document order.
    pub groups: Vec<ConfigurationGroup>,
}

/// An ordered complete value set.
pub type ConfigurationValues = Vec<(String, ConfigurationValue)>;

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
struct SchemaBuilder {
    fields: Vec<(String, RawField)>,
    order: Vec<String>,
    groups: Vec<ConfigurationGroup>,
    current_field_name: Option<String>,
    current_field: RawField,
    current_group_name: Option<String>,
    current_subgroups: Vec<ConfigurationSubgroup>,
    current_subgroup_name: Option<String>,
    current_subgroup_fields: Vec<String>,
}

/// Decode and normalize one complete Qt `qCompress` schema.
pub fn decode_qt_compressed(input: &[u8]) -> Result<ConfigurationSchema, ConfigurationError> {
    if input.len() < 4 {
        return Err(ConfigurationError::TruncatedEnvelope);
    }
    if input.len() > MAX_COMPRESSED_BYTES {
        return Err(ConfigurationError::CompressedTooLarge);
    }
    let expected = u32::from_be_bytes(input[..4].try_into().expect("checked length")) as usize;
    if expected > MAX_EXPANDED_BYTES {
        return Err(ConfigurationError::ExpandedTooLarge);
    }

    let compressed = &input[4..];
    let mut output = vec![0; expected];
    let mut decompressor = Decompress::new(true);
    let status = decompressor
        .decompress(compressed, &mut output, FlushDecompress::Finish)
        .map_err(|_| ConfigurationError::DecompressionFailed)?;
    if status != Status::StreamEnd {
        return Err(ConfigurationError::DecompressionFailed);
    }
    if decompressor.total_out() != expected as u64
        || decompressor.total_in() != compressed.len() as u64
    {
        return Err(ConfigurationError::ExpandedLengthMismatch);
    }
    parse_schema(&output)
}

/// Validate and normalize one complete expanded `ConfigParams` document.
pub fn parse_schema(xml: &[u8]) -> Result<ConfigurationSchema, ConfigurationError> {
    if xml.len() > MAX_EXPANDED_BYTES {
        return Err(ConfigurationError::ExpandedTooLarge);
    }
    let xml_text = std::str::from_utf8(xml).map_err(|_| ConfigurationError::MalformedXml)?;
    let lowercase = xml_text.to_ascii_lowercase();
    if lowercase.contains("<!doctype") || lowercase.contains("<!entity") {
        return Err(ConfigurationError::ProhibitedXml);
    }

    let mut reader = Reader::from_str(xml_text);
    reader.config_mut().expand_empty_elements = true;
    reader.config_mut().check_comments = true;
    let mut stack: Vec<String> = Vec::new();
    let mut contents = String::new();
    let mut element_count = 0usize;
    let mut builder = SchemaBuilder::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) => {
                let name = std::str::from_utf8(start.name().as_ref())
                    .map_err(|_| ConfigurationError::MalformedXml)?
                    .to_owned();
                element_count = element_count
                    .checked_add(1)
                    .ok_or(ConfigurationError::StructuralLimit)?;
                stack.push(name.clone());
                contents.clear();
                if stack.len() > MAX_XML_DEPTH || element_count > MAX_XML_ELEMENTS {
                    return Err(ConfigurationError::StructuralLimit);
                }
                if stack.len() == 3 && stack[0] == "ConfigParams" && stack[1] == "Params" {
                    builder.current_field_name = Some(name);
                    builder.current_field = RawField::default();
                } else if stack_ends_with(&stack, &["Grouping", "group"]) {
                    builder.current_group_name = None;
                    builder.current_subgroups.clear();
                } else if stack_ends_with(&stack, &["group", "subgroup"]) {
                    builder.current_subgroup_name = None;
                    builder.current_subgroup_fields.clear();
                }
            }
            Ok(Event::End(end)) => {
                let end_name = end.name();
                let name = std::str::from_utf8(end_name.as_ref())
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                let Some(opened) = stack.last() else {
                    return Err(ConfigurationError::MalformedXml);
                };
                if opened != name {
                    return Err(ConfigurationError::MalformedXml);
                }
                accept_element(&stack, name, &contents, &mut builder)?;
                stack.pop();
                contents.clear();
            }
            Ok(Event::Text(text)) => {
                let decoded = text
                    .xml10_content()
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                append_text(&mut contents, decoded.as_ref())?;
            }
            Ok(Event::CData(text)) => {
                let decoded = text
                    .xml10_content()
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                append_text(&mut contents, decoded.as_ref())?;
            }
            Ok(Event::GeneralRef(reference)) => {
                if let Some(character) = reference
                    .resolve_char_ref()
                    .map_err(|_| ConfigurationError::MalformedXml)?
                {
                    append_text(&mut contents, character.encode_utf8(&mut [0; 4]))?;
                    continue;
                }
                let decoded = reference
                    .decode()
                    .map_err(|_| ConfigurationError::MalformedXml)?;
                let resolved = match decoded.as_ref() {
                    "lt" => "<",
                    "gt" => ">",
                    "amp" => "&",
                    "apos" => "'",
                    "quot" => "\"",
                    _ => return Err(ConfigurationError::MalformedXml),
                };
                append_text(&mut contents, resolved)?;
            }
            Ok(Event::DocType(_)) => return Err(ConfigurationError::ProhibitedXml),
            Ok(Event::Decl(_) | Event::PI(_) | Event::Comment(_)) => {}
            Ok(Event::Eof) => break,
            Ok(_) => return Err(ConfigurationError::MalformedXml),
            Err(_) => return Err(ConfigurationError::MalformedXml),
        }
    }

    if !stack.is_empty() || element_count == 0 {
        return Err(ConfigurationError::MalformedXml);
    }
    finish_schema(xml, builder)
}

fn append_text(target: &mut String, value: &str) -> Result<(), ConfigurationError> {
    let length = target
        .len()
        .checked_add(value.len())
        .ok_or(ConfigurationError::StructuralLimit)?;
    if length > MAX_ELEMENT_TEXT_BYTES {
        return Err(ConfigurationError::StructuralLimit);
    }
    target.push_str(value);
    Ok(())
}

fn stack_ends_with(stack: &[String], suffix: &[&str]) -> bool {
    stack.len() >= suffix.len()
        && stack[stack.len() - suffix.len()..]
            .iter()
            .map(String::as_str)
            .eq(suffix.iter().copied())
}

fn accept_element(
    stack: &[String],
    name: &str,
    contents: &str,
    builder: &mut SchemaBuilder,
) -> Result<(), ConfigurationError> {
    if stack.len() == 4 && stack[0] == "ConfigParams" && stack[1] == "Params" {
        builder
            .current_field
            .values
            .entry(name.to_owned())
            .or_default()
            .push(contents.to_owned());
    } else if stack.len() == 3
        && stack[0] == "ConfigParams"
        && stack[1] == "SerOrder"
        && stack[2] == "ser"
    {
        builder.order.push(contents.to_owned());
    } else if stack_ends_with(stack, &["group", "subgroup", "subgroupName"]) {
        builder.current_subgroup_name = Some(contents.to_owned());
    } else if stack_ends_with(stack, &["subgroupParams", "param"]) {
        builder.current_subgroup_fields.push(contents.to_owned());
    } else if stack_ends_with(stack, &["group", "groupName"]) {
        builder.current_group_name = Some(contents.to_owned());
    }

    if stack.len() == 3 && stack[0] == "ConfigParams" && stack[1] == "Params" {
        let field_name = builder
            .current_field_name
            .take()
            .ok_or(ConfigurationError::MalformedXml)?;
        builder
            .fields
            .push((field_name, std::mem::take(&mut builder.current_field)));
    } else if stack_ends_with(stack, &["group", "subgroup"]) {
        if builder.current_subgroups.len() >= MAX_SUBGROUPS {
            return Err(ConfigurationError::StructuralLimit);
        }
        builder.current_subgroups.push(ConfigurationSubgroup {
            name: builder.current_subgroup_name.take().unwrap_or_default(),
            fields: std::mem::take(&mut builder.current_subgroup_fields),
        });
    } else if stack_ends_with(stack, &["Grouping", "group"]) {
        if builder.groups.len() >= MAX_GROUPS {
            return Err(ConfigurationError::StructuralLimit);
        }
        builder.groups.push(ConfigurationGroup {
            name: builder.current_group_name.take().unwrap_or_default(),
            subgroups: std::mem::take(&mut builder.current_subgroups),
        });
    }
    Ok(())
}

fn finish_schema(
    xml: &[u8],
    builder: SchemaBuilder,
) -> Result<ConfigurationSchema, ConfigurationError> {
    if builder.fields.len() > MAX_FIELDS
        || builder.groups.len() > MAX_GROUPS
        || builder
            .groups
            .iter()
            .map(|group| group.subgroups.len())
            .sum::<usize>()
            > MAX_SUBGROUPS
    {
        return Err(ConfigurationError::StructuralLimit);
    }
    let mut raw_by_name = HashMap::with_capacity(builder.fields.len());
    for (name, field) in builder.fields {
        if raw_by_name.insert(name, field).is_some() {
            return Err(ConfigurationError::DuplicateField);
        }
    }
    let mut seen = HashSet::with_capacity(builder.order.len());
    if builder.order.iter().any(|name| !seen.insert(name)) {
        return Err(ConfigurationError::DuplicateOrder);
    }
    let fields = builder
        .order
        .iter()
        .map(|name| {
            let raw = raw_by_name
                .get(name)
                .ok_or(ConfigurationError::MissingReference)?;
            normalize_field(name, raw)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let digest = Sha256::digest(xml);
    let mut fingerprint = String::with_capacity(64);
    for byte in digest {
        write!(&mut fingerprint, "{byte:02x}").expect("writing into String cannot fail");
    }
    Ok(ConfigurationSchema {
        wire_signature: wire_signature(&fields),
        fingerprint,
        fields,
        groups: builder.groups,
    })
}

fn normalize_field(name: &str, raw: &RawField) -> Result<ConfigurationField, ConfigurationError> {
    bounded_nonempty(name, MAX_NAME_BYTES)?;
    let field_type = raw
        .first("type")
        .parse::<i64>()
        .ok()
        .and_then(FieldType::from_wire)
        .ok_or(ConfigurationError::UnsupportedType)?;
    let wire_type = raw
        .values
        .get("vTx")
        .and_then(|values| values.first())
        .and_then(|value| value.parse::<i64>().ok())
        .and_then(WireType::from_wire);
    if matches!(field_type, FieldType::Decimal | FieldType::Integer) && wire_type.is_none() {
        return Err(ConfigurationError::UnsupportedEncoding);
    }
    let choices = raw.values.get("enumNames").cloned().unwrap_or_default();
    if choices.len() > MAX_ENUM_CHOICES {
        return Err(ConfigurationError::StructuralLimit);
    }
    let default_value = match field_type {
        FieldType::Decimal => ConfigurationValue::Decimal(required_finite(raw.first("valDouble"))?),
        FieldType::Integer | FieldType::Enumeration | FieldType::Bitfield => {
            ConfigurationValue::Integer(
                raw.first("valInt")
                    .parse()
                    .map_err(|_| ConfigurationError::InvalidNumber)?,
            )
        }
        FieldType::Boolean => match raw.first("valInt").parse::<i64>() {
            Ok(0) => ConfigurationValue::Boolean(false),
            Ok(1) => ConfigurationValue::Boolean(true),
            _ => return Err(ConfigurationError::InvalidNumber),
        },
        FieldType::String => ConfigurationValue::String(raw.first("valString").to_owned()),
    };
    let description = sanitize_description(raw.first("description"));
    bounded_nonempty(&description, MAX_DESCRIPTION_BYTES)?;
    Ok(ConfigurationField {
        name: name.to_owned(),
        long_name: raw.first("longName").to_owned(),
        field_type,
        wire_type,
        default_value,
        minimum: optional_finite(if field_type == FieldType::Decimal {
            raw.first("minDouble")
        } else {
            raw.first("minInt")
        })?,
        maximum: optional_finite(if field_type == FieldType::Decimal {
            raw.first("maxDouble")
        } else {
            raw.first("maxInt")
        })?,
        step: optional_finite(if field_type == FieldType::Decimal {
            raw.first("stepDouble")
        } else {
            raw.first("stepInt")
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
    let value = value
        .parse::<f64>()
        .map_err(|_| ConfigurationError::InvalidNumber)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ConfigurationError::InvalidNumber)
    }
}

fn optional_finite(value: &str) -> Result<Option<f64>, ConfigurationError> {
    if value.is_empty() {
        Ok(None)
    } else {
        required_finite(value).map(Some)
    }
}

fn sanitize_description(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut remaining = value;
    let mut hidden = None;
    while !remaining.is_empty() {
        let text_end = remaining.find('<').unwrap_or(remaining.len());
        if hidden.is_none() {
            // Source newlines are HTML whitespace; only block tags create breaks.
            result.extend(remaining[..text_end].chars().map(|character| {
                if character.is_whitespace() {
                    ' '
                } else {
                    character
                }
            }));
        }
        remaining = &remaining[text_end..];
        if remaining.is_empty() {
            break;
        }
        let Some(tag_end) = remaining.find('>') else {
            break;
        };
        let tag = remaining[1..tag_end].trim();
        let closing = tag.starts_with('/');
        let name = tag
            .trim_start_matches('/')
            .split([' ', '\t', '\r', '\n', '/'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        remaining = &remaining[tag_end + 1..];
        if let Some(hidden_name) = hidden {
            if closing && name == hidden_name {
                hidden = None;
            }
            continue;
        }
        match name.as_str() {
            "head" | "style" | "script" if !closing => {
                hidden = Some(match name.as_str() {
                    "head" => "head",
                    "style" => "style",
                    _ => "script",
                })
            }
            "p" | "div" | "ul" | "ol" => result.push_str("\n\n"),
            "br" => result.push('\n'),
            "li" if !closing => {
                result.push('\n');
                result.push_str("• ");
            }
            _ => {}
        }
    }
    let mut normalized = String::with_capacity(result.len());
    let mut breaks = 0;
    for line in result.split('\n') {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !line.is_empty() {
            if !normalized.is_empty() {
                normalized.push_str(if breaks > 1 { "\n\n" } else { "\n" });
            }
            normalized.push_str(&line);
            breaks = 0;
        }
        breaks += 1;
    }
    normalized
}

/// Compute the exact Refloat/VESC CRC32C wire signature for ordered fields.
#[must_use]
pub fn wire_signature(fields: &[ConfigurationField]) -> u32 {
    let mut identity = String::new();
    for field in fields {
        identity.push_str(&field.name);
        write!(&mut identity, "{}", field.field_type as u8)
            .expect("writing into String cannot fail");
        write!(
            &mut identity,
            "{}",
            field.wire_type.map_or(0, |value| value as u8)
        )
        .expect("writing into String cannot fail");
        for choice in &field.choices {
            identity.push_str(choice);
        }
    }
    crc32c(identity.as_bytes())
}

fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0x82f6_3b78 & mask);
        }
    }
    !crc
}

struct ValueReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ValueReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

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
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte)))
    }

    fn signed(&mut self, count: usize) -> Result<i64, ConfigurationError> {
        let value = self.unsigned(count)?;
        let shift = 64 - count * 8;
        Ok(((value << shift) as i64) >> shift)
    }
}

/// Decode a complete signature-prefixed value set against one exact schema.
pub fn decode_values(
    data: &[u8],
    schema: &ConfigurationSchema,
) -> Result<ConfigurationValues, ConfigurationError> {
    let mut reader = ValueReader::new(data);
    if reader.unsigned(4)? != u64::from(schema.wire_signature) {
        return Err(ConfigurationError::SignatureMismatch);
    }
    let values = schema
        .fields
        .iter()
        .map(|field| Ok((field.name.clone(), decode_value(field, &mut reader)?)))
        .collect::<Result<Vec<_>, ConfigurationError>>()?;
    if reader.remaining() != 0 {
        return Err(ConfigurationError::Remainder);
    }
    Ok(values)
}

fn decode_value(
    field: &ConfigurationField,
    reader: &mut ValueReader<'_>,
) -> Result<ConfigurationValue, ConfigurationError> {
    match field.field_type {
        FieldType::Decimal => {
            let scale = field
                .scale
                .filter(|value| *value > 0.0)
                .ok_or(ConfigurationError::InvalidMetadata)?;
            let raw = match field.wire_type {
                Some(WireType::Decimal16) => reader.signed(2)?,
                Some(WireType::Decimal32) => reader.signed(4)?,
                _ => return Err(ConfigurationError::UnsupportedEncoding),
            };
            Ok(ConfigurationValue::Decimal(raw as f64 / scale))
        }
        FieldType::Integer => {
            let raw = match field.wire_type {
                Some(WireType::Uint8) => reader.unsigned(1)? as i64,
                Some(WireType::Int8) => reader.signed(1)?,
                Some(WireType::Uint16) => reader.unsigned(2)? as i64,
                Some(WireType::Int16) => reader.signed(2)?,
                Some(WireType::Uint32) => reader.unsigned(4)? as i64,
                Some(WireType::Int32) => reader.signed(4)?,
                _ => return Err(ConfigurationError::UnsupportedEncoding),
            };
            Ok(ConfigurationValue::Integer(raw))
        }
        FieldType::Enumeration => Ok(ConfigurationValue::Integer(reader.signed(1)?)),
        FieldType::Bitfield => Ok(ConfigurationValue::Integer(reader.unsigned(1)? as i64)),
        FieldType::Boolean => match reader.signed(1)? {
            0 => Ok(ConfigurationValue::Boolean(false)),
            1 => Ok(ConfigurationValue::Boolean(true)),
            _ => Err(ConfigurationError::InvalidValue),
        },
        FieldType::String => {
            let remaining = &reader.bytes[reader.offset..];
            let length = remaining
                .iter()
                .position(|byte| *byte == 0)
                .ok_or(ConfigurationError::TruncatedValues)?;
            if length > MAX_STRING_VALUE_BYTES {
                return Err(ConfigurationError::InvalidValue);
            }
            let bytes = reader.take(length + 1)?;
            let value = std::str::from_utf8(&bytes[..length])
                .map_err(|_| ConfigurationError::InvalidValue)?;
            Ok(ConfigurationValue::String(value.to_owned()))
        }
    }
}

/// Encode one exact complete value set against one exact schema.
pub fn encode_values(
    values: &[(String, ConfigurationValue)],
    schema: &ConfigurationSchema,
) -> Result<Vec<u8>, ConfigurationError> {
    let mut provided = HashMap::with_capacity(values.len());
    for (name, value) in values {
        if provided.insert(name.as_str(), value).is_some() {
            return Err(ConfigurationError::ExtraValue);
        }
    }
    if schema
        .fields
        .iter()
        .any(|field| !provided.contains_key(field.name.as_str()))
    {
        return Err(ConfigurationError::MissingValue);
    }
    if provided
        .keys()
        .any(|name| !schema.fields.iter().any(|field| field.name == *name))
    {
        return Err(ConfigurationError::ExtraValue);
    }
    let mut output = Vec::new();
    output.extend_from_slice(&schema.wire_signature.to_be_bytes());
    for field in &schema.fields {
        encode_value(
            provided
                .get(field.name.as_str())
                .expect("complete value set checked"),
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
        (FieldType::Decimal, ConfigurationValue::Decimal(number)) => {
            let scale = field
                .scale
                .filter(|value| *value > 0.0)
                .ok_or(ConfigurationError::InvalidMetadata)?;
            if !number.is_finite() || !within(*number, field) {
                return Err(ConfigurationError::InvalidValue);
            }
            let scaled = number * scale;
            if !scaled.is_finite() || scaled.round() != scaled {
                return Err(ConfigurationError::InvalidValue);
            }
            let raw = scaled as i64;
            match field.wire_type {
                Some(WireType::Decimal16) if i16::try_from(raw).is_ok() => {
                    append_integer(raw, 2, output)
                }
                Some(WireType::Decimal32) if i32::try_from(raw).is_ok() => {
                    append_integer(raw, 4, output)
                }
                _ => return Err(ConfigurationError::InvalidValue),
            }
        }
        (FieldType::Integer, ConfigurationValue::Integer(number)) => {
            if !within(*number as f64, field) {
                return Err(ConfigurationError::InvalidValue);
            }
            let (minimum, maximum, count) = match field.wire_type {
                Some(WireType::Uint8) => (0, 255, 1),
                Some(WireType::Int8) => (-128, 127, 1),
                Some(WireType::Uint16) => (0, 65_535, 2),
                Some(WireType::Int16) => (-32_768, 32_767, 2),
                Some(WireType::Uint32) => (0, 4_294_967_295, 4),
                Some(WireType::Int32) => (-2_147_483_648, 2_147_483_647, 4),
                _ => return Err(ConfigurationError::UnsupportedEncoding),
            };
            if *number < minimum || *number > maximum {
                return Err(ConfigurationError::InvalidValue);
            }
            append_integer(*number, count, output);
        }
        (FieldType::Enumeration, ConfigurationValue::Integer(number)) => {
            let choice = usize::try_from(*number).map_err(|_| ConfigurationError::InvalidValue)?;
            if choice >= field.choices.len() || *number > i64::from(i8::MAX) {
                return Err(ConfigurationError::InvalidValue);
            }
            append_integer(*number, 1, output);
        }
        (FieldType::Bitfield, ConfigurationValue::Integer(number)) => {
            if !(0..=255).contains(number) {
                return Err(ConfigurationError::InvalidValue);
            }
            append_integer(*number, 1, output);
        }
        (FieldType::Boolean, ConfigurationValue::Boolean(value)) => {
            output.push(u8::from(*value));
        }
        (FieldType::String, ConfigurationValue::String(value)) => {
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

fn within(value: f64, field: &ConfigurationField) -> bool {
    field.minimum.is_none_or(|minimum| value >= minimum)
        && field.maximum.is_none_or(|maximum| value <= maximum)
}

fn append_integer(value: i64, count: usize, output: &mut Vec<u8>) {
    let bytes = value.to_be_bytes();
    output.extend_from_slice(&bytes[bytes.len() - count..]);
}
