use std::path::{Path, PathBuf};

use float_view_refloat_configuration_codec::{
    ConfigurationValue, decode_qt_compressed, decode_qt_compressed_to_record, decode_schema_record,
    decode_value_record, decode_values, decode_values_to_record, encode_values,
    encode_values_from_records,
};

struct Release {
    version: &'static str,
    fingerprint: &'static str,
    signature: u32,
    fields: usize,
    groups: usize,
}

const RELEASES: &[Release] = &[
    Release {
        version: "1.0.0",
        fingerprint: "324e7f5ef1ea1b2038478af7e6f43c91d63ec06a6caf2907c5b7e6c812382ad9",
        signature: 0xf3d4_9d76,
        fields: 144,
        groups: 1,
    },
    Release {
        version: "1.0.1",
        fingerprint: "887e7b03c03304a2f8455e71b369b89830e8d6446a73aa598ab067ac81944526",
        signature: 0xf3d4_9d76,
        fields: 144,
        groups: 1,
    },
    Release {
        version: "1.1.0",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        signature: 0x00f1_c21b,
        fields: 156,
        groups: 2,
    },
    Release {
        version: "1.1.1",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        signature: 0x00f1_c21b,
        fields: 156,
        groups: 2,
    },
    Release {
        version: "1.1.2",
        fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
        signature: 0x00f1_c21b,
        fields: 156,
        groups: 2,
    },
    Release {
        version: "1.2.0",
        fingerprint: "2561e00a5b6694de0ac272bae077e06050cfc00cf4ba371bb1d5d9616e137f7e",
        signature: 0x90b7_a9ba,
        fields: 167,
        groups: 2,
    },
    Release {
        version: "1.2.1",
        fingerprint: "9b8979a36759810a7fa9c19d01534ed52ec9ed8ac4a2d3ab3aefc594ba3b93cc",
        signature: 0x90b7_a9ba,
        fields: 167,
        groups: 2,
    },
    Release {
        version: "1.2.2",
        fingerprint: "d7ca6a2c06ce5701858eebfaaf224e97e08a212d1dab60b664d268a90763eb39",
        signature: 0x90b7_a9ba,
        fields: 167,
        groups: 2,
    },
    Release {
        version: "1.2.3",
        fingerprint: "fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c",
        signature: 0x90b7_a9ba,
        fields: 167,
        groups: 2,
    },
    Release {
        version: "1.3.0",
        fingerprint: "62681208dc609eb19420c89df65105f018499aa9f5741f295f00d62ea5a1bceb",
        signature: 0x191a_6c1b,
        fields: 172,
        groups: 2,
    },
];

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/refloat-configuration")
}

fn read(path: impl AsRef<Path>) -> Vec<u8> {
    std::fs::read(path).expect("committed fixture must be readable")
}

#[test]
fn every_audited_schema_matches_exact_source_evidence() {
    let root = fixture_root();
    for release in RELEASES {
        let compressed = read(root.join(format!("schemas/{}.qcompress", release.version)));
        let schema = decode_qt_compressed(&compressed).expect("audited schema must decode");
        assert_eq!(
            schema.fingerprint, release.fingerprint,
            "{}",
            release.version
        );
        assert_eq!(
            schema.wire_signature, release.signature,
            "{}",
            release.version
        );
        assert_eq!(schema.fields.len(), release.fields, "{}", release.version);
        assert_eq!(schema.groups.len(), release.groups, "{}", release.version);
        let schema_record = decode_qt_compressed_to_record(&compressed).unwrap();
        assert_eq!(decode_schema_record(&schema_record), Ok(schema.clone()));
        assert_eq!(
            decode_qt_compressed(&compressed),
            Ok(schema.clone()),
            "{} is deterministic",
            release.version
        );

        let kp = schema
            .fields
            .iter()
            .find(|field| field.name == "kp")
            .expect("every release carries representative kp");
        assert_eq!(kp.long_name, "Angle P");
        assert_eq!(kp.default_value, ConfigurationValue::Decimal(20.0));
        assert_eq!(kp.minimum, Some(0.0));
        assert_eq!(kp.maximum, Some(40.0));
        assert_eq!(kp.scale, Some(10.0));
        assert!(!kp.description.contains('<'));
        assert!(!kp.description.contains('>'));
        assert!(!kp.description.is_empty());
        assert_eq!(schema.groups[0].name, "General");
        assert!(!schema.groups[0].subgroups.is_empty());
    }

    assert_eq!(RELEASES[0].signature, RELEASES[1].signature);
    assert_ne!(RELEASES[0].fingerprint, RELEASES[1].fingerprint);
}

#[test]
fn every_audited_value_family_round_trips_exact_bytes() {
    let root = fixture_root();
    for release in RELEASES {
        let schema = decode_qt_compressed(&read(
            root.join(format!("schemas/{}.qcompress", release.version)),
        ))
        .expect("audited schema must decode");
        for kind in ["default", "current", "write", "readback"] {
            let bytes = read(root.join(format!("values/{}-{kind}.bin", release.version)));
            let values = decode_values(&bytes, &schema).unwrap_or_else(|error| {
                panic!("{} {kind} failed to decode: {error}", release.version)
            });
            assert_eq!(values.len(), release.fields, "{} {kind}", release.version);
            assert_eq!(
                encode_values(&values, &schema),
                Ok(bytes.clone()),
                "{} {kind}",
                release.version
            );
            let schema_record =
                float_view_refloat_configuration_codec::encode_schema_record(&schema)
                    .expect("audited schema record must encode");
            let value_record = decode_values_to_record(&schema_record, &bytes)
                .expect("audited values record must decode");
            assert_eq!(decode_value_record(&value_record), Ok(values));
            assert_eq!(
                encode_values_from_records(&schema_record, &value_record),
                Ok(bytes),
                "{} {kind} canonical record",
                release.version
            );
        }
    }
}
