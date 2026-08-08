//! Portable third-party dependency proofs.

use std::io::{Cursor, Read, Write};

use flate2::{Compression, read::ZlibDecoder, write::GzEncoder};
use sha2::{Digest, Sha256};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

/// Runs deterministic crypto, random, compression, `XML`, `CRC32C`, and archive checks.
///
/// # Errors
///
/// Returns a stable category when a selected dependency violates its expected contract.
pub fn prove_dependencies() -> Result<(), &'static str> {
    let digest = Sha256::digest(b"abc");
    if format!("{digest:x}") != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" {
        return Err("sha256");
    }

    let mut random = [0_u8; 32];
    getrandom::fill(&mut random).map_err(|_| "secure-random")?;
    if random == [0_u8; 32] {
        return Err("secure-random-zero");
    }

    let payload = b"portable dependency smoke";
    let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
    gzip.write_all(payload).map_err(|_| "gzip-write")?;
    let gzip = gzip.finish().map_err(|_| "gzip-finish")?;
    if &gzip[..2] != b"\x1f\x8b" {
        return Err("gzip-header");
    }

    let zlib = [
        0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0x00, 0x06, 0x2c, 0x02, 0x15,
    ];
    let mut decoded = Vec::new();
    ZlibDecoder::new(zlib.as_slice())
        .read_to_end(&mut decoded)
        .map_err(|_| "zlib")?;
    if decoded != b"hello" {
        return Err("zlib-payload");
    }

    let document = roxmltree::Document::parse_with_options(
        "<config><value name=\"gain\">1.25</value></config>",
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 16,
            entity_resolver: None,
        },
    )
    .map_err(|_| "xml")?;
    if document
        .descendants()
        .find(|node| node.has_tag_name("value"))
        .and_then(|node| node.text())
        != Some("1.25")
    {
        return Err("xml-value");
    }

    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record(["name", "value"]).map_err(|_| "csv")?;
    csv.write_record(["gain", "1,25"]).map_err(|_| "csv")?;
    let csv = csv.into_inner().map_err(|_| "csv")?;
    if csv != b"name,value\ngain,\"1,25\"\n" {
        return Err("csv-quoting");
    }

    if crc32c::crc32c(b"123456789") != 0xe306_9283 {
        return Err("crc32c");
    }

    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    writer
        .start_file("manifest.json", SimpleFileOptions::default())
        .map_err(|_| "archive-start")?;
    writer
        .write_all(br#"{"format":1}"#)
        .map_err(|_| "archive-write")?;
    let bytes = writer.finish().map_err(|_| "archive-finish")?.into_inner();
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| "archive-open")?;
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .map_err(|_| "archive-entry")?
        .take(1024)
        .read_to_string(&mut manifest)
        .map_err(|_| "archive-read")?;
    if manifest != r#"{"format":1}"# {
        return Err("archive-payload");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn selected_dependencies_satisfy_portable_contracts() {
        super::prove_dependencies().unwrap();
    }
}
