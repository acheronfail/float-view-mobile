use crate::{Error, ProductStore, Result, json, limits, root};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchemaIdentity {
    pub firmware_version: String,
    pub hardware_name: String,
    pub package_version: String,
    pub custom_index: u8,
    pub hardware_configuration_crc: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    identity: SchemaIdentity,
    digest: String,
    data: String,
}
impl SchemaIdentity {
    fn key(&self) -> Result<String> {
        for s in [
            &self.firmware_version,
            &self.hardware_name,
            &self.package_version,
        ] {
            if s.is_empty() || s.len() > 128 || s.contains('\0') {
                return Err(Error::InvalidInput);
            }
        }
        Ok(digest(
            format!(
                "{}\0{}\0{}\0{}\0{}",
                self.firmware_version,
                self.hardware_name,
                self.package_version,
                self.custom_index,
                self.hardware_configuration_crc
            )
            .as_bytes(),
        ))
    }
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
impl ProductStore {
    pub fn cached_schema(&self, identity: &SchemaIdentity) -> Result<Option<Vec<u8>>> {
        if identity.hardware_configuration_crc == 0 {
            return Ok(None);
        }
        let path = self.cache_path(identity)?;
        if !path.try_exists()? {
            return Ok(None);
        }
        root::require_kind(&path, false)?;
        let result = (|| {
            if path.metadata()?.len() > limits::MAX_SCHEMA_CACHE_BYTES as u64 {
                return Err(Error::Corrupt);
            }
            let mut bytes = Vec::new();
            fs::File::open(&path)?
                .take(limits::MAX_SCHEMA_CACHE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > limits::MAX_SCHEMA_CACHE_BYTES {
                return Err(Error::Corrupt);
            }
            let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| Error::Corrupt)?;
            if envelope.identity != *identity
                || envelope.data.len() > limits::MAX_SCHEMA_BYTES.div_ceil(3) * 4
            {
                return Err(Error::Corrupt);
            }
            let data = STANDARD.decode(envelope.data).map_err(|_| Error::Corrupt)?;
            if data.len() > limits::MAX_SCHEMA_BYTES || digest(&data) != envelope.digest {
                return Err(Error::Corrupt);
            }
            Ok(data)
        })();
        match result {
            Ok(data) => Ok(Some(data)),
            Err(Error::Corrupt) => {
                fs::remove_file(path)?;
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
    pub fn store_schema(&mut self, identity: &SchemaIdentity, data: &[u8]) -> Result<()> {
        if identity.hardware_configuration_crc == 0 || data.len() > limits::MAX_SCHEMA_BYTES {
            return Err(Error::InvalidInput);
        }
        let destination = self.cache_path(identity)?;
        let directory = destination.parent().ok_or(Error::UnsafePath)?;
        if !directory.exists() {
            fs::create_dir(directory)?;
        }
        root::require_kind(directory, true)?;
        let envelope = Envelope {
            identity: identity.clone(),
            digest: digest(data),
            data: STANDARD.encode(data),
        };
        let bytes = json(&envelope, limits::MAX_SCHEMA_CACHE_BYTES)?;
        // Keep only a bounded selection while scanning, and evict before write.
        let mut existing = Vec::new();
        for (i, entry) in fs::read_dir(directory)?.enumerate() {
            if i >= limits::MAX_DIRECTORY_ENTRIES {
                return Err(Error::Capacity);
            }
            let path = entry?.path();
            root::require_kind(&path, false)?;
            if path == destination {
                continue;
            }
            let modified = path.metadata()?.modified()?;
            existing.push((modified, path));
            existing.sort();
            if existing.len() >= limits::MAX_SCHEMA_CACHE_ENTRIES {
                fs::remove_file(existing.remove(0).1)?;
            }
        }
        if destination.symlink_metadata().is_ok() {
            root::require_kind(&destination, false)?;
        }
        let temporary = destination.with_extension("tmp");
        if temporary.symlink_metadata().is_ok() {
            root::require_kind(&temporary, false)?;
            fs::remove_file(&temporary)?;
        }
        let write = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(bytes.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, &destination)?;
            Ok(())
        })();
        if write.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        write
    }
    pub fn remove_schema(&mut self, identity: &SchemaIdentity) -> Result<()> {
        let path = self.cache_path(identity)?;
        if path.symlink_metadata().is_ok() {
            root::require_kind(&path, false)?;
            fs::remove_file(path)?;
        }
        Ok(())
    }
    fn cache_path(&self, identity: &SchemaIdentity) -> Result<PathBuf> {
        let directory = self.root.join("refloat-schema-cache");
        if directory.symlink_metadata().is_ok() {
            root::require_kind(&directory, true)?;
        }
        Ok(directory.join(format!("{}.json", identity.key()?)))
    }
}
