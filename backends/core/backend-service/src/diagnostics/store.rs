//! Private bounded files. This directory never participates in product resets.
use super::model::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
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
    "version.tmp",
    ".lock",
    ".lock-journal",
    ".lock-wal",
    ".lock-shm",
    "journal-0.jsonl",
    "journal-1.jsonl",
    "incident-0.json",
    "incident-1.json",
    "incident-2.json",
    "incident-3.json",
    "incident-4.json",
    "incident-5.json",
    "incident-6.json",
    "incident-7.json",
    "incident-8.json",
    "incident-9.json",
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
    validate_journal_version(path, 2)
}
fn validate_journal_version(path: &Path, max_version: u8) -> Io<()> {
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
                if matches!(row.v, 1 | 2)
                    && row.v <= max_version
                    && row.at >= 0
                    && row.at <= 253_402_300_799_999
                    && (row.v == 2 || row.record.content.is_none())
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
        if n >= NAMES.len() {
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
    slots: BTreeMap<usize, (u64, usize)>,
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
        let version = if fresh {
            b"2\n".to_vec()
        } else {
            read(&marker, 16)?
        };
        if version != b"1\n" && version != b"2\n" {
            return Err(bad());
        }
        if version == b"1\n"
            && (3..MAX_INCIDENTS).any(|slot| root.join(format!("incident-{slot}.json")).exists())
        {
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
            f.write_all(b"2\n")?;
            f.sync_all()?;
        }
        let mut incidents = Vec::new();
        let mut slots = BTreeMap::new();
        for slot in 0..MAX_INCIDENTS {
            let p = root.join(format!("incident-{slot}.json"));
            if p.exists() {
                let i = validate_incident(&p)?;
                if epoch.saturating_sub(i.created_ms) > RETENTION_MS {
                    fs::remove_file(p)?;
                } else {
                    slots.insert(
                        slot,
                        (i.id, serde_json::to_vec(&i).map_err(|_| bad())?.len()),
                    );
                    incidents.push(i);
                }
            }
        }
        for n in ["journal-0.jsonl", "journal-1.jsonl"] {
            let p = root.join(n);
            if p.exists() {
                validate_journal_version(&p, if version == b"1\n" { 1 } else { 2 })?;
            }
        }
        let temp = root.join("incident.tmp");
        if temp.exists() {
            let _ = read(&temp, MAX_INCIDENT_BYTES as u64)?;
        }
        let staging = root.join("version.tmp");
        if staging.exists() {
            let bytes = read(&staging, 2)?;
            if ![b"".as_slice(), b"2".as_slice(), b"2\n".as_slice()].contains(&bytes.as_slice()) {
                return Err(bad());
            }
        }
        if version == b"1\n" {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&staging)?;
            file.write_all(b"2\n")?;
            file.sync_all()?;
            fs::rename(&staging, &marker)?;
            File::open(&root)?.sync_all()?;
        } else if staging.exists() {
            fs::remove_file(&staging)?;
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
            slots,
        };
        store.prune(epoch)?;
        store.evict_for_bytes(None, 0)?;
        incidents.retain(|i| store.slots.values().any(|(id, _)| *id == i.id));
        Ok((store, incidents))
    }
    fn prune(&mut self, epoch: i64) -> Io<()> {
        for slot in 0..MAX_INCIDENTS {
            let path = self.root.join(format!("incident-{slot}.json"));
            if path.exists() {
                let incident = validate_incident(&path)?;
                if epoch.saturating_sub(incident.created_ms) > RETENTION_MS {
                    fs::remove_file(path)?;
                    self.slots.remove(&slot);
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
                    v: 2,
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
    fn evict_for_bytes(&mut self, except: Option<usize>, incoming: usize) -> Io<()> {
        while self
            .slots
            .iter()
            .filter(|(slot, _)| Some(**slot) != except)
            .map(|(_, (_, bytes))| bytes)
            .sum::<usize>()
            + incoming
            > MAX_RETAINED_INCIDENT_BYTES
        {
            let slot = self
                .slots
                .iter()
                .filter(|(slot, _)| Some(**slot) != except)
                .min_by_key(|(_, (id, _))| id)
                .map(|(slot, _)| *slot)
                .ok_or_else(bad)?;
            validate_incident(&self.root.join(format!("incident-{slot}.json")))?;
            fs::remove_file(self.root.join(format!("incident-{slot}.json")))?;
            self.slots.remove(&slot);
        }
        Ok(())
    }
    fn incident(&mut self, incident: &Incident) -> Io<()> {
        if !incident.valid() {
            return Err(bad());
        }
        let bytes = serde_json::to_vec(incident).map_err(|_| bad())?;
        let slot = self
            .slots
            .iter()
            .find(|(_, (id, _))| *id == incident.id)
            .map(|(slot, _)| *slot)
            .or_else(|| (0..MAX_INCIDENTS).find(|slot| !self.slots.contains_key(slot)))
            .or_else(|| {
                self.slots
                    .iter()
                    .min_by_key(|(_, (id, _))| id)
                    .map(|(slot, _)| *slot)
            })
            .ok_or_else(bad)?;
        let target = self.root.join(format!("incident-{slot}.json"));
        if target.exists() {
            validate_incident(&target)?;
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
        f.write_all(&bytes)?;
        f.sync_all()?;
        self.evict_for_bytes(Some(slot), bytes.len())?;
        fs::rename(p, target)?;
        self.slots.insert(slot, (incident.id, bytes.len()));
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
    fn clear(&mut self) -> Io<()> {
        check_entries(&self.root)?;
        for slot in 0..MAX_INCIDENTS {
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
        self.slots.clear();
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
            content: None,
        }
    }
    #[test]
    fn unknown_calibration_codes_still_preserve_data_and_fail_closed() {
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        drop(Store::open(&path, 1000).unwrap());
        let bytes = serde_json::to_vec(&serde_json::json!({"v":2,"at":1000,"record":{
            "sequence":1,"elapsed_ms":0,"kind":"calibration","facts":{"state":"unknownFutureCalibrationState"}
        }})).unwrap();
        let journal = path.join("journal-0.jsonl");
        fs::write(&journal, &bytes).unwrap();
        assert!(Store::open(&path, 1000).is_err());
        assert_eq!(fs::read(&journal).unwrap(), bytes);
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
            serde_json::to_vec(&serde_json::json!({"v":99,"at":1000,"record":row()})).unwrap();
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
    fn incident(id: u64) -> Incident {
        Incident {
            id,
            created_ms: 1000,
            reason: Reason::Manual,
            complete: true,
            clipped: false,
            occurrences: 1,
            events: vec![row()],
            context: vec![],
        }
    }
    #[test]
    fn ten_slots_preserve_v1_incidents_and_roll_over_after_reopen() {
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        drop(Store::open(&path, 1000).unwrap());
        fs::write(path.join("version"), b"1\n").unwrap();
        let journal =
            serde_json::to_vec(&serde_json::json!({"v":1,"at":1000,"record":row()})).unwrap();
        let mut journal = journal;
        journal.push(b'\n');
        fs::write(path.join("journal-0.jsonl"), &journal).unwrap();
        let originals = (101..104)
            .map(|id| {
                let file = path.join(format!("incident-{}.json", id % 3));
                let bytes = serde_json::to_vec(&incident(id)).unwrap();
                fs::write(&file, &bytes).unwrap();
                (file, bytes)
            })
            .collect::<Vec<_>>();
        fs::write(path.join("version.tmp"), b"2").unwrap();
        let (mut store, loaded) = Store::open(&path, 1000).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(fs::read(path.join("version")).unwrap(), b"2\n");
        assert_eq!(fs::read(path.join("journal-0.jsonl")).unwrap(), journal);
        for (file, bytes) in &originals {
            assert_eq!(&fs::read(file).unwrap(), bytes);
        }
        for id in 104..111 {
            store.incident(&incident(id)).unwrap();
        }
        assert_eq!(store.slots.len(), 10);
        drop(store);
        let (mut store, loaded) = Store::open(&path, 1000).unwrap();
        assert_eq!(
            loaded.iter().map(|i| i.id).collect::<Vec<_>>(),
            (101..111).collect::<Vec<_>>()
        );
        store.incident(&incident(111)).unwrap();
        drop(store);
        let (mut store, loaded) = Store::open(&path, 1000).unwrap();
        assert_eq!(
            loaded.iter().map(|i| i.id).collect::<Vec<_>>(),
            (102..112).collect::<Vec<_>>()
        );
        store.clear().unwrap();
        drop(store);
        assert!(Store::open(&path, 1000).unwrap().1.is_empty());
    }
    #[test]
    fn upgrade_is_restartable_and_corruption_never_advances_the_marker() {
        for partial in [b"".as_slice(), b"2".as_slice(), b"2\n".as_slice()] {
            let t = Temp::new();
            let path = t.0.join("diagnostics");
            drop(Store::open(&path, 1000).unwrap());
            fs::write(path.join("version"), b"1\n").unwrap();
            fs::write(path.join("version.tmp"), partial).unwrap();
            drop(Store::open(&path, 1000).unwrap());
            assert_eq!(fs::read(path.join("version")).unwrap(), b"2\n");
            assert!(!path.join("version.tmp").exists());
        }
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        drop(Store::open(&path, 1000).unwrap());
        fs::write(path.join("version"), b"1\n").unwrap();
        fs::write(path.join("journal-0.jsonl"), b"BROKEN\n").unwrap();
        assert!(Store::open(&path, 1000).is_err());
        assert_eq!(fs::read(path.join("version")).unwrap(), b"1\n");
        assert_eq!(fs::read(path.join("journal-0.jsonl")).unwrap(), b"BROKEN\n");
    }
    #[test]
    fn richer_incidents_obey_the_unchanged_byte_budget_and_expiry() {
        let t = Temp::new();
        let path = t.0.join("diagnostics");
        let (mut store, _) = Store::open(&path, 1000).unwrap();
        let mut large = row();
        large.facts.insert(Key::ContentId, Datum::Count(1));
        large.facts.insert(Key::Offset, Datum::Count(0));
        large
            .facts
            .insert(Key::TotalBytes, Datum::Count(CONTENT_CHUNK_BYTES as u64));
        large.facts.insert(Key::Truncated, Datum::Flag(false));
        large.content = Some(Content {
            category: ContentKind::PacketRx,
            hex: "ab".repeat(CONTENT_CHUNK_BYTES),
        });
        for id in 1..=10 {
            let mut next = incident(id);
            next.events = vec![large.clone(); 768];
            assert!(next.valid());
            store.incident(&next).unwrap();
        }
        assert!(store.slots.len() < 10);
        assert!(
            store.slots.values().map(|(_, bytes)| bytes).sum::<usize>()
                <= MAX_RETAINED_INCIDENT_BYTES
        );
        drop(store);
        let (store, loaded) = Store::open(&path, 1000).unwrap();
        assert_eq!(loaded.last().unwrap().id, 10);
        drop(store);
        let (store, loaded) = Store::open(&path, RETENTION_MS + 1001).unwrap();
        assert!(loaded.is_empty());
        assert!(store.slots.is_empty());
    }
}
