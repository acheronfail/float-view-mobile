//! Inspect a read-only BLE schema capture using the production parser and audit.
use float_view_refloat_configuration_codec::{MAX_COMPRESSED_BYTES, decode_qt_compressed};
use float_view_refloat_configuration_service::resolve_catalog;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("Expected schema capture path")?;
    let version = args.next().ok_or("Expected reported Refloat version")?;
    if args.next().is_some() || version.len() > 128 {
        return Err("Invalid arguments".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take((MAX_COMPRESSED_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    let schema = match decode_qt_compressed(&bytes) {
        Ok(schema) => schema,
        Err(error) => {
            return Err(format!("Production schema parser rejected capture: {error:?}").into());
        }
    };
    println!(
        "fields={} wire_signature={} fingerprint={}",
        schema.fields.len(),
        schema.wire_signature,
        schema.fingerprint
    );
    match resolve_catalog(&version, &schema) {
        Some(entry) => println!("audited_version={}", entry.version),
        None => return Err("Production schema audit rejected capture: UnauditedSchema".into()),
    }
    Ok(())
}
