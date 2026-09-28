use std::ptr;
use std::sync::Mutex;

use float_view_application_runtime_apple_ffi::{
    ABI_VERSION, MAX_COMMAND_BYTES, Status, float_view_application_runtime_abi_version,
    float_view_application_runtime_service_apply,
    float_view_application_runtime_service_copy_result,
    float_view_application_runtime_service_create, float_view_application_runtime_service_destroy,
};
use serde_json::{Value, json};

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct Handle(u64, std::path::PathBuf);

impl Handle {
    fn create() -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("float-view-abi-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let path = root.to_str().unwrap().as_bytes();
        let mut handle = 0;
        assert_eq!(
            unsafe {
                float_view_application_runtime_service_create(
                    path.as_ptr(),
                    path.len(),
                    &mut handle,
                )
            },
            Status::Ok as i32
        );
        assert_ne!(handle, 0);
        Self(handle, root)
    }

    fn apply(&self, command: Value) -> (i32, usize) {
        let bytes = serde_json::to_vec(&command).expect("command JSON");
        let mut required = usize::MAX;
        let status = unsafe {
            float_view_application_runtime_service_apply(
                self.0,
                bytes.as_ptr(),
                bytes.len(),
                &mut required,
            )
        };
        (status, required)
    }

    fn take(&self, capacity: usize) -> (i32, usize, Vec<u8>) {
        let mut output = vec![0xa5; capacity];
        let mut written = usize::MAX;
        let status = unsafe {
            float_view_application_runtime_service_copy_result(
                self.0,
                if capacity == 0 {
                    ptr::null_mut()
                } else {
                    output.as_mut_ptr()
                },
                capacity,
                &mut written,
            )
        };
        output.truncate(capacity.min(written));
        (status, written, output)
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        assert_eq!(
            float_view_application_runtime_service_destroy(self.0),
            Status::Ok as i32
        );
        std::fs::remove_dir_all(&self.1).unwrap();
    }
}

fn begin(_: u64) -> Value {
    json!({"command":"drain"})
}

#[test]
fn applies_once_and_copies_one_bounded_ordered_result() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    assert_eq!(float_view_application_runtime_abi_version(), ABI_VERSION);
    let handle = Handle::create();
    let (status, required) = handle.apply(begin(10));
    assert_eq!(status, Status::Ok as i32);
    assert!(required > 0);
    assert_eq!(handle.apply(begin(11)).0, Status::ResultPending as i32);

    let (status, reported, untouched) = handle.take(required - 1);
    assert_eq!(status, Status::BufferTooSmall as i32);
    assert_eq!(reported, required);
    assert!(untouched.iter().all(|byte| *byte == 0xa5));

    let (status, written, output) = handle.take(required);
    assert_eq!(status, Status::Ok as i32);
    assert_eq!(written, required);
    let value: Value = serde_json::from_slice(&output).expect("result JSON");
    assert!(value["lifetime"].as_u64().unwrap() > 0);
    assert!(
        value["effects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["effect"] == "host_state")
    );
    assert_eq!(handle.take(0).0, Status::InvalidState as i32);
}

#[test]
fn malformed_inputs_fail_without_poisoning_the_handle() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let handle = Handle::create();
    let malformed = br#"{"command":"begin","now_ms":1}"#;
    let mut required = 99;
    assert_eq!(
        unsafe {
            float_view_application_runtime_service_apply(
                handle.0,
                malformed.as_ptr(),
                malformed.len(),
                &mut required,
            )
        },
        Status::InvalidEnvelope as i32
    );
    assert_eq!(required, 0);
    assert_eq!(
        unsafe {
            float_view_application_runtime_service_apply(handle.0, ptr::null(), 1, &mut required)
        },
        Status::InvalidArgument as i32
    );
    assert_eq!(handle.apply(begin(10)).0, Status::Ok as i32);
}

#[test]
fn destroyed_and_foreign_handles_fail_closed() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let handle = Handle::create();
    let id = handle.0;
    drop(handle);
    assert_eq!(
        float_view_application_runtime_service_destroy(id),
        Status::InvalidHandle as i32
    );
    let command = serde_json::to_vec(&begin(0)).unwrap();
    for id in [id, u64::MAX] {
        let mut required = 0;
        assert_eq!(
            unsafe {
                float_view_application_runtime_service_apply(
                    id,
                    command.as_ptr(),
                    command.len(),
                    &mut required,
                )
            },
            Status::InvalidHandle as i32
        );
    }
}

#[test]
fn rejects_oversized_invalid_utf8_nonfinite_and_out_of_bounds_paths() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let handle = Handle::create();
    let mut required = 77;
    let one = [b'{'];
    assert_eq!(
        unsafe {
            float_view_application_runtime_service_apply(
                handle.0,
                one.as_ptr(),
                MAX_COMMAND_BYTES + 1,
                &mut required,
            )
        },
        Status::InvalidArgument as i32
    );
    assert_eq!(required, 77);

    for bytes in [&[0xff_u8][..], br#"{"command":"reset","now_ms":NaN}"#] {
        assert_eq!(
            unsafe {
                float_view_application_runtime_service_apply(
                    handle.0,
                    bytes.as_ptr(),
                    bytes.len(),
                    &mut required,
                )
            },
            Status::InvalidEnvelope as i32
        );
    }

    for path in [
        "relative.sqlite".to_owned(),
        format!("/{}", "x".repeat(4_097)),
    ] {
        assert_eq!(
            handle
                .apply(json!({
                    "command": "attach_session",
                    "path": path,
                    "mode": "create",
                    "expected_riding_persistence_ms": 100,
                    "expected_idle_persistence_ms": 1_000,
                    "committed_at_epoch_ms": 1_700_000_000_000_i64
                }))
                .0,
            Status::InvalidEnvelope as i32
        );
    }
}

#[test]
fn rejects_misaligned_scalars_and_overlapping_caller_memory() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let handle = Handle::create();
    let command = serde_json::to_vec(&begin(10)).expect("JSON");
    let mut storage = vec![0_u8; command.len() + std::mem::size_of::<usize>() + 8];
    storage[..command.len()].copy_from_slice(&command);
    let overlapping_required = unsafe { storage.as_mut_ptr().add(1).cast::<usize>() };
    assert_eq!(
        unsafe {
            float_view_application_runtime_service_apply(
                handle.0,
                storage.as_ptr(),
                command.len(),
                overlapping_required,
            )
        },
        Status::InvalidArgument as i32
    );

    let mut handle_storage = [0_u8; 16];
    assert_eq!(
        unsafe {
            float_view_application_runtime_service_create(
                b"/tmp".as_ptr(),
                4,
                handle_storage.as_mut_ptr().add(1).cast(),
            )
        },
        Status::InvalidArgument as i32
    );

    let (_, required) = handle.apply(begin(11));
    let mut output = vec![0_u8; required + std::mem::size_of::<usize>()];
    let overlapping_written = output.as_mut_ptr().cast::<usize>();
    assert_eq!(
        unsafe {
            float_view_application_runtime_service_copy_result(
                handle.0,
                output.as_mut_ptr(),
                required,
                overlapping_written,
            )
        },
        Status::InvalidArgument as i32
    );
    let (status, written, _) = handle.take(required);
    assert_eq!((status, written), (Status::Ok as i32, required));
}
