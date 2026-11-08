use flate2::{Compress, Compression, FlushCompress};
use float_view_refloat_configuration_codec::{
    ConfigurationError, ConfigurationField, ConfigurationGroup, ConfigurationSchema,
    ConfigurationValue, FieldType, MAX_COMPRESSED_BYTES, MAX_DESCRIPTION_BYTES, MAX_EXPANDED_BYTES,
    MAX_FIELDS, MAX_GROUPS, MAX_STRING_VALUE_BYTES, MAX_SUBGROUPS, MAX_XML_DEPTH, WireType,
    decode_qt_compressed, decode_values, encode_values, parse_schema, wire_signature,
};

fn field_xml(name: &str, body: &str) -> String {
    format!("<{name}>{body}</{name}>")
}

fn schema_xml(fields: &str, order: &str, grouping: &str) -> String {
    format!(
        "<ConfigParams><Params>{fields}</Params><SerOrder>{order}</SerOrder><Grouping>{grouping}</Grouping></ConfigParams>"
    )
}

fn qt_compress(xml: &[u8]) -> Vec<u8> {
    let mut compressor = Compress::new(Compression::default(), true);
    let mut compressed = vec![0; xml.len() + 128];
    compressor
        .compress(xml, &mut compressed, FlushCompress::Finish)
        .expect("test zlib compression must succeed");
    compressed.truncate(compressor.total_out() as usize);
    let mut result = (xml.len() as u32).to_be_bytes().to_vec();
    result.extend_from_slice(&compressed);
    result
}

fn parse_error(xml: impl AsRef<[u8]>, expected: ConfigurationError) {
    assert_eq!(parse_schema(xml.as_ref()), Err(expected));
}

#[test]
fn compression_envelopes_and_stream_completion_are_bounded() {
    assert_eq!(
        decode_qt_compressed(&[0, 0]),
        Err(ConfigurationError::TruncatedEnvelope)
    );
    let mut too_large = vec![0; MAX_COMPRESSED_BYTES + 1];
    too_large[..4].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(
        decode_qt_compressed(&too_large),
        Err(ConfigurationError::CompressedTooLarge)
    );
    let expanded_too_large = ((MAX_EXPANDED_BYTES + 1) as u32).to_be_bytes();
    assert_eq!(
        decode_qt_compressed(&expanded_too_large),
        Err(ConfigurationError::ExpandedTooLarge)
    );
    let mut invalid = 8u32.to_be_bytes().to_vec();
    invalid.extend_from_slice(b"not-zlib");
    assert_eq!(
        decode_qt_compressed(&invalid),
        Err(ConfigurationError::DecompressionFailed)
    );

    let xml = schema_xml("", "", "");
    let mut wrong_length = qt_compress(xml.as_bytes());
    wrong_length[..4].copy_from_slice(&((xml.len() + 1) as u32).to_be_bytes());
    assert!(matches!(
        decode_qt_compressed(&wrong_length),
        Err(ConfigurationError::DecompressionFailed | ConfigurationError::ExpandedLengthMismatch)
    ));
    let mut trailing = qt_compress(xml.as_bytes());
    trailing.push(0);
    assert_eq!(
        decode_qt_compressed(&trailing),
        Err(ConfigurationError::ExpandedLengthMismatch)
    );
}

#[test]
fn xml_constructs_metadata_and_collections_fail_closed() {
    parse_error(
        vec![b'x'; MAX_EXPANDED_BYTES + 1],
        ConfigurationError::ExpandedTooLarge,
    );
    parse_error(
        "<?xml version=\"1.0\"?><!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]><ConfigParams>&e;</ConfigParams>",
        ConfigurationError::ProhibitedXml,
    );
    parse_error(
        "<?xml version=\"1.0\"?><!entity e \"x\"><ConfigParams/>",
        ConfigurationError::ProhibitedXml,
    );
    parse_error([0xff], ConfigurationError::MalformedXml);
    parse_error("<ConfigParams>", ConfigurationError::MalformedXml);
    parse_error(
        format!(
            "<ConfigParams>{}{}</ConfigParams>",
            "<x>".repeat(MAX_XML_DEPTH),
            "</x>".repeat(MAX_XML_DEPTH)
        ),
        ConfigurationError::StructuralLimit,
    );
    parse_error(
        schema_xml("<x/><x/>", "", ""),
        ConfigurationError::DuplicateField,
    );
    parse_error(
        schema_xml(
            &field_xml("x", "<type>5</type>"),
            "<ser>x</ser><ser>x</ser>",
            "",
        ),
        ConfigurationError::DuplicateOrder,
    );
    parse_error(
        schema_xml("", "<ser>x</ser>", ""),
        ConfigurationError::MissingReference,
    );
    parse_error(
        schema_xml(&field_xml("x", "<type>99</type>"), "<ser>x</ser>", ""),
        ConfigurationError::UnsupportedType,
    );
    parse_error(
        schema_xml(
            &field_xml("x", "<type>1</type><vTx>99</vTx><valDouble>1</valDouble>"),
            "<ser>x</ser>",
            "",
        ),
        ConfigurationError::UnsupportedEncoding,
    );
    parse_error(
        schema_xml(
            &field_xml("x", "<type>1</type><vTx>7</vTx><valDouble>NaN</valDouble>"),
            "<ser>x</ser>",
            "",
        ),
        ConfigurationError::InvalidNumber,
    );
    parse_error(
        schema_xml(
            &field_xml(
                "x",
                &format!(
                    "<type>5</type><valInt>0</valInt><description>{}</description>",
                    "x".repeat(MAX_DESCRIPTION_BYTES + 1)
                ),
            ),
            "<ser>x</ser>",
            "",
        ),
        ConfigurationError::InvalidMetadata,
    );
    parse_error(
        schema_xml(
            &field_xml(&"x".repeat(129), "<type>5</type><valInt>0</valInt>"),
            &format!("<ser>{}</ser>", "x".repeat(129)),
            "",
        ),
        ConfigurationError::InvalidMetadata,
    );
    parse_error(
        schema_xml(
            &field_xml(
                "x",
                &format!(
                    "<type>4</type><valInt>0</valInt>{}",
                    "<enumNames>choice</enumNames>".repeat(129)
                ),
            ),
            "<ser>x</ser>",
            "",
        ),
        ConfigurationError::StructuralLimit,
    );

    let fields = (0..=MAX_FIELDS)
        .map(|index| field_xml(&format!("f{index}"), ""))
        .collect::<String>();
    parse_error(
        schema_xml(&fields, "", ""),
        ConfigurationError::StructuralLimit,
    );

    let groups = "<group><groupName>g</groupName></group>".repeat(MAX_GROUPS + 1);
    parse_error(
        schema_xml("", "", &groups),
        ConfigurationError::StructuralLimit,
    );

    let subgroups = "<subgroup><subgroupName>s</subgroupName><subgroupParams/></subgroup>"
        .repeat(MAX_SUBGROUPS + 1);
    parse_error(
        schema_xml("", "", &format!("<group>{subgroups}</group>")),
        ConfigurationError::StructuralLimit,
    );

    let elements = "<x/>".repeat(12_001);
    parse_error(
        format!("<ConfigParams>{elements}</ConfigParams>"),
        ConfigurationError::StructuralLimit,
    );
}

#[test]
fn entity_text_grouping_and_description_normalization_match_native_semantics() {
    let xml = schema_xml(
        &field_xml(
            "x",
            "<longName>Example</longName><type>3</type><valString>a&amp;b&#32;c</valString><description>&lt;p&gt; hello   world &amp;amp; friends &lt;/p&gt;</description>",
        ),
        "<ser>x</ser>",
        "<group><groupName>General</groupName><subgroup><subgroupName>Main</subgroupName><subgroupParams><param>x</param><param>::sep::Next</param></subgroupParams></subgroup></group>",
    );
    let schema = parse_schema(xml.as_bytes()).expect("valid schema");
    assert_eq!(
        schema.fields[0].default_value,
        ConfigurationValue::String("a&b c".into())
    );
    assert_eq!(schema.fields[0].description, "hello world & friends");
    assert_eq!(schema.groups[0].name, "General");
    assert_eq!(schema.groups[0].subgroups[0].name, "Main");
    assert_eq!(schema.groups[0].subgroups[0].fields, ["x", "::sep::Next"]);
}

#[test]
fn descriptions_drop_nonvisible_html_and_preserve_readable_blocks() {
    for (html, expected) in [
        (
            "<HEAD><title>Hidden title</title><style>p, li { white-space: pre-wrap; }</style></HEAD><p>Parking <b>Brake</b></p><p>Possible values:</p><ul><li>Always</li><li>Idle<br/>after stopping</li></ul>",
            "Parking Brake\n\nPossible values:\n\n• Always\n• Idle\nafter stopping",
        ),
        (
            "<style type=\"text/css\">p { color: red; }</style><script>hidden()</script><p>Visible</p>",
            "Visible",
        ),
        (
            "plain   text\nwith source whitespace",
            "plain text with source whitespace",
        ),
        ("<p>First</p><p><br /></p><p>Last</p>", "First\n\nLast"),
        ("<p>Safe</p><style>unterminated style", "Safe"),
        (
            "<p>Considered &quot;error&quot; &amp; rider&apos;s input: 2 &gt; 1 &lt; 3.</p>",
            "Considered \"error\" & rider's input: 2 > 1 < 3.",
        ),
        (
            "<p>&#34;Angle&#x20;I&#x22;&nbsp;value</p>",
            "\"Angle I\" value",
        ),
        (
            "<p>&lt;script&gt;literal&lt;/script&gt;</p>",
            "<script>literal</script>",
        ),
        (
            "<p>&amp;quot; &unknown; &#x110000; &unfinished</p>",
            "&quot; &unknown; &#x110000; &unfinished",
        ),
    ] {
        let escaped = html
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let xml = schema_xml(
            &field_xml(
                "x",
                &format!("<type>5</type><valInt>0</valInt><description>{escaped}</description>"),
            ),
            "<ser>x</ser>",
            "",
        );
        let schema = parse_schema(xml.as_bytes()).expect("valid schema");
        assert_eq!(schema.fields[0].description, expected);
    }
}

fn test_field(
    name: &str,
    field_type: FieldType,
    wire_type: Option<WireType>,
    minimum: Option<f64>,
    maximum: Option<f64>,
    scale: Option<f64>,
    choices: &[&str],
) -> ConfigurationField {
    ConfigurationField {
        name: name.into(),
        long_name: name.into(),
        field_type,
        wire_type,
        default_value: match field_type {
            FieldType::Decimal => ConfigurationValue::Decimal(0.0),
            FieldType::Integer | FieldType::Enumeration | FieldType::Bitfield => {
                ConfigurationValue::Integer(0)
            }
            FieldType::Boolean => ConfigurationValue::Boolean(false),
            FieldType::String => ConfigurationValue::String(String::new()),
        },
        minimum,
        maximum,
        step: None,
        precision: None,
        editor_scale: 1.0,
        edit_as_percentage: false,
        scale,
        suffix: String::new(),
        choices: choices.iter().map(|value| (*value).into()).collect(),
        description: "test".into(),
    }
}

fn decimal_schema(wire_type: WireType, scale: f64) -> ConfigurationSchema {
    ConfigurationSchema {
        fingerprint: String::new(),
        wire_signature: 0,
        fields: vec![test_field(
            "x",
            FieldType::Decimal,
            Some(wire_type),
            None,
            None,
            Some(scale),
            &[],
        )],
        groups: vec![],
    }
}

#[test]
fn fixed_point_decimals_round_trip_exact_wire_integers() {
    for wire_type in [WireType::Decimal16, WireType::Decimal32] {
        for scale in [0.1, 10.0, 100.0, 1000.0] {
            let schema = decimal_schema(wire_type, scale);
            // All signed 16-bit values, including negative roundoff cases.
            let mut raw_values: Vec<i32> = (i16::MIN..=i16::MAX).map(i32::from).collect();
            if wire_type == WireType::Decimal32 {
                raw_values.extend([
                    i32::MIN,
                    i32::MIN + 1,
                    -123_456_789,
                    123_456_789,
                    i32::MAX - 1,
                    i32::MAX,
                ]);
            }
            for raw in raw_values {
                let mut expected = schema.wire_signature.to_be_bytes().to_vec();
                match wire_type {
                    WireType::Decimal16 => expected.extend_from_slice(&(raw as i16).to_be_bytes()),
                    WireType::Decimal32 => expected.extend_from_slice(&raw.to_be_bytes()),
                    _ => unreachable!(),
                }
                let values = decode_values(&expected, &schema).unwrap();
                assert_eq!(
                    encode_values(&values, &schema),
                    Ok(expected),
                    "{wire_type:?} scale={scale} raw={raw}"
                );
            }
        }
    }
}

#[test]
fn decimal_rounding_matches_vesc_tool_wire_bytes() {
    for wire_type in [WireType::Decimal16, WireType::Decimal32] {
        let schema = decimal_schema(wire_type, 100.0);
        for (number, raw) in [
            (1.091, 109_i32),
            (-1.091, -109),
            (1.095, 110),
            (-1.095, -110),
            (0.005, 1),
            (-0.005, -1),
            (0.0049, 0),
            (-0.0049, 0),
            (1.09_f64.next_up(), 109),
            (1.09_f64.next_down(), 109),
            (-1.09_f64.next_up(), -109),
            (-1.09_f64.next_down(), -109),
        ] {
            let mut expected = schema.wire_signature.to_be_bytes().to_vec();
            match wire_type {
                WireType::Decimal16 => expected.extend_from_slice(&(raw as i16).to_be_bytes()),
                WireType::Decimal32 => expected.extend_from_slice(&raw.to_be_bytes()),
                _ => unreachable!(),
            }
            assert_eq!(
                encode_values(
                    &[("x".into(), ConfigurationValue::Decimal(number))],
                    &schema
                ),
                Ok(expected)
            );
        }
    }
}

#[test]
fn decimal_rounding_rejects_nonfinite_out_of_range_and_overflow_values() {
    for wire_type in [WireType::Decimal16, WireType::Decimal32] {
        let mut schema = decimal_schema(wire_type, 100.0);
        let limit = match wire_type {
            WireType::Decimal16 => i16::MAX as f64,
            WireType::Decimal32 => i32::MAX as f64,
            _ => unreachable!(),
        };
        for number in [
            (limit + 1.0) / 100.0,
            (-limit - 2.0) / 100.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            -f64::MAX,
        ] {
            assert_eq!(
                encode_values(
                    &[("x".into(), ConfigurationValue::Decimal(number))],
                    &schema
                ),
                Err(ConfigurationError::InvalidValue),
                "{wire_type:?} number={number}"
            );
        }
        schema.fields[0].minimum = Some(-1.0);
        schema.fields[0].maximum = Some(1.0);
        for number in [-1.01, 1.01] {
            assert_eq!(
                encode_values(
                    &[("x".into(), ConfigurationValue::Decimal(number))],
                    &schema
                ),
                Err(ConfigurationError::InvalidValue)
            );
        }
    }
}

fn all_types_schema() -> ConfigurationSchema {
    let fields = vec![
        test_field(
            "d16",
            FieldType::Decimal,
            Some(WireType::Decimal16),
            Some(-3276.8),
            Some(3276.7),
            Some(10.0),
            &[],
        ),
        test_field(
            "d32",
            FieldType::Decimal,
            Some(WireType::Decimal32),
            None,
            None,
            Some(100.0),
            &[],
        ),
        test_field(
            "u8",
            FieldType::Integer,
            Some(WireType::Uint8),
            None,
            None,
            None,
            &[],
        ),
        test_field(
            "i8",
            FieldType::Integer,
            Some(WireType::Int8),
            None,
            None,
            None,
            &[],
        ),
        test_field(
            "u16",
            FieldType::Integer,
            Some(WireType::Uint16),
            None,
            None,
            None,
            &[],
        ),
        test_field(
            "i16",
            FieldType::Integer,
            Some(WireType::Int16),
            None,
            None,
            None,
            &[],
        ),
        test_field(
            "u32",
            FieldType::Integer,
            Some(WireType::Uint32),
            None,
            None,
            None,
            &[],
        ),
        test_field(
            "i32",
            FieldType::Integer,
            Some(WireType::Int32),
            None,
            None,
            None,
            &[],
        ),
        test_field(
            "enum",
            FieldType::Enumeration,
            None,
            None,
            None,
            None,
            &["zero", "one"],
        ),
        test_field("bits", FieldType::Bitfield, None, None, None, None, &[]),
        test_field("bool", FieldType::Boolean, None, None, None, None, &[]),
        test_field("string", FieldType::String, None, None, None, None, &[]),
    ];
    ConfigurationSchema {
        fingerprint: "test".into(),
        wire_signature: wire_signature(&fields),
        fields,
        groups: Vec::<ConfigurationGroup>::new(),
    }
}

fn all_type_values() -> Vec<(String, ConfigurationValue)> {
    vec![
        ("d16".into(), ConfigurationValue::Decimal(-3276.8)),
        ("d32".into(), ConfigurationValue::Decimal(12345.67)),
        ("u8".into(), ConfigurationValue::Integer(255)),
        ("i8".into(), ConfigurationValue::Integer(-128)),
        ("u16".into(), ConfigurationValue::Integer(65_535)),
        ("i16".into(), ConfigurationValue::Integer(-32_768)),
        ("u32".into(), ConfigurationValue::Integer(4_294_967_295)),
        ("i32".into(), ConfigurationValue::Integer(-2_147_483_648)),
        ("enum".into(), ConfigurationValue::Integer(1)),
        ("bits".into(), ConfigurationValue::Integer(255)),
        ("bool".into(), ConfigurationValue::Boolean(true)),
        (
            "string".into(),
            ConfigurationValue::String("board ✓".into()),
        ),
    ]
}

#[test]
fn every_value_wire_type_and_signed_extreme_round_trips() {
    let schema = all_types_schema();
    let values = all_type_values();
    let bytes = encode_values(&values, &schema).expect("all types must encode");
    assert_eq!(decode_values(&bytes, &schema), Ok(values));
}

#[test]
fn hostile_value_sets_fail_before_publication() {
    let schema = all_types_schema();
    let values = all_type_values();
    let bytes = encode_values(&values, &schema).expect("valid values");

    let mut mismatch = bytes.clone();
    mismatch[0] ^= 0xff;
    assert_eq!(
        decode_values(&mismatch, &schema),
        Err(ConfigurationError::SignatureMismatch)
    );
    assert_eq!(
        decode_values(&bytes[..3], &schema),
        Err(ConfigurationError::TruncatedValues)
    );
    let mut remainder = bytes.clone();
    remainder.push(0);
    assert_eq!(
        decode_values(&remainder, &schema),
        Err(ConfigurationError::Remainder)
    );

    let mut invalid_boolean = bytes.clone();
    let boolean_offset = invalid_boolean.len() - "board ✓".len() - 2;
    invalid_boolean[boolean_offset] = 2;
    assert_eq!(
        decode_values(&invalid_boolean, &schema),
        Err(ConfigurationError::InvalidValue)
    );

    let mut invalid_utf8 = bytes.clone();
    let string_offset = boolean_offset + 1;
    invalid_utf8[string_offset] = 0xff;
    assert_eq!(
        decode_values(&invalid_utf8, &schema),
        Err(ConfigurationError::InvalidValue)
    );

    let mut missing_terminator = bytes.clone();
    missing_terminator.pop();
    assert_eq!(
        decode_values(&missing_terminator, &schema),
        Err(ConfigurationError::TruncatedValues)
    );

    let string_field = test_field("string", FieldType::String, None, None, None, None, &[]);
    let string_schema = ConfigurationSchema {
        fingerprint: "test".into(),
        wire_signature: wire_signature(std::slice::from_ref(&string_field)),
        fields: vec![string_field],
        groups: vec![],
    };
    let mut oversized_string = string_schema.wire_signature.to_be_bytes().to_vec();
    oversized_string.extend(std::iter::repeat_n(b'x', MAX_STRING_VALUE_BYTES + 1));
    oversized_string.push(0);
    assert_eq!(
        decode_values(&oversized_string, &string_schema),
        Err(ConfigurationError::InvalidValue)
    );

    let mut missing = values.clone();
    missing.pop();
    assert_eq!(
        encode_values(&missing, &schema),
        Err(ConfigurationError::MissingValue)
    );
    let mut extra = values.clone();
    extra.push(("extra".into(), ConfigurationValue::Integer(0)));
    assert_eq!(
        encode_values(&extra, &schema),
        Err(ConfigurationError::ExtraValue)
    );
    let mut duplicate = values.clone();
    duplicate.push(values[0].clone());
    assert_eq!(
        encode_values(&duplicate, &schema),
        Err(ConfigurationError::ExtraValue)
    );

    let mut wrong_type = values.clone();
    wrong_type[0].1 = ConfigurationValue::Integer(0);
    assert_eq!(
        encode_values(&wrong_type, &schema),
        Err(ConfigurationError::InvalidValue)
    );
    let mut non_finite = values.clone();
    non_finite[0].1 = ConfigurationValue::Decimal(f64::NAN);
    assert_eq!(
        encode_values(&non_finite, &schema),
        Err(ConfigurationError::InvalidValue)
    );
    let mut unscaled = values.clone();
    unscaled[0].1 = ConfigurationValue::Decimal(0.01);
    let encoded = encode_values(&unscaled, &schema).expect("VESC Tool rounds to wire precision");
    unscaled[0].1 = ConfigurationValue::Decimal(0.0);
    assert_eq!(decode_values(&encoded, &schema), Ok(unscaled));
    let mut decimal_wire_overflow = values.clone();
    decimal_wire_overflow[0].1 = ConfigurationValue::Decimal(3276.8);
    assert_eq!(
        encode_values(&decimal_wire_overflow, &schema),
        Err(ConfigurationError::InvalidValue)
    );
    let mut out_of_range = values.clone();
    out_of_range[2].1 = ConfigurationValue::Integer(256);
    assert_eq!(
        encode_values(&out_of_range, &schema),
        Err(ConfigurationError::InvalidValue)
    );
    let mut bad_enum = values.clone();
    bad_enum[8].1 = ConfigurationValue::Integer(2);
    assert_eq!(
        encode_values(&bad_enum, &schema),
        Err(ConfigurationError::InvalidValue)
    );
    let mut bad_bits = values.clone();
    bad_bits[9].1 = ConfigurationValue::Integer(256);
    assert_eq!(
        encode_values(&bad_bits, &schema),
        Err(ConfigurationError::InvalidValue)
    );
    let mut nul_string = values.clone();
    nul_string[11].1 = ConfigurationValue::String("bad\0value".into());
    assert_eq!(
        encode_values(&nul_string, &schema),
        Err(ConfigurationError::InvalidValue)
    );
    let mut long_string = values;
    long_string[11].1 = ConfigurationValue::String("x".repeat(MAX_STRING_VALUE_BYTES + 1));
    assert_eq!(
        encode_values(&long_string, &schema),
        Err(ConfigurationError::InvalidValue)
    );
}

#[test]
fn errors_are_deterministic_and_payload_free() {
    let payload_marker = "PRIVATE-CONTROLLER-PAYLOAD";
    let xml = schema_xml(
        &field_xml(payload_marker, "<type>99</type>"),
        &format!("<ser>{payload_marker}</ser>"),
        "",
    );
    let first = parse_schema(xml.as_bytes()).expect_err("unsupported type");
    let second = parse_schema(xml.as_bytes()).expect_err("unsupported type");
    assert_eq!(first, second);
    assert!(!first.to_string().contains(payload_marker));
    assert!(!format!("{first:?}").contains(payload_marker));
}

#[test]
fn editor_metadata_defaults_and_invalid_values_are_explicit() {
    let base = "<type>1</type><vTx>7</vTx><valDouble>0</valDouble><vTxDoubleScale>100</vTxDoubleScale><description>test</description>";
    let schema =
        parse_schema(schema_xml(&field_xml("x", base), "<ser>x</ser>", "").as_bytes()).unwrap();
    assert_eq!(schema.fields[0].editor_scale, 1.0);
    assert!(!schema.fields[0].edit_as_percentage);
    for extra in [
        "<editorScale>0</editorScale>",
        "<editorScale>-1</editorScale>",
        "<editorScale>NaN</editorScale>",
        "<editorScale>inf</editorScale>",
        "<editAsPercentage>2</editAsPercentage>",
    ] {
        assert!(
            parse_schema(
                schema_xml(
                    &field_xml("x", &format!("{base}{extra}")),
                    "<ser>x</ser>",
                    ""
                )
                .as_bytes()
            )
            .is_err()
        );
    }
}

#[test]
fn advertised_numeric_limits_encode_and_adjacent_wire_values_overflow() {
    for (kind, wire) in [
        (FieldType::Decimal, WireType::Decimal16),
        (FieldType::Decimal, WireType::Decimal32),
        (FieldType::Integer, WireType::Uint8),
        (FieldType::Integer, WireType::Int8),
        (FieldType::Integer, WireType::Uint16),
        (FieldType::Integer, WireType::Int16),
        (FieldType::Integer, WireType::Uint32),
        (FieldType::Integer, WireType::Int32),
    ] {
        let scale = if kind == FieldType::Decimal {
            100.0
        } else {
            1.0
        };
        let field = test_field("x", kind, Some(wire), None, None, Some(scale), &[]);
        let (minimum, maximum) = field.numeric_wire_bounds().unwrap();
        let schema = ConfigurationSchema {
            fingerprint: String::new(),
            wire_signature: 0,
            fields: vec![field],
            groups: vec![],
        };
        for number in [minimum, maximum] {
            let value = if kind == FieldType::Decimal {
                ConfigurationValue::Decimal(number)
            } else {
                ConfigurationValue::Integer(number as i64)
            };
            assert!(
                encode_values(&[("x".into(), value)], &schema).is_ok(),
                "{wire:?} {number}"
            );
        }
        for number in [minimum - 1.0 / scale, maximum + 1.0 / scale] {
            let value = if kind == FieldType::Decimal {
                ConfigurationValue::Decimal(number)
            } else {
                ConfigurationValue::Integer(number as i64)
            };
            assert_eq!(
                encode_values(&[("x".into(), value)], &schema),
                Err(ConfigurationError::InvalidValue)
            );
        }
    }
    for field in all_types_schema()
        .fields
        .iter()
        .filter(|f| !matches!(f.field_type, FieldType::Decimal | FieldType::Integer))
    {
        assert_eq!(field.numeric_wire_bounds(), None);
    }
}
