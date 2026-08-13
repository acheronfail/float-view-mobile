use std::collections::HashSet;
use std::fmt::Write as _;

use crate::{
    ConfigurationError, ConfigurationField, ConfigurationGroup, ConfigurationSchema,
    ConfigurationSubgroup, ConfigurationValue, ConfigurationValues, FieldType,
    MAX_DESCRIPTION_BYTES, MAX_ENUM_CHOICES, MAX_FIELDS, MAX_GROUPS, MAX_NAME_BYTES,
    MAX_STRING_VALUE_BYTES, MAX_SUBGROUPS, WireType, decode_qt_compressed, decode_values,
    encode_values, wire_signature,
};

/// Maximum serialized canonical schema record bytes.
pub const MAX_SCHEMA_RECORD_BYTES: usize = 1024 * 1024;
/// Maximum serialized canonical values record bytes.
pub const MAX_VALUE_RECORD_BYTES: usize = 3 * 1024 * 1024;
/// Maximum signature-prefixed Refloat configuration value payload bytes.
pub const MAX_VALUE_PAYLOAD_BYTES: usize = 4 + MAX_FIELDS * (MAX_STRING_VALUE_BYTES + 1);

const RECORD_VERSION: u8 = 1;
const SCHEMA_KIND: u8 = 1;
const VALUES_KIND: u8 = 2;
const SCHEMA_HEADER_BYTES: usize = 48;
const VALUES_HEADER_BYTES: usize = 12;
const MAX_RECORD_TEXT_BYTES: usize = MAX_DESCRIPTION_BYTES;

const VALUE_DECIMAL: u8 = 1;
const VALUE_INTEGER: u8 = 2;
const VALUE_BOOLEAN: u8 = 3;
const VALUE_STRING: u8 = 4;

/// Stable failures at the canonical-record boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordError {
    /// A schema record is malformed, non-canonical, or outside its bound.
    InvalidSchemaRecord,
    /// A value record is malformed, non-canonical, or outside its bound.
    InvalidValueRecord,
    /// The underlying safe protocol codec rejected the operation.
    Codec(ConfigurationError),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSchemaRecord => formatter.write_str("invalid canonical schema record"),
            Self::InvalidValueRecord => formatter.write_str("invalid canonical value record"),
            Self::Codec(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RecordError {}

/// Decode a Qt-compressed schema and serialize its canonical record.
pub fn decode_qt_compressed_to_record(input: &[u8]) -> Result<Vec<u8>, RecordError> {
    let schema = decode_qt_compressed(input).map_err(RecordError::Codec)?;
    encode_schema_record(&schema)
}

/// Decode values using one exact canonical schema record and serialize a value record.
pub fn decode_values_to_record(
    schema_record: &[u8],
    payload: &[u8],
) -> Result<Vec<u8>, RecordError> {
    if payload.len() > MAX_VALUE_PAYLOAD_BYTES {
        return Err(RecordError::Codec(ConfigurationError::InvalidValue));
    }
    let schema = decode_schema_record(schema_record)?;
    let values = decode_values(payload, &schema).map_err(RecordError::Codec)?;
    encode_value_record(&values)
}

/// Parse exact canonical schema and value records and encode Refloat wire bytes.
pub fn encode_values_from_records(
    schema_record: &[u8],
    value_record: &[u8],
) -> Result<Vec<u8>, RecordError> {
    let schema = decode_schema_record(schema_record)?;
    let values = decode_value_record(value_record)?;
    let payload = encode_values(&values, &schema).map_err(RecordError::Codec)?;
    if payload.len() > MAX_VALUE_PAYLOAD_BYTES {
        return Err(RecordError::Codec(ConfigurationError::InvalidValue));
    }
    Ok(payload)
}

/// Serialize one complete normalized schema into the version-1 canonical record.
pub fn encode_schema_record(schema: &ConfigurationSchema) -> Result<Vec<u8>, RecordError> {
    validate_schema(schema)?;
    let field_count =
        u16::try_from(schema.fields.len()).map_err(|_| RecordError::InvalidSchemaRecord)?;
    let group_count =
        u16::try_from(schema.groups.len()).map_err(|_| RecordError::InvalidSchemaRecord)?;
    let fingerprint = decode_fingerprint(&schema.fingerprint)?;
    let mut output = Vec::with_capacity(SCHEMA_HEADER_BYTES);
    output.extend_from_slice(&[RECORD_VERSION, SCHEMA_KIND, 0, 0]);
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(&field_count.to_be_bytes());
    output.extend_from_slice(&group_count.to_be_bytes());
    output.extend_from_slice(&schema.wire_signature.to_be_bytes());
    output.extend_from_slice(&fingerprint);

    for field in &schema.fields {
        push_string(&mut output, &field.name, MAX_NAME_BYTES, false, true)?;
        push_string(
            &mut output,
            &field.long_name,
            MAX_RECORD_TEXT_BYTES,
            true,
            true,
        )?;
        let mut flags = 0_u8;
        flags |= u8::from(field.minimum.is_some());
        flags |= u8::from(field.maximum.is_some()) << 1;
        flags |= u8::from(field.step.is_some()) << 2;
        flags |= u8::from(field.precision.is_some()) << 3;
        flags |= u8::from(field.scale.is_some()) << 4;
        output.extend_from_slice(&[
            field.field_type as u8,
            field.wire_type.map_or(0, |wire| wire as u8),
            value_kind(&field.default_value),
            flags,
        ]);
        push_value_payload(&mut output, &field.default_value)?;
        for value in [field.minimum, field.maximum, field.step]
            .into_iter()
            .flatten()
        {
            output.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        if let Some(precision) = field.precision {
            output.extend_from_slice(&precision.to_be_bytes());
        }
        if let Some(scale) = field.scale {
            output.extend_from_slice(&scale.to_bits().to_be_bytes());
        }
        push_string(
            &mut output,
            &field.suffix,
            MAX_RECORD_TEXT_BYTES,
            true,
            true,
        )?;
        let choice_count =
            u16::try_from(field.choices.len()).map_err(|_| RecordError::InvalidSchemaRecord)?;
        output.extend_from_slice(&choice_count.to_be_bytes());
        output.extend_from_slice(&[0, 0]);
        for choice in &field.choices {
            push_string(&mut output, choice, MAX_RECORD_TEXT_BYTES, true, true)?;
        }
        push_string(
            &mut output,
            &field.description,
            MAX_DESCRIPTION_BYTES,
            false,
            true,
        )?;
    }

    for group in &schema.groups {
        push_string(&mut output, &group.name, MAX_RECORD_TEXT_BYTES, true, true)?;
        let subgroup_count =
            u16::try_from(group.subgroups.len()).map_err(|_| RecordError::InvalidSchemaRecord)?;
        output.extend_from_slice(&subgroup_count.to_be_bytes());
        output.extend_from_slice(&[0, 0]);
        for subgroup in &group.subgroups {
            push_string(
                &mut output,
                &subgroup.name,
                MAX_RECORD_TEXT_BYTES,
                true,
                true,
            )?;
            let reference_count = u16::try_from(subgroup.fields.len())
                .map_err(|_| RecordError::InvalidSchemaRecord)?;
            output.extend_from_slice(&reference_count.to_be_bytes());
            output.extend_from_slice(&[0, 0]);
            for reference in &subgroup.fields {
                push_string(&mut output, reference, MAX_RECORD_TEXT_BYTES, true, true)?;
            }
        }
    }
    finish_record(
        &mut output,
        MAX_SCHEMA_RECORD_BYTES,
        RecordError::InvalidSchemaRecord,
    )?;
    Ok(output)
}

/// Parse and validate one exact canonical schema record.
pub fn decode_schema_record(record: &[u8]) -> Result<ConfigurationSchema, RecordError> {
    if record.len() < SCHEMA_HEADER_BYTES || record.len() > MAX_SCHEMA_RECORD_BYTES {
        return Err(RecordError::InvalidSchemaRecord);
    }
    let mut reader = RecordReader::new(record, RecordError::InvalidSchemaRecord);
    reader.header(RECORD_VERSION, SCHEMA_KIND, SCHEMA_HEADER_BYTES)?;
    let field_count = usize::from(reader.u16()?);
    let group_count = usize::from(reader.u16()?);
    if field_count > MAX_FIELDS || group_count > MAX_GROUPS {
        return Err(RecordError::InvalidSchemaRecord);
    }
    let recorded_signature = reader.u32()?;
    let fingerprint_bytes = reader.take(32)?;
    let mut fingerprint = String::with_capacity(64);
    for byte in fingerprint_bytes {
        write!(&mut fingerprint, "{byte:02x}").expect("writing into String cannot fail");
    }

    let mut fields = Vec::with_capacity(field_count);
    let mut names = HashSet::with_capacity(field_count);
    for _ in 0..field_count {
        let name = reader.string(MAX_NAME_BYTES, false)?;
        if !names.insert(name.clone()) {
            return Err(RecordError::InvalidSchemaRecord);
        }
        let long_name = reader.string(MAX_RECORD_TEXT_BYTES, true)?;
        let field_type = field_type(reader.u8()?)?;
        let wire_type = wire_type(reader.u8()?)?;
        let default_kind = reader.u8()?;
        let flags = reader.u8()?;
        if flags & !0x1f != 0 {
            return Err(RecordError::InvalidSchemaRecord);
        }
        let default_value = reader.value(default_kind, MAX_STRING_VALUE_BYTES)?;
        let minimum = reader.optional_f64(flags & 1 != 0)?;
        let maximum = reader.optional_f64(flags & 2 != 0)?;
        let step = reader.optional_f64(flags & 4 != 0)?;
        let precision = (flags & 8 != 0).then(|| reader.i32()).transpose()?;
        let scale = reader.optional_f64(flags & 16 != 0)?;
        let suffix = reader.string(MAX_RECORD_TEXT_BYTES, true)?;
        let choice_count = usize::from(reader.u16()?);
        if choice_count > MAX_ENUM_CHOICES || reader.u16()? != 0 {
            return Err(RecordError::InvalidSchemaRecord);
        }
        let choices = (0..choice_count)
            .map(|_| reader.string(MAX_RECORD_TEXT_BYTES, true))
            .collect::<Result<Vec<_>, _>>()?;
        let description = reader.string(MAX_DESCRIPTION_BYTES, false)?;
        fields.push(ConfigurationField {
            name,
            long_name,
            field_type,
            wire_type,
            default_value,
            minimum,
            maximum,
            step,
            precision,
            scale,
            suffix,
            choices,
            description,
        });
    }

    let mut groups = Vec::with_capacity(group_count);
    let mut subgroup_total = 0_usize;
    for _ in 0..group_count {
        let name = reader.string(MAX_RECORD_TEXT_BYTES, true)?;
        let subgroup_count = usize::from(reader.u16()?);
        if reader.u16()? != 0 {
            return Err(RecordError::InvalidSchemaRecord);
        }
        subgroup_total = subgroup_total
            .checked_add(subgroup_count)
            .filter(|count| *count <= MAX_SUBGROUPS)
            .ok_or(RecordError::InvalidSchemaRecord)?;
        let mut subgroups = Vec::with_capacity(subgroup_count);
        for _ in 0..subgroup_count {
            let name = reader.string(MAX_RECORD_TEXT_BYTES, true)?;
            let reference_count = usize::from(reader.u16()?);
            if reference_count > MAX_FIELDS || reader.u16()? != 0 {
                return Err(RecordError::InvalidSchemaRecord);
            }
            let fields = (0..reference_count)
                .map(|_| reader.string(MAX_RECORD_TEXT_BYTES, true))
                .collect::<Result<Vec<_>, _>>()?;
            subgroups.push(ConfigurationSubgroup { name, fields });
        }
        groups.push(ConfigurationGroup { name, subgroups });
    }
    reader.finish()?;
    let schema = ConfigurationSchema {
        fingerprint,
        wire_signature: recorded_signature,
        fields,
        groups,
    };
    validate_schema(&schema)?;
    Ok(schema)
}

/// Serialize one ordered complete value set into the version-1 canonical record.
pub fn encode_value_record(values: &ConfigurationValues) -> Result<Vec<u8>, RecordError> {
    if values.len() > MAX_FIELDS {
        return Err(RecordError::InvalidValueRecord);
    }
    let count = u16::try_from(values.len()).map_err(|_| RecordError::InvalidValueRecord)?;
    let mut output = Vec::with_capacity(VALUES_HEADER_BYTES);
    output.extend_from_slice(&[RECORD_VERSION, VALUES_KIND, 0, 0]);
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(&count.to_be_bytes());
    output.extend_from_slice(&[0, 0]);
    let mut names = HashSet::with_capacity(values.len());
    for (name, value) in values {
        if !names.insert(name.as_str()) {
            return Err(RecordError::InvalidValueRecord);
        }
        push_string(&mut output, name, MAX_NAME_BYTES, false, false)?;
        output.extend_from_slice(&[value_kind(value), 0, 0, 0]);
        push_value_payload(&mut output, value)?;
    }
    finish_record(
        &mut output,
        MAX_VALUE_RECORD_BYTES,
        RecordError::InvalidValueRecord,
    )?;
    Ok(output)
}

/// Parse and validate one exact canonical values record.
pub fn decode_value_record(record: &[u8]) -> Result<ConfigurationValues, RecordError> {
    if record.len() < VALUES_HEADER_BYTES || record.len() > MAX_VALUE_RECORD_BYTES {
        return Err(RecordError::InvalidValueRecord);
    }
    let mut reader = RecordReader::new(record, RecordError::InvalidValueRecord);
    reader.header(RECORD_VERSION, VALUES_KIND, VALUES_HEADER_BYTES)?;
    let count = usize::from(reader.u16()?);
    if count > MAX_FIELDS || reader.u16()? != 0 {
        return Err(RecordError::InvalidValueRecord);
    }
    let mut names = HashSet::with_capacity(count);
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let name = reader.string(MAX_NAME_BYTES, false)?;
        if !names.insert(name.clone()) {
            return Err(RecordError::InvalidValueRecord);
        }
        let kind = reader.u8()?;
        if reader.take(3)? != [0, 0, 0] {
            return Err(RecordError::InvalidValueRecord);
        }
        values.push((name, reader.value(kind, MAX_STRING_VALUE_BYTES)?));
    }
    reader.finish()?;
    Ok(values)
}

fn validate_schema(schema: &ConfigurationSchema) -> Result<(), RecordError> {
    if schema.fields.len() > MAX_FIELDS
        || schema.groups.len() > MAX_GROUPS
        || schema
            .groups
            .iter()
            .map(|group| group.subgroups.len())
            .sum::<usize>()
            > MAX_SUBGROUPS
        || schema.wire_signature != wire_signature(&schema.fields)
    {
        return Err(RecordError::InvalidSchemaRecord);
    }
    decode_fingerprint(&schema.fingerprint)?;
    let mut names = HashSet::with_capacity(schema.fields.len());
    for field in &schema.fields {
        validate_text(
            &field.name,
            MAX_NAME_BYTES,
            false,
            RecordError::InvalidSchemaRecord,
        )?;
        if !names.insert(field.name.as_str())
            || field.choices.len() > MAX_ENUM_CHOICES
            || !value_matches_field(&field.default_value, field.field_type)
        {
            return Err(RecordError::InvalidSchemaRecord);
        }
        validate_text(
            &field.long_name,
            MAX_RECORD_TEXT_BYTES,
            true,
            RecordError::InvalidSchemaRecord,
        )?;
        validate_text(
            &field.suffix,
            MAX_RECORD_TEXT_BYTES,
            true,
            RecordError::InvalidSchemaRecord,
        )?;
        validate_text(
            &field.description,
            MAX_DESCRIPTION_BYTES,
            false,
            RecordError::InvalidSchemaRecord,
        )?;
        for choice in &field.choices {
            validate_text(
                choice,
                MAX_RECORD_TEXT_BYTES,
                true,
                RecordError::InvalidSchemaRecord,
            )?;
        }
        if [field.minimum, field.maximum, field.step, field.scale]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite())
            || field
                .minimum
                .zip(field.maximum)
                .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            return Err(RecordError::InvalidSchemaRecord);
        }
        match field.field_type {
            FieldType::Decimal
                if !matches!(
                    field.wire_type,
                    Some(WireType::Decimal16 | WireType::Decimal32)
                ) || !field.scale.is_some_and(|scale| scale > 0.0) =>
            {
                return Err(RecordError::InvalidSchemaRecord);
            }
            FieldType::Integer
                if !matches!(
                    field.wire_type,
                    Some(
                        WireType::Uint8
                            | WireType::Int8
                            | WireType::Uint16
                            | WireType::Int16
                            | WireType::Uint32
                            | WireType::Int32
                    )
                ) =>
            {
                return Err(RecordError::InvalidSchemaRecord);
            }
            _ => {}
        }
        if let ConfigurationValue::Decimal(value) = field.default_value
            && !value.is_finite()
        {
            return Err(RecordError::InvalidSchemaRecord);
        }
        if let ConfigurationValue::String(value) = &field.default_value {
            validate_text(
                value,
                MAX_STRING_VALUE_BYTES,
                true,
                RecordError::InvalidSchemaRecord,
            )?;
        }
    }
    for group in &schema.groups {
        validate_text(
            &group.name,
            MAX_RECORD_TEXT_BYTES,
            true,
            RecordError::InvalidSchemaRecord,
        )?;
        for subgroup in &group.subgroups {
            validate_text(
                &subgroup.name,
                MAX_RECORD_TEXT_BYTES,
                true,
                RecordError::InvalidSchemaRecord,
            )?;
            if subgroup.fields.len() > MAX_FIELDS {
                return Err(RecordError::InvalidSchemaRecord);
            }
            for reference in &subgroup.fields {
                validate_text(
                    reference,
                    MAX_RECORD_TEXT_BYTES,
                    true,
                    RecordError::InvalidSchemaRecord,
                )?;
            }
        }
    }
    Ok(())
}

fn value_matches_field(value: &ConfigurationValue, field_type: FieldType) -> bool {
    matches!(
        (value, field_type),
        (ConfigurationValue::Decimal(_), FieldType::Decimal)
            | (
                ConfigurationValue::Integer(_),
                FieldType::Integer | FieldType::Enumeration | FieldType::Bitfield
            )
            | (ConfigurationValue::Boolean(_), FieldType::Boolean)
            | (ConfigurationValue::String(_), FieldType::String)
    )
}

fn field_type(value: u8) -> Result<FieldType, RecordError> {
    Ok(match value {
        1 => FieldType::Decimal,
        2 => FieldType::Integer,
        3 => FieldType::String,
        4 => FieldType::Enumeration,
        5 => FieldType::Boolean,
        6 => FieldType::Bitfield,
        _ => return Err(RecordError::InvalidSchemaRecord),
    })
}

fn wire_type(value: u8) -> Result<Option<WireType>, RecordError> {
    Ok(match value {
        0 => None,
        1 => Some(WireType::Uint8),
        2 => Some(WireType::Int8),
        3 => Some(WireType::Uint16),
        4 => Some(WireType::Int16),
        5 => Some(WireType::Uint32),
        6 => Some(WireType::Int32),
        7 => Some(WireType::Decimal16),
        8 => Some(WireType::Decimal32),
        9 => Some(WireType::Decimal32Auto),
        _ => return Err(RecordError::InvalidSchemaRecord),
    })
}

fn value_kind(value: &ConfigurationValue) -> u8 {
    match value {
        ConfigurationValue::Decimal(_) => VALUE_DECIMAL,
        ConfigurationValue::Integer(_) => VALUE_INTEGER,
        ConfigurationValue::Boolean(_) => VALUE_BOOLEAN,
        ConfigurationValue::String(_) => VALUE_STRING,
    }
}

fn push_value_payload(output: &mut Vec<u8>, value: &ConfigurationValue) -> Result<(), RecordError> {
    match value {
        ConfigurationValue::Decimal(value) if value.is_finite() => {
            output.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        ConfigurationValue::Integer(value) => output.extend_from_slice(&value.to_be_bytes()),
        ConfigurationValue::Boolean(value) => output.push(u8::from(*value)),
        ConfigurationValue::String(value) => {
            push_string(output, value, MAX_STRING_VALUE_BYTES, true, false)?;
        }
        ConfigurationValue::Decimal(_) => return Err(RecordError::InvalidValueRecord),
    }
    Ok(())
}

fn decode_fingerprint(value: &str) -> Result<[u8; 32], RecordError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(RecordError::InvalidSchemaRecord);
    }
    let mut output = [0_u8; 32];
    for (index, target) in output.iter_mut().enumerate() {
        *target = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| RecordError::InvalidSchemaRecord)?;
    }
    Ok(output)
}

fn push_string(
    output: &mut Vec<u8>,
    value: &str,
    maximum: usize,
    allow_empty: bool,
    schema: bool,
) -> Result<(), RecordError> {
    let error = if schema {
        RecordError::InvalidSchemaRecord
    } else {
        RecordError::InvalidValueRecord
    };
    validate_text(value, maximum, allow_empty, error)?;
    let length = u32::try_from(value.len()).map_err(|_| error)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn validate_text(
    value: &str,
    maximum: usize,
    allow_empty: bool,
    error: RecordError,
) -> Result<(), RecordError> {
    if value.len() > maximum || (!allow_empty && value.is_empty()) || value.as_bytes().contains(&0)
    {
        Err(error)
    } else {
        Ok(())
    }
}

fn finish_record(output: &mut [u8], maximum: usize, error: RecordError) -> Result<(), RecordError> {
    if output.len() > maximum {
        return Err(error);
    }
    let length = u32::try_from(output.len()).map_err(|_| error)?;
    output[4..8].copy_from_slice(&length.to_be_bytes());
    Ok(())
}

struct RecordReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    error: RecordError,
}

impl<'a> RecordReader<'a> {
    fn new(bytes: &'a [u8], error: RecordError) -> Self {
        Self {
            bytes,
            offset: 0,
            error,
        }
    }

    fn header(&mut self, version: u8, kind: u8, header_bytes: usize) -> Result<(), RecordError> {
        if self.take(4)? != [version, kind, 0, 0]
            || usize::try_from(self.u32()?).ok() != Some(self.bytes.len())
        {
            return Err(self.error);
        }
        if self.offset > header_bytes {
            return Err(self.error);
        }
        Ok(())
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], RecordError> {
        let end = self.offset.checked_add(count).ok_or(self.error)?;
        let bytes = self.bytes.get(self.offset..end).ok_or(self.error)?;
        self.offset = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, RecordError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, RecordError> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().map_err(|_| self.error)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, RecordError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().map_err(|_| self.error)?,
        ))
    }
    fn i32(&mut self) -> Result<i32, RecordError> {
        Ok(i32::from_be_bytes(
            self.take(4)?.try_into().map_err(|_| self.error)?,
        ))
    }
    fn i64(&mut self) -> Result<i64, RecordError> {
        Ok(i64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| self.error)?,
        ))
    }
    fn f64(&mut self) -> Result<f64, RecordError> {
        let value = f64::from_bits(u64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| self.error)?,
        ));
        value.is_finite().then_some(value).ok_or(self.error)
    }
    fn optional_f64(&mut self, present: bool) -> Result<Option<f64>, RecordError> {
        present.then(|| self.f64()).transpose()
    }

    fn string(&mut self, maximum: usize, allow_empty: bool) -> Result<String, RecordError> {
        let length = usize::try_from(self.u32()?).map_err(|_| self.error)?;
        if length > maximum || (!allow_empty && length == 0) {
            return Err(self.error);
        }
        let bytes = self.take(length)?;
        if bytes.contains(&0) {
            return Err(self.error);
        }
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| self.error)
    }

    fn value(
        &mut self,
        kind: u8,
        maximum_string: usize,
    ) -> Result<ConfigurationValue, RecordError> {
        Ok(match kind {
            VALUE_DECIMAL => ConfigurationValue::Decimal(self.f64()?),
            VALUE_INTEGER => ConfigurationValue::Integer(self.i64()?),
            VALUE_BOOLEAN => match self.u8()? {
                0 => ConfigurationValue::Boolean(false),
                1 => ConfigurationValue::Boolean(true),
                _ => return Err(self.error),
            },
            VALUE_STRING => ConfigurationValue::String(self.string(maximum_string, true)?),
            _ => return Err(self.error),
        })
    }

    fn finish(self) -> Result<(), RecordError> {
        (self.offset == self.bytes.len())
            .then_some(())
            .ok_or(self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> ConfigurationSchema {
        let fields = vec![
            ConfigurationField {
                name: "gain".into(),
                long_name: "Gain".into(),
                field_type: FieldType::Decimal,
                wire_type: Some(WireType::Decimal16),
                default_value: ConfigurationValue::Decimal(1.5),
                minimum: Some(-2.0),
                maximum: Some(3.0),
                step: Some(0.1),
                precision: Some(2),
                scale: Some(100.0),
                suffix: "%".into(),
                choices: vec![],
                description: "A gain".into(),
            },
            ConfigurationField {
                name: "mode".into(),
                long_name: "Mode".into(),
                field_type: FieldType::Enumeration,
                wire_type: None,
                default_value: ConfigurationValue::Integer(1),
                minimum: None,
                maximum: None,
                step: None,
                precision: None,
                scale: None,
                suffix: String::new(),
                choices: vec!["A".into(), "B".into()],
                description: "Mode choice".into(),
            },
            ConfigurationField {
                name: "title".into(),
                long_name: "Title".into(),
                field_type: FieldType::String,
                wire_type: None,
                default_value: ConfigurationValue::String("board".into()),
                minimum: None,
                maximum: None,
                step: None,
                precision: None,
                scale: None,
                suffix: String::new(),
                choices: vec![],
                description: "Title text".into(),
            },
        ];
        ConfigurationSchema {
            fingerprint: "00".repeat(32),
            wire_signature: wire_signature(&fields),
            fields,
            groups: vec![ConfigurationGroup {
                name: "General".into(),
                subgroups: vec![ConfigurationSubgroup {
                    name: "Main".into(),
                    fields: vec!["gain".into(), "mode".into(), "title".into()],
                }],
            }],
        }
    }

    #[test]
    fn schema_and_value_records_round_trip_every_shape() {
        let schema = schema();
        let schema_record = encode_schema_record(&schema).unwrap();
        assert_eq!(decode_schema_record(&schema_record).unwrap(), schema);
        let values = vec![
            ("gain".into(), ConfigurationValue::Decimal(2.25)),
            ("mode".into(), ConfigurationValue::Integer(0)),
            ("title".into(), ConfigurationValue::String("float".into())),
        ];
        let value_record = encode_value_record(&values).unwrap();
        assert_eq!(decode_value_record(&value_record).unwrap(), values);
        let payload = encode_values_from_records(&schema_record, &value_record).unwrap();
        assert_eq!(
            decode_values_to_record(&schema_record, &payload).unwrap(),
            value_record
        );
    }

    #[test]
    fn record_headers_and_reserved_bytes_are_canonical() {
        let mut schema_record = encode_schema_record(&schema()).unwrap();
        assert_eq!(&schema_record[..4], &[1, 1, 0, 0]);
        schema_record[2] = 1;
        assert_eq!(
            decode_schema_record(&schema_record),
            Err(RecordError::InvalidSchemaRecord)
        );
        let mut value_record =
            encode_value_record(&vec![("mode".into(), ConfigurationValue::Integer(0))]).unwrap();
        assert_eq!(&value_record[..4], &[1, 2, 0, 0]);
        value_record.push(0);
        assert_eq!(
            decode_value_record(&value_record),
            Err(RecordError::InvalidValueRecord)
        );
    }

    #[test]
    fn malformed_and_mismatched_value_records_fail_closed() {
        let schema_record = encode_schema_record(&schema()).unwrap();
        let duplicate = vec![
            ("gain".into(), ConfigurationValue::Decimal(1.0)),
            ("gain".into(), ConfigurationValue::Decimal(1.0)),
        ];
        assert_eq!(
            encode_value_record(&duplicate),
            Err(RecordError::InvalidValueRecord)
        );
        let incomplete =
            encode_value_record(&vec![("gain".into(), ConfigurationValue::Decimal(1.0))]).unwrap();
        assert_eq!(
            encode_values_from_records(&schema_record, &incomplete),
            Err(RecordError::Codec(ConfigurationError::MissingValue))
        );
        let mut noncanonical = incomplete;
        let name_end = VALUES_HEADER_BYTES + 4 + "gain".len();
        noncanonical[name_end + 1] = 1;
        assert_eq!(
            decode_value_record(&noncanonical),
            Err(RecordError::InvalidValueRecord)
        );
    }

    #[test]
    fn nonfinite_and_bounds_fail_closed() {
        assert_eq!(
            encode_value_record(&vec![(
                "gain".into(),
                ConfigurationValue::Decimal(f64::NAN)
            )]),
            Err(RecordError::InvalidValueRecord)
        );
        let oversized = "x".repeat(MAX_STRING_VALUE_BYTES + 1);
        assert_eq!(
            encode_value_record(&vec![(
                "title".into(),
                ConfigurationValue::String(oversized)
            )]),
            Err(RecordError::InvalidValueRecord)
        );
        let mut record = encode_schema_record(&schema()).unwrap();
        let incorrect_length = u32::try_from(record.len() + 1).unwrap();
        record[4..8].copy_from_slice(&incorrect_length.to_be_bytes());
        assert_eq!(
            decode_schema_record(&record),
            Err(RecordError::InvalidSchemaRecord)
        );
    }
}
