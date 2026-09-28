use crate::{CATALOG_SCHEMA, Error, Result, STORAGE_VERSION, limits};
use rusqlite::Connection;
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

pub(crate) fn prepare(root: &Path) -> Result<(PathBuf, std::sync::Arc<fs::File>)> {
    if !root.is_absolute() || root.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Error::UnsafePath);
    }
    let name = root.file_name().ok_or(Error::UnsafePath)?;
    // Parent is supplied by trusted native startup; permit system aliases such
    // as macOS /var, but never follow symlinks within the owned root.
    let parent = root.parent().ok_or(Error::UnsafePath)?.canonicalize()?;
    let root = parent.join(name);
    if root.exists() || root.symlink_metadata().is_ok() {
        require_kind(&root, true)?;
    } else {
        fs::create_dir(&root)?;
    }
    let lock_path = root.join(".service.lock");
    if lock_path.symlink_metadata().is_ok() {
        require_kind(&lock_path, false)?;
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    require_kind(&lock_path, false)?;
    lock.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => Error::Busy,
        std::fs::TryLockError::Error(_) => Error::Io,
    })?;
    for name in [
        "catalog.sqlite",
        "catalog.sqlite-wal",
        "catalog.sqlite-shm",
        ".schema-version",
    ] {
        let p = root.join(name);
        if p.symlink_metadata().is_ok() {
            require_kind(&p, false)?;
        }
    }
    for name in ["Exports", "refloat-schema-cache"] {
        let p = root.join(name);
        if p.symlink_metadata().is_ok() {
            require_kind(&p, true)?;
        }
    }
    let marker = root.join(".schema-version");
    let version = match fs::File::open(&marker) {
        Ok(file) => {
            let mut text = String::new();
            file.take(65).read_to_string(&mut text)?;
            if text.len() > 64 {
                return Err(Error::Corrupt);
            }
            Some(text)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    if version.as_deref().map(str::trim) != Some(STORAGE_VERSION) {
        // Enumerate twice: fail on hostile owned entries before any deletion.
        visit_owned(&root, false)?;
        visit_owned(&root, true)?;
        let temporary = root.join(".schema-version.tmp");
        if temporary.symlink_metadata().is_ok() {
            require_kind(&temporary, false)?;
            fs::remove_file(&temporary)?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        std::io::Write::write_all(&mut file, STORAGE_VERSION.as_bytes())?;
        file.sync_all()?;
        fs::rename(temporary, marker)?;
        fs::File::open(&root)?.sync_all()?;
    }
    Ok((root, std::sync::Arc::new(lock)))
}
pub(crate) fn require_kind(path: &Path, directory: bool) -> Result<()> {
    let meta = path.symlink_metadata()?;
    if meta.file_type().is_symlink()
        || (directory && !meta.is_dir())
        || (!directory && !meta.is_file())
    {
        return Err(Error::UnsafePath);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if !directory && meta.nlink() != 1 {
            return Err(Error::UnsafePath);
        }
    }
    Ok(())
}
fn visit_owned(root: &Path, delete: bool) -> Result<()> {
    for (i, entry) in fs::read_dir(root)?.enumerate() {
        if i >= limits::MAX_DIRECTORY_ENTRIES {
            return Err(Error::Capacity);
        }
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let path = entry.path();
        if matches!(name, "Exports" | "refloat-schema-cache") {
            require_kind(&path, true)?;
            for (j, child) in fs::read_dir(&path)?.enumerate() {
                if j >= limits::MAX_DIRECTORY_ENTRIES {
                    return Err(Error::Capacity);
                }
                let child = child?.path();
                require_kind(&child, false)?;
                if delete {
                    fs::remove_file(child)?;
                }
            }
            if delete {
                fs::remove_dir(path)?;
            }
        } else if matches!(
            name,
            "catalog.sqlite" | "catalog.sqlite-wal" | "catalog.sqlite-shm" | ".schema-version.tmp"
        ) || (name.starts_with("session-")
            && [".sqlite", ".sqlite-wal", ".sqlite-shm"]
                .iter()
                .any(|s| name.ends_with(s)))
            || (name.starts_with("import-")
                && [".floatview.tmp", ".floatview.tmp-wal", ".floatview.tmp-shm"]
                    .iter()
                    .any(|s| name.ends_with(s)))
        {
            require_kind(&path, false)?;
            if delete {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_catalog(catalog: &Connection, fresh: bool) -> Result<()> {
    if catalog.query_row("PRAGMA quick_check(1)", [], |r| r.get::<_, String>(0))? != "ok" {
        return Err(Error::Corrupt);
    }
    let expected = Connection::open_in_memory()?;
    expected.execute_batch(CATALOG_SCHEMA)?;
    fn schema(db: &Connection) -> Result<Vec<(String, String, String)>> {
        let mut statement = db.prepare("SELECT type,name,COALESCE(sql,'') FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name LIMIT 65")?;
        let rows = statement
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if rows.len() > 64 {
            return Err(Error::Corrupt);
        }
        Ok(rows)
    }
    let actual = schema(catalog)?;
    if !(fresh && actual.is_empty()) && actual != schema(&expected)? {
        return Err(Error::Corrupt);
    }
    Ok(())
}
