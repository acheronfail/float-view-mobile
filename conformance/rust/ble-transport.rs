//! Independent OS-exchange traces, including the exact VESC firmware request.
use float_view_backend_service::{
    BackendService,
    ble::{Profile, Progress, WriteMode},
    os::{Effect, Envelope, Fact, Permission, Radio},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Host {
    service: BackendService,
    root: PathBuf,
    sequence: u64,
}
impl Host {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "ble-trace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self {
            service: BackendService::open(&root).unwrap(),
            root,
            sequence: 0,
        }
    }
    fn fact(&mut self, fact: Fact) {
        self.sequence += 1;
        self.service
            .apply_os(Envelope {
                lifetime: self.service.lifetime(),
                sequence: self.sequence,
                monotonic_ms: self.sequence,
                epoch_ms: 1_700_000_000_000 + self.sequence as i64,
                fact,
            })
            .unwrap();
    }
    fn drain(&mut self) -> Vec<Effect> {
        let mut all = vec![];
        loop {
            let e = self.service.drain_os().effects;
            if e.is_empty() {
                return all;
            }
            all.extend(e);
        }
    }
    fn connect(&mut self) -> u64 {
        self.fact(Fact::Bluetooth {
            radio: Radio::PoweredOn,
            scan: Permission::Granted,
            connect: Permission::Granted,
        });
        let scan_id = self.service.start_scan(10_000).unwrap();
        self.fact(Fact::Discovered {
            scan_id,
            device_id: "trace".into(),
            name: None,
            rssi: None,
            advertises_uart: true,
        });
        self.service.connect("trace", false).unwrap();
        self.drain()
            .into_iter()
            .find_map(|e| {
                if let Effect::Connect { generation, .. } = e {
                    Some(generation)
                } else {
                    None
                }
            })
            .unwrap()
    }
    fn setup(&mut self, generation: u64, response: bool, notify: bool) -> Vec<Effect> {
        self.fact(Fact::Profile {
            generation,
            operation: generation,
            profile: Profile {
                service: true,
                notify,
                with_response: if response { 2 } else { 0 },
                without_response: 2,
            },
        });
        self.drain()
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn writes(effects: Vec<Effect>) -> Vec<(u64, WriteMode, Vec<u8>)> {
    effects
        .into_iter()
        .filter_map(|e| {
            if let Effect::Write {
                operation,
                mode,
                bytes,
                ..
            } = e
            {
                Some((operation, mode, bytes))
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn exact_segment_trace_waits_for_submission_and_the_selected_completion() {
    for response in [true, false] {
        let mut h = Host::new();
        let generation = h.connect();
        let effects = h.setup(generation, response, true);
        let operation = effects
            .into_iter()
            .find_map(|e| {
                if let Effect::EnableNotifications { operation, .. } = e {
                    Some(operation)
                } else {
                    None
                }
            })
            .unwrap();
        h.fact(Fact::Notifications {
            generation,
            operation,
            success: true,
        });
        let mode = if response {
            WriteMode::WithResponse
        } else {
            WriteMode::WithoutResponse
        };
        let completion = if response {
            Progress::Acknowledged
        } else {
            Progress::Available
        };
        let wrong = if response {
            Progress::Available
        } else {
            Progress::Acknowledged
        };
        for expected in [[2, 1], [0, 0], [0, 3]] {
            let chunk = writes(h.drain());
            assert_eq!(chunk.len(), 1);
            let (operation, actual_mode, bytes) = &chunk[0];
            assert_eq!(*actual_mode, mode);
            assert_eq!(bytes, &expected);
            for (g, op, progress) in [
                (generation + 1, *operation, completion),
                (generation, operation + 1, completion),
                (generation, *operation, wrong),
                (generation, *operation, Progress::Submitted),
                (generation, *operation, Progress::Submitted),
            ] {
                h.fact(Fact::WriteProgress {
                    generation: g,
                    operation: op,
                    progress,
                });
                assert!(writes(h.drain()).is_empty());
            }
            h.fact(Fact::WriteProgress {
                generation,
                operation: *operation,
                progress: completion,
            });
            // A duplicate result cannot free the newly dispatched chunk.
            h.fact(Fact::WriteProgress {
                generation,
                operation: *operation,
                progress: completion,
            });
        }
        assert!(writes(h.drain()).is_empty());
    }
}
#[test]
fn missing_notification_capability_never_emits_ready_traffic() {
    let mut h = Host::new();
    let generation = h.connect();
    let effects = h.setup(generation, true, false);
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::Disconnect { .. }))
    );
    assert!(writes(effects).is_empty());
    h.fact(Fact::Notifications {
        generation,
        operation: generation,
        success: true,
    });
    assert!(writes(h.drain()).is_empty());
}
