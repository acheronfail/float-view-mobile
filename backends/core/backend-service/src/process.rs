//! Process lifetime and portable maintenance executor, independent of listeners.
use crate::{BackendService, operations::Failure};
use std::{
    path::Path,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub struct Process {
    pub service: Arc<Mutex<BackendService>>,
    stop: Arc<AtomicBool>,
    finished: Mutex<mpsc::Receiver<()>>,
}
impl Process {
    pub fn open(root: &Path) -> float_view_product_store::Result<Self> {
        let service = Arc::new(Mutex::new(BackendService::open(root)?));
        let stop = Arc::new(AtomicBool::new(false));
        let weak = Arc::downgrade(&service);
        let stopped = stop.clone();
        let (done, finished) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("float-view-maintenance".into())
            .spawn(move || {
                run(weak, stopped);
                let _ = done.send(());
            })
            .map_err(|_| float_view_product_store::Error::Io)?;
        Ok(Self {
            service,
            stop,
            finished: Mutex::new(finished),
        })
    }
    /// Failure never destroys an in-flight worker's owned reader/lease.
    pub fn shutdown(&self) -> Result<(), Failure> {
        let deadline = Instant::now() + Duration::from_secs(5);
        self.stop.store(true, Ordering::Release);
        loop {
            match self.service.try_lock() {
                Ok(mut service) => {
                    service.stopping = true;
                    service.cancel_workers();
                    service.disconnect("service_shutdown")?;
                    break;
                }
                Err(std::sync::TryLockError::Poisoned(_)) => return Err(Failure::Internal),
                Err(std::sync::TryLockError::WouldBlock) => {}
            }
            if Instant::now() >= deadline {
                return Err(Failure::Unavailable);
            }
            thread::sleep(Duration::from_millis(10));
        }
        match self
            .finished
            .lock()
            .map_err(|_| Failure::Internal)?
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {}
            Err(_) => return Err(Failure::Unavailable),
        }
        loop {
            if let Ok(service) = self.service.try_lock()
                && service.active_workers() == 0
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err(Failure::Unavailable);
            }
            thread::sleep(Duration::from_millis(10));
        }
        if Instant::now() > deadline {
            return Err(Failure::Unavailable);
        }
        Ok(())
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut s) = self.service.try_lock() {
            s.cancel_workers();
        }
    }
}
fn run(weak: Weak<Mutex<BackendService>>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        let Some(owner) = weak.upgrade() else {
            break;
        };
        let next = owner.lock().ok().and_then(|mut s| {
            let epoch = s.epoch_ms();
            let now = s.monotonic_ms();
            s.expire_file_transfers(now);
            if epoch == 0 {
                return None;
            }
            s.next_maintenance_work(epoch)
                .ok()
                .flatten()
                .map(|w| (w, epoch))
        });
        drop(owner);
        if let Some((work, epoch)) = next {
            let completion = work.run(epoch);
            if let Some(owner) = weak.upgrade()
                && let Ok(mut s) = owner.lock()
            {
                let at = s.epoch_ms();
                let _ = s.complete_maintenance_work(completion, at);
            }
        } else {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_reports_contention_without_freeing_the_owner_and_can_be_retried() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("float-view-service-stop-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let process = Process::open(&root).unwrap();
        let owner = process.service.clone();
        let guard = owner.lock().unwrap();
        let started = Instant::now();
        assert!(matches!(process.shutdown(), Err(Failure::Unavailable)));
        assert!(started.elapsed() < Duration::from_secs(6));
        drop(guard);
        assert!(process.service.lock().is_ok());
        process.shutdown().unwrap();
        drop(process);
        drop(owner);
        std::fs::remove_dir_all(root).unwrap();
    }
}
