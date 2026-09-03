use std::ptr;
use std::sync::Mutex;

use float_view_application_runtime_apple_ffi::{
    ABI_VERSION, MAX_COMMAND_BYTES, Status, float_view_application_runtime_abi_version,
    float_view_application_runtime_apply, float_view_application_runtime_copy_result,
    float_view_application_runtime_create, float_view_application_runtime_destroy,
};
use serde_json::{Value, json};

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct Handle(u64);

impl Handle {
    fn create() -> Self {
        let mut handle = 0;
        assert_eq!(
            unsafe { float_view_application_runtime_create(&mut handle) },
            Status::Ok as i32
        );
        assert_ne!(handle, 0);
        Self(handle)
    }

    fn apply(&self, command: Value) -> (i32, usize) {
        let bytes = serde_json::to_vec(&command).expect("command JSON");
        let mut required = usize::MAX;
        let status = unsafe {
            float_view_application_runtime_apply(self.0, bytes.as_ptr(), bytes.len(), &mut required)
        };
        (status, required)
    }

    fn take(&self, capacity: usize) -> (i32, usize, Vec<u8>) {
        let mut output = vec![0xa5; capacity];
        let mut written = usize::MAX;
        let status = unsafe {
            float_view_application_runtime_copy_result(
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
            float_view_application_runtime_destroy(self.0),
            Status::Ok as i32
        );
    }
}

fn begin(now_ms: u64) -> Value {
    json!({
        "command": "begin",
        "now_ms": now_ms,
        "idle_interval_ms": 1_000,
        "post_ride_tail_ms": 5_000,
        "preferred_controller_can_id": null,
        "preferred_bms_can_id": null,
        "routing_hint_can_id": null,
        "routing_hint_controller_uuid": null,
        "has_active_session": false,
        "continuity_can_id": null,
        "continuity_controller_uuid": null
    })
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
    assert_eq!(value["version"], ABI_VERSION);
    let effects = value["effects"].as_array().expect("effects");
    assert!(
        effects
            .iter()
            .any(|effect| effect["effect"] == "write_bytes")
    );
    assert!(
        effects
            .iter()
            .any(|effect| effect["effect"] == "arm_deadline")
    );
    assert!(
        effects
            .iter()
            .any(|effect| effect["effect"] == "publish_state")
    );
    assert_eq!(handle.take(0).0, Status::InvalidEnvelope as i32);
}

#[test]
fn malformed_inputs_fail_without_poisoning_the_handle() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let handle = Handle::create();
    let malformed = br#"{"command":"begin","now_ms":1}"#;
    let mut required = 99;
    assert_eq!(
        unsafe {
            float_view_application_runtime_apply(
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
        unsafe { float_view_application_runtime_apply(handle.0, ptr::null(), 1, &mut required,) },
        Status::InvalidArgument as i32
    );
    assert_eq!(handle.apply(begin(10)).0, Status::Ok as i32);
}

#[test]
fn deadline_tokens_round_trip_once_and_foreign_handles_fail_closed() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let handle = Handle::create();
    let (_, required) = handle.apply(begin(10));
    let (_, _, output) = handle.take(required);
    let value: Value = serde_json::from_slice(&output).expect("result JSON");
    let effects = value["effects"].as_array().expect("effects");
    let deadline = effects
        .iter()
        .find(|effect| effect["effect"] == "arm_deadline")
        .expect("deadline")["token"]
        .as_u64()
        .expect("token");
    let generation = effects
        .iter()
        .find(|effect| effect["effect"] == "publish_state")
        .expect("state")["generation"]
        .as_u64()
        .expect("generation");
    assert_eq!(
        handle
            .apply(json!({
                "command": "deadline_fired",
                "generation": generation,
                "token": deadline,
                "now_ms": 2_100
            }))
            .0,
        Status::Ok as i32
    );
    let (_, required, _) = handle.take(0);
    let _ = handle.take(required);
    assert_eq!(
        handle
            .apply(json!({
                "command": "deadline_fired",
                "generation": generation,
                "token": deadline,
                "now_ms": 2_101
            }))
            .0,
        Status::InvalidEnvelope as i32
    );

    let command = serde_json::to_vec(&begin(10)).expect("JSON");
    let mut required = 0;
    assert_eq!(
        unsafe {
            float_view_application_runtime_apply(
                u64::MAX,
                command.as_ptr(),
                command.len(),
                &mut required,
            )
        },
        Status::InvalidHandle as i32
    );
}

#[test]
fn destroy_is_idempotent_and_invalidates_the_handle() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut handle = 0;
    assert_eq!(
        unsafe { float_view_application_runtime_create(&mut handle) },
        Status::Ok as i32
    );
    assert_eq!(
        float_view_application_runtime_destroy(handle),
        Status::Ok as i32
    );
    assert_eq!(
        float_view_application_runtime_destroy(handle),
        Status::Ok as i32
    );
    let command = serde_json::to_vec(&begin(10)).expect("JSON");
    let mut required = 0;
    assert_eq!(
        unsafe {
            float_view_application_runtime_apply(
                handle,
                command.as_ptr(),
                command.len(),
                &mut required,
            )
        },
        Status::InvalidHandle as i32
    );
}

#[test]
fn rejects_oversized_invalid_utf8_nonfinite_and_out_of_bounds_paths() {
    let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let handle = Handle::create();
    let mut required = 77;
    let one = [b'{'];
    assert_eq!(
        unsafe {
            float_view_application_runtime_apply(
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
                float_view_application_runtime_apply(
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
            Status::RuntimeFailure as i32
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
            float_view_application_runtime_apply(
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
        unsafe { float_view_application_runtime_create(handle_storage.as_mut_ptr().add(1).cast()) },
        Status::InvalidArgument as i32
    );

    let (_, required) = handle.apply(begin(11));
    let mut output = vec![0_u8; required + std::mem::size_of::<usize>()];
    let overlapping_written = output.as_mut_ptr().cast::<usize>();
    assert_eq!(
        unsafe {
            float_view_application_runtime_copy_result(
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
