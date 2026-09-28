//! App-private file leases; no caller-supplied path is accepted.
use crate::{Error, ProductStore, Result, identifier, limits, root};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Default)]
pub(crate) struct FileBudget {
    pub count: usize,
    pub reserved: u64,
}
pub(crate) type FileBudgets = Arc<Mutex<FileBudget>>;
#[derive(Clone, Default)]
pub struct FileCancellation(Arc<AtomicBool>);
impl FileCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub(crate) fn check(&self) -> Result<()> {
        if self.0.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    pub(crate) fn interrupted(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
/// Ownership retains the file across transport disconnection and native UI.
/// Drop only when the actual reader/presenter has released its access.
pub struct StagedFile {
    pub(crate) path: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) budget: FileBudgets,
    pub(crate) reserved: u64,
    pub(crate) owned: bool,
}
impl StagedFile {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn len(&self) -> Result<u64> {
        root::require_kind(&self.path, false)?;
        Ok(self.path.metadata()?.len())
    }
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
    pub fn read(&self, offset: u64, target: &mut [u8]) -> Result<usize> {
        if target.len() > limits::MAX_TRANSFER_BYTES {
            return Err(Error::InvalidInput);
        }
        if offset > self.len()? {
            return Err(Error::InvalidInput);
        }
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;
        Ok(file.read(target)?)
    }
}
impl Drop for StagedFile {
    fn drop(&mut self) {
        if self.owned {
            let _ = crate::sessions::remove_session_files(&self.path);
        }
        if let Ok(mut b) = self.budget.lock() {
            b.count -= 1;
            b.reserved -= self.reserved;
        }
    }
}
pub struct ImportSink {
    pub(crate) stage: StagedFile,
    file: File,
    expected: u64,
    exact: bool,
    written: u64,
}
impl ImportSink {
    pub fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        if offset != self.written
            || bytes.len() > limits::MAX_TRANSFER_BYTES
            || self
                .written
                .checked_add(bytes.len() as u64)
                .is_none_or(|n| n > self.expected)
        {
            return Err(Error::InvalidInput);
        }
        self.file.write_all(bytes)?;
        self.written += bytes.len() as u64;
        Ok(())
    }
    pub(crate) fn finish(self) -> Result<StagedFile> {
        if self.written == 0 || self.exact && self.written != self.expected {
            return Err(Error::InvalidInput);
        }
        self.file.sync_all()?;
        Ok(self.stage)
    }
}
impl ProductStore {
    pub fn begin_import(&self, length: u64) -> Result<ImportSink> {
        if length == 0 || length > limits::MAX_IMPORT_BYTES {
            return Err(Error::InvalidInput);
        }
        let stage = self.stage(false, limits::MAX_IMPORT_BYTES * 2)?;
        let file = OpenOptions::new().write(true).open(&stage.path)?;
        Ok(ImportSink {
            stage,
            file,
            expected: length,
            exact: true,
            written: 0,
        })
    }
    pub fn begin_import_stream(&self) -> Result<ImportSink> {
        let mut sink = self.begin_import(limits::MAX_IMPORT_BYTES)?;
        sink.exact = false;
        Ok(sink)
    }
    pub(crate) fn stage(&self, export: bool, reserved: u64) -> Result<StagedFile> {
        let mut b = self.file_budget.lock().map_err(|_| Error::Database)?;
        if b.count >= limits::MAX_TRANSFERS
            || b.reserved
                .checked_add(reserved)
                .is_none_or(|v| v > limits::MAX_STAGING_BYTES)
        {
            return Err(Error::Capacity);
        }
        let id = identifier()?;
        let path = if export {
            let dir = self.root.join("Exports");
            if !dir.try_exists()? {
                fs::create_dir(&dir)?;
            }
            root::require_kind(&dir, true)?;
            dir.join(format!("float-view-{id}.tmp"))
        } else {
            self.root.join(format!("import-{id}.floatview.tmp"))
        };
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        b.count += 1;
        b.reserved += reserved;
        Ok(StagedFile {
            path,
            root: self.root.clone(),
            budget: self.file_budget.clone(),
            reserved,
            owned: true,
        })
    }
    pub(crate) fn cleanup_staging(&self) -> Result<()> {
        for (i, entry) in fs::read_dir(&self.root)?.enumerate() {
            if i >= limits::MAX_DIRECTORY_ENTRIES {
                return Err(Error::Capacity);
            }
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with("import-")
                && [".floatview.tmp", ".floatview.tmp-wal", ".floatview.tmp-shm"]
                    .iter()
                    .any(|s| name.ends_with(s))
            {
                root::require_kind(&entry.path(), false)?;
                fs::remove_file(entry.path())?;
            }
        }
        let dir = self.root.join("Exports");
        if dir.try_exists()? {
            root::require_kind(&dir, true)?;
            for (i, entry) in fs::read_dir(dir)?.enumerate() {
                if i >= limits::MAX_DIRECTORY_ENTRIES {
                    return Err(Error::Capacity);
                }
                let path = entry?.path();
                root::require_kind(&path, false)?;
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
    pub(crate) fn cleanup_orphan_sessions(&self) -> Result<()> {
        for (i, entry) in fs::read_dir(&self.root)?.enumerate() {
            if i >= limits::MAX_DIRECTORY_ENTRIES {
                return Err(Error::Capacity);
            }
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let base = name
                .strip_suffix("-wal")
                .or_else(|| name.strip_suffix("-shm"))
                .or_else(|| name.strip_suffix("-journal"))
                .unwrap_or(name);
            if base.starts_with("session-") && base.ends_with(".sqlite") {
                let referenced: bool = self.catalog.query_row(
                    "SELECT EXISTS(SELECT 1 FROM session_catalog WHERE file_name=?)",
                    [base],
                    |r| r.get(0),
                )?;
                if !referenced {
                    root::require_kind(&entry.path(), false)?;
                    fs::remove_file(entry.path())?;
                }
            }
        }
        Ok(())
    }
}
