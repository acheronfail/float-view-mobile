//! Private bounded files. This directory never participates in product resets.
use super::model::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};
type Io<T> = std::io::Result<T>;
pub(super) type Lease = Arc<Mutex<rusqlite::Connection>>;
const SEGMENT: u64 = 2 * 1_048_576;
const NAMES: &[&str] = &[
    "version",
    ".lock",
    ".lock-journal",
    ".lock-wal",
    ".lock-shm",
    "journal-0.jsonl",
    "journal-1.jsonl",
    "incident-0.json",
    "incident-1.json",
    "incident-2.json",
    "incident.tmp",
    "float-view-diagnostics-0.zip",
    "float-view-diagnostics-1.zip",
];
fn bad() -> std::io::Error {
    std::io::Error::other("diagnostic storage unavailable")
}
pub(super) fn safe(path: &Path, directory: bool) -> Io<()> {
    let m = path.symlink_metadata()?;
    if m.file_type().is_symlink() || (if directory { !m.is_dir() } else { !m.is_file() }) {
        return Err(bad());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if !directory && m.nlink() != 1 {
            return Err(bad());
        }
    }
    Ok(())
}
fn read(path: &Path, limit: u64) -> Io<Vec<u8>> {
    safe(path, false)?;
    if path.metadata()?.len() > limit {
        return Err(bad());
    }
    let mut b = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut b)?;
    if b.len() as u64 > limit {
        return Err(bad());
    }
    Ok(b)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    v: u8,
    at: i64,
    record: Record,
}
fn validate_journal(path: &Path) -> Io<()> {
    let b = read(path, SEGMENT)?;
    let mut lines = b.split(|c| *c == b'\n').peekable();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if line.len() > MAX_EVENT + 96 {
            return Err(bad());
        }
        match serde_json::from_slice::<Row>(line) {
            Ok(row)
                if row.v == 1
                    && row.at >= 0
                    && row.at <= 253_402_300_799_999
                    && row.record.valid() => {}
            // A torn final append is the only recoverable malformed extent.
            Err(error) if error.is_eof() && lines.peek().is_none() && !b.ends_with(b"\n") => {}
            _ => return Err(bad()),
        }
    }
    Ok(())
}
fn validate_incident(path: &Path) -> Io<Incident> {
    let b = read(path, MAX_INCIDENT_BYTES as u64)?;
    let i: Incident = serde_json::from_slice(&b).map_err(|_| bad())?;
    if !i.valid() {
        return Err(bad());
    }
    Ok(i)
}
fn check_entries(root: &Path) -> Io<()> {
    let mut total = 0u64;
    for (n, entry) in fs::read_dir(root)?.enumerate() {
        if n >= 16 {
            return Err(bad());
        }
        let e = entry?;
        let name = e.file_name();
        if !NAMES.iter().any(|s| name == *s) {
            return Err(bad());
        }
        match safe(&e.path(), false) {
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && name
                        .to_string_lossy()
                        .starts_with("float-view-diagnostics-") =>
            {
                continue;
            }
            result => result?,
        }
        let length = match e.metadata() {
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && name
                        .to_string_lossy()
                        .starts_with("float-view-diagnostics-") =>
            {
                continue;
            }
            result => result?.len(),
        };
        if length > MAX_REPORT_BYTES {
            return Err(bad());
        }
        total = total.checked_add(length).ok_or_else(bad)?;
    }
    if total > 20 * 1_048_576 {
        return Err(bad());
    }
    Ok(())
}
pub(super) struct Store {
    root: PathBuf,
    pub lease: Lease,
    journal_bytes: u64,
    segment: usize,
}
impl Store {
    pub fn open(path: &Path, epoch: i64) -> Io<(Self, Vec<Incident>)> {
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(bad());
        }
        let root = path
            .parent()
            .ok_or_else(bad)?
            .canonicalize()?
            .join(path.file_name().ok_or_else(bad)?);
        if root.symlink_metadata().is_ok() {
            safe(&root, true)?;
        } else {
            fs::create_dir(&root)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
            }
        }
        check_entries(&root)?;
        let marker = root.join("version");
        let fresh = !marker.exists();
        if fresh && fs::read_dir(&root)?.next().is_some() {
            return Err(bad());
        }
        if !fresh && read(&marker, 16)? != b"1\n" {
            return Err(bad());
        }
        let lock = rusqlite::Connection::open(root.join(".lock")).map_err(|_| bad())?;
        lock.busy_timeout(Duration::ZERO).map_err(|_| bad())?;
        lock.execute_batch("PRAGMA trusted_schema=OFF; BEGIN EXCLUSIVE")
            .map_err(|_| bad())?;
        let lease = Arc::new(Mutex::new(lock));
        check_entries(&root)?;
        if fresh {
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)?;
            f.write_all(b"1\n")?;
            f.sync_all()?;
        }
        let mut incidents = Vec::new();
        for slot in 0..3 {
            let p = root.join(format!("incident-{slot}.json"));
            if p.exists() {
                let i = validate_incident(&p)?;
                if epoch.saturating_sub(i.created_ms) > RETENTION_MS {
                    fs::remove_file(p)?;
                } else {
                    incidents.push(i);
                }
            }
        }
        for n in ["journal-0.jsonl", "journal-1.jsonl"] {
            let p = root.join(n);
            if p.exists() {
                validate_journal(&p)?;
            }
        }
        let temp = root.join("incident.tmp");
        if temp.exists() {
            // Known unpublished staging can be interrupted before valid JSON.
            let _ = read(&temp, MAX_INCIDENT_BYTES as u64)?;
            fs::remove_file(temp)?;
        }
        // No prior process can still present a file while this exclusive root is held.
        for slot in 0..2 {
            let p = root.join(format!("float-view-diagnostics-{slot}.zip"));
            if p.exists() {
                safe(&p, false)?;
                fs::remove_file(p)?;
            }
        }
        incidents.sort_by_key(|i| i.id);
        let journal_bytes = SEGMENT; // Rotate before appending after reopen, including a torn tail.
        let mut store = Self {
            root,
            lease,
            journal_bytes,
            segment: 0,
        };
        store.prune(epoch)?;
        Ok((store, incidents))
    }
    fn prune(&mut self, epoch: i64) -> Io<()> {
        for slot in 0..3 {
            let path = self.root.join(format!("incident-{slot}.json"));
            if path.exists() {
                let incident = validate_incident(&path)?;
                if epoch.saturating_sub(incident.created_ms) > RETENTION_MS {
                    fs::remove_file(path)?;
                }
            }
        }
        for slot in 0..2 {
            let path = self.root.join(format!("journal-{slot}.jsonl"));
            if !path.exists() {
                continue;
            }
            validate_journal(&path)?;
            let bytes = read(&path, SEGMENT)?;
            let expired = bytes
                .split(|b| *b == b'\n')
                .filter_map(|line| serde_json::from_slice::<Row>(line).ok())
                .any(|r| epoch.saturating_sub(r.at) > RETENTION_MS);
            if expired {
                fs::remove_file(path)?;
                if self.segment == slot {
                    self.journal_bytes = 0;
                }
            }
        }
        Ok(())
    }
    fn append(&mut self, rows: &[Record], at: i64) -> Io<()> {
        if rows.len() > 32 {
            return Err(bad());
        }
        let mut b = Vec::new();
        for r in rows {
            if !r.valid() {
                return Err(bad());
            }
            serde_json::to_writer(
                &mut b,
                &Row {
                    v: 1,
                    at,
                    record: r.clone(),
                },
            )?;
            b.push(b'\n');
        }
        if self.journal_bytes + b.len() as u64 > SEGMENT {
            self.segment = 1 - self.segment;
            let p = self.root.join(format!("journal-{}.jsonl", self.segment));
            if p.exists() {
                validate_journal(&p)?;
                fs::remove_file(p)?;
            }
            self.journal_bytes = 0;
        }
        let p = self.root.join(format!("journal-{}.jsonl", self.segment));
        if p.exists() {
            safe(&p, false)?;
        }
        let mut f = OpenOptions::new().create(true).append(true).open(p)?;
        f.write_all(&b)?;
        f.sync_data()?;
        self.journal_bytes += b.len() as u64;
        Ok(())
    }
    fn incident(&self, incident: &Incident) -> Io<()> {
        if !incident.valid() {
            return Err(bad());
        }
        let p = self.root.join("incident.tmp");
        if p.exists() {
            safe(&p, false)?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&p)?;
        serde_json::to_writer(&mut f, incident)?;
        f.sync_all()?;
        let target = self.root.join(format!("incident-{}.json", incident.id % 3));
        if target.exists() {
            validate_incident(&target)?;
        }
        fs::rename(p, target)?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
    fn clear(&mut self) -> Io<()> {
        check_entries(&self.root)?;
        for slot in 0..3 {
            let p = self.root.join(format!("incident-{slot}.json"));
            if p.exists() {
                validate_incident(&p)?;
                fs::remove_file(p)?;
            }
        }
        for n in ["journal-0.jsonl", "journal-1.jsonl", "incident.tmp"] {
            let p = self.root.join(n);
            if p.exists() {
                if n.starts_with("journal-") {
                    validate_journal(&p)?;
                } else {
                    let _ = read(&p, MAX_INCIDENT_BYTES as u64)?;
                }
                fs::remove_file(p)?;
            }
        }
        self.journal_bytes = 0;
        Ok(())
    }
}
enum Work {
    Prune(i64),
    Rows(Vec<Record>, i64),
    Incident(Incident),
    Clear(mpsc::SyncSender<bool>),
}
pub(super) struct Sink {
    sender: Option<mpsc::SyncSender<Work>>,
    finished: mpsc::Receiver<()>,
    pub dropped: Arc<AtomicU64>,
    pub available: Arc<AtomicBool>,
    pub root: PathBuf,
    pub lease: Lease,
    queued: Arc<AtomicU64>,
}
impl Sink {
    pub fn new(store: Store) -> Io<Self> {
        let root = store.root.clone();
        let lease = store.lease.clone();
        let (sender, receiver) = mpsc::sync_channel(9);
        let queued = Arc::new(AtomicU64::new(0));
        let queue_count = queued.clone();
        let (done, finished) = mpsc::sync_channel(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let available = Arc::new(AtomicBool::new(true));
        let losses = dropped.clone();
        let status = available.clone();
        std::thread::Builder::new()
            .name("float-view-diagnostics".into())
            .spawn(move || {
                let mut store = store;
                while let Ok(work) = receiver.recv() {
                    if !matches!(&work, Work::Clear(_)) {
                        queue_count.fetch_sub(1, Ordering::Relaxed);
                    }
                    let count = match &work {
                        Work::Rows(rows, _) => rows.len() as u64,
                        _ => 1,
                    };
                    if !status.load(Ordering::Relaxed)
                        && matches!(&work, Work::Rows(..) | Work::Incident(_))
                    {
                        losses.fetch_add(count, Ordering::Relaxed);
                        continue;
                    }
                    let result = match work {
                        Work::Prune(at) => store.prune(at),
                        Work::Rows(rows, at) => store.append(&rows, at),
                        Work::Incident(i) => store.incident(&i),
                        Work::Clear(reply) => {
                            let ok = store.clear().is_ok();
                            let _ = reply.send(ok);
                            if ok { Ok(()) } else { Err(bad()) }
                        }
                    };
                    if result.is_err() {
                        losses.fetch_add(count, Ordering::Relaxed);
                        store.journal_bytes = SEGMENT;
                        status.store(false, Ordering::Relaxed);
                    } else {
                        status.store(true, Ordering::Relaxed);
                    }
                }
                drop(store);
                let _ = done.send(());
            })?;
        Ok(Self {
            sender: Some(sender),
            finished,
            dropped,
            available,
            root,
            lease,
            queued,
        })
    }
    fn send(&self, work: Work, count: u64) -> bool {
        let data = !matches!(&work, Work::Clear(_));
        if data
            && self
                .queued
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    (n < 8).then_some(n + 1)
                })
                .is_err()
        {
            self.dropped.fetch_add(count, Ordering::Relaxed);
            return false;
        }
        if self
            .sender
            .as_ref()
            .is_none_or(|s| s.try_send(work).is_err())
        {
            if data {
                self.queued.fetch_sub(1, Ordering::Relaxed);
            }
            self.dropped.fetch_add(count, Ordering::Relaxed);
            false
        } else {
            true
        }
    }
    pub fn prune(&self, at: i64) {
        self.send(Work::Prune(at), 1);
    }
    pub fn rows(&self, rows: Vec<Record>, at: i64) {
        let n = rows.len() as u64;
        self.send(Work::Rows(rows, at), n);
    }
    pub fn incident(&self, i: Incident) {
        self.send(Work::Incident(i), 1);
    }
    pub fn clear(&self) -> Option<mpsc::Receiver<bool>> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.send(Work::Clear(tx), 1).then_some(rx)
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        self.sender.take();
        let _ = self.finished.recv_timeout(Duration::from_secs(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "float-journal-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn row() -> Record {
        Record {
            run: 1,
            sequence: 1,
            elapsed_ms: 0,
            kind: Kind::Link,
            facts: Default::default(),
        }
    }
    #[test]
    fn expiry_removes_only_valid_owned_diagnostics() {
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        let (mut store, _) = Store::open(&path, 1000).unwrap();
        store.append(&[row()], 1000).unwrap();
        store
            .incident(&Incident {
                id: 1,
                created_ms: 1000,
                reason: Reason::Connection,
                complete: false,
                clipped: false,
                occurrences: 1,
                events: vec![row()],
                context: vec![],
            })
            .unwrap();
        drop(store);
        let (_, incidents) = Store::open(&path, RETENTION_MS + 1001).unwrap();
        assert!(incidents.is_empty());
        assert!(!path.join("incident-1.json").exists());
        assert!(!path.join("journal-1.jsonl").exists());
    }
    #[test]
    fn committed_corruption_is_preserved_and_torn_tail_reopens() {
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        let (mut store, _) = Store::open(&path, 1000).unwrap();
        store.append(&[row()], 1000).unwrap();
        drop(store);
        let journal = path.join("journal-1.jsonl");
        let original = fs::read(&journal).unwrap();
        OpenOptions::new()
            .append(true)
            .open(&journal)
            .unwrap()
            .write_all(b"{\"v\":")
            .unwrap();
        let (store, _) = Store::open(&path, 1000).unwrap();
        drop(store);
        assert!(fs::read(&journal).unwrap().starts_with(&original));
        fs::write(&journal, b"{invalid committed row}\n").unwrap();
        assert!(Store::open(&path, 1000).is_err());
        assert_eq!(fs::read(&journal).unwrap(), b"{invalid committed row}\n");
        let future =
            serde_json::to_vec(&serde_json::json!({"v":2,"at":1000,"record":row()})).unwrap();
        fs::write(&journal, &future).unwrap();
        assert!(Store::open(&path, RETENTION_MS + 1001).is_err());
        assert_eq!(fs::read(&journal).unwrap(), future);
    }
    #[test]
    fn exclusive_owner_and_record_bounds() {
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        let (mut store, _) = Store::open(&path, 1000).unwrap();
        assert!(Store::open(&path, 1000).is_err());
        assert!(store.append(&vec![row(); 33], 1000).is_err());
        let i = Incident {
            id: 1,
            created_ms: 1000,
            reason: Reason::Manual,
            complete: false,
            clipped: false,
            occurrences: 1,
            events: vec![row(); 769],
            context: vec![],
        };
        assert!(store.incident(&i).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_and_hardlinks_are_never_followed() {
        let t = Temp::new();
        let outside = t.0.join("outside");
        fs::write(&outside, b"PRIVATE").unwrap();
        let path = t.0.join("diagnostics");
        fs::create_dir(&path).unwrap();
        std::os::unix::fs::symlink(&outside, path.join("version")).unwrap();
        assert!(Store::open(&path, 0).is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"PRIVATE");
        fs::remove_file(path.join("version")).unwrap();
        fs::hard_link(&outside, path.join("version")).unwrap();
        assert!(Store::open(&path, 0).is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"PRIVATE");
    }
    #[test]
    fn saturated_data_queue_reserves_admission_for_clear() {
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        let (store, _) = Store::open(&path, 1000).unwrap();
        let (sender, receiver) = mpsc::sync_channel(9);
        let (_done, finished) = mpsc::sync_channel(1);
        let sink = Sink {
            sender: Some(sender),
            finished,
            dropped: Arc::new(AtomicU64::new(0)),
            available: Arc::new(AtomicBool::new(true)),
            root: path,
            lease: store.lease.clone(),
            queued: Arc::new(AtomicU64::new(0)),
        };
        for _ in 0..9 {
            sink.rows(vec![row()], 1000);
        }
        assert_eq!(sink.dropped.load(Ordering::Relaxed), 1);
        assert!(sink.clear().is_some());
        let work: Vec<_> = receiver.try_iter().collect();
        assert_eq!(work.len(), 9);
        assert!(matches!(work.last(), Some(Work::Clear(_))));
    }
}
