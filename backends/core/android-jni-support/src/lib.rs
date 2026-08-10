//! Protocol-neutral, call-local safety support for Float View's Android JNI bridges.
//!
//! Domain crates and their JNI adapters retain their records, status mappings, and
//! exported symbols. This crate owns only panic containment and checked primitive-
//! array operations over the JVM-owned function table for the duration of one call.

#![deny(unsafe_op_in_unsafe_fn)]

pub use jni_sys::{JNIEnv, jarray, jbyte, jbyteArray, jint, jintArray, jlong, jobject, jsize};

use jni_sys::JNINativeInterface__1_2;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A protocol-neutral JNI boundary failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryError {
    /// The environment pointer or its function table was null.
    InvalidEnvironment,
    /// A required local reference was null.
    NullReference,
    /// A primitive array length was negative or did not match its required shape.
    InvalidLength,
    /// A copy would exceed its explicit caller-owned bound.
    LengthExceeded,
    /// Two references that must be independent identify the same JVM object.
    AliasedReference,
    /// A JVM exception was already pending or was raised by a JNI operation.
    VmException,
}

/// Contain a panic at an FFI boundary and return the caller's neutral fallback.
pub fn contain<T>(fallback: T, operation: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(fallback)
}

/// Borrow one JVM environment only for the dynamic extent of `operation`.
///
/// The higher-ranked closure prevents the environment wrapper from escaping the
/// current JNI call. This crate never attaches threads or retains JVM references.
///
/// # Safety
///
/// `raw` must be null or the `JNIEnv` supplied by the JVM for the current native
/// invocation. Any references passed through the resulting [`Environment`] must
/// belong to that invocation and have the primitive-array types declared by the
/// native method.
pub unsafe fn with_environment<T>(
    raw: *mut JNIEnv,
    operation: impl for<'call> FnOnce(&Environment<'call>) -> T,
) -> Result<T, BoundaryError> {
    if raw.is_null() {
        return Err(BoundaryError::InvalidEnvironment);
    }
    let functions = unsafe { *raw };
    if functions.is_null() {
        return Err(BoundaryError::InvalidEnvironment);
    }
    let environment = Environment {
        raw,
        vm: RawVm {
            functions: unsafe { &(*functions).v1_2 },
        },
    };
    Ok(operation(&environment))
}

/// Checked access to primitive JNI operations for one native invocation.
pub struct Environment<'call> {
    raw: *mut JNIEnv,
    vm: RawVm<'call>,
}

impl Environment<'_> {
    /// Fail when the JVM has a pending exception.
    pub fn ensure_no_exception(&self) -> Result<(), BoundaryError> {
        ensure_no_exception(&self.vm, self.raw)
    }

    /// Require every local reference to be non-null.
    pub fn require_references(&self, references: &[jobject]) -> Result<(), BoundaryError> {
        if references.iter().any(|reference| reference.is_null()) {
            return Err(BoundaryError::NullReference);
        }
        Ok(())
    }

    /// Read a primitive array length, checking for a newly raised exception.
    pub fn array_length(&self, array: jarray) -> Result<usize, BoundaryError> {
        array_length(&self.vm, self.raw, array)
    }

    /// Require a primitive array to have exactly `expected` elements.
    pub fn require_array_length(
        &self,
        array: jarray,
        expected: usize,
    ) -> Result<(), BoundaryError> {
        if self.array_length(array)? != expected {
            return Err(BoundaryError::InvalidLength);
        }
        Ok(())
    }

    /// Require every pair of references to identify distinct JVM objects.
    pub fn require_distinct(&self, references: &[jobject]) -> Result<(), BoundaryError> {
        require_distinct(&self.vm, self.raw, references)
    }

    /// Copy exactly `length` bytes after enforcing the explicit allocation bound.
    pub fn read_bytes(
        &self,
        array: jbyteArray,
        length: usize,
        maximum: usize,
    ) -> Result<Vec<u8>, BoundaryError> {
        read_bytes(&self.vm, self.raw, array, length, maximum)
    }

    /// Publish bytes only when they fit the caller-validated array capacity.
    pub fn write_bytes(
        &self,
        array: jbyteArray,
        capacity: usize,
        bytes: &[u8],
    ) -> Result<(), BoundaryError> {
        write_bytes(&self.vm, self.raw, array, capacity, bytes)
    }

    /// Publish integer metadata only when it exactly fills the validated array.
    pub fn write_ints_exact(
        &self,
        array: jintArray,
        validated_length: usize,
        values: &[jint],
    ) -> Result<(), BoundaryError> {
        if values.len() != validated_length {
            return Err(BoundaryError::InvalidLength);
        }
        write_ints(&self.vm, self.raw, array, values)
    }
}

trait VmCalls {
    fn exception_pending(&self, env: *mut JNIEnv) -> bool;
    fn array_length(&self, env: *mut JNIEnv, array: jarray) -> jsize;
    fn same_object(&self, env: *mut JNIEnv, left: jobject, right: jobject) -> bool;
    fn get_bytes(&self, env: *mut JNIEnv, array: jbyteArray, output: &mut [u8]);
    fn set_bytes(&self, env: *mut JNIEnv, array: jbyteArray, input: &[u8]);
    fn set_ints(&self, env: *mut JNIEnv, array: jintArray, input: &[jint]);
}

struct RawVm<'call> {
    functions: &'call JNINativeInterface__1_2,
}

impl VmCalls for RawVm<'_> {
    fn exception_pending(&self, env: *mut JNIEnv) -> bool {
        unsafe { (self.functions.ExceptionCheck)(env) }
    }

    fn array_length(&self, env: *mut JNIEnv, array: jarray) -> jsize {
        unsafe { (self.functions.GetArrayLength)(env, array) }
    }

    fn same_object(&self, env: *mut JNIEnv, left: jobject, right: jobject) -> bool {
        unsafe { (self.functions.IsSameObject)(env, left, right) }
    }

    fn get_bytes(&self, env: *mut JNIEnv, array: jbyteArray, output: &mut [u8]) {
        unsafe {
            (self.functions.GetByteArrayRegion)(
                env,
                array,
                0,
                output.len() as jsize,
                output.as_mut_ptr().cast::<jbyte>(),
            )
        }
    }

    fn set_bytes(&self, env: *mut JNIEnv, array: jbyteArray, input: &[u8]) {
        unsafe {
            (self.functions.SetByteArrayRegion)(
                env,
                array,
                0,
                input.len() as jsize,
                input.as_ptr().cast::<jbyte>(),
            )
        }
    }

    fn set_ints(&self, env: *mut JNIEnv, array: jintArray, input: &[jint]) {
        unsafe {
            (self.functions.SetIntArrayRegion)(env, array, 0, input.len() as jsize, input.as_ptr())
        }
    }
}

fn ensure_no_exception(vm: &impl VmCalls, env: *mut JNIEnv) -> Result<(), BoundaryError> {
    if vm.exception_pending(env) {
        Err(BoundaryError::VmException)
    } else {
        Ok(())
    }
}

fn array_length(
    vm: &impl VmCalls,
    env: *mut JNIEnv,
    array: jarray,
) -> Result<usize, BoundaryError> {
    let length = vm.array_length(env, array);
    ensure_no_exception(vm, env)?;
    usize::try_from(length).map_err(|_| BoundaryError::InvalidLength)
}

fn require_distinct(
    vm: &impl VmCalls,
    env: *mut JNIEnv,
    references: &[jobject],
) -> Result<(), BoundaryError> {
    for (index, left) in references.iter().enumerate() {
        for right in &references[index + 1..] {
            if vm.same_object(env, *left, *right) {
                ensure_no_exception(vm, env)?;
                return Err(BoundaryError::AliasedReference);
            }
            ensure_no_exception(vm, env)?;
        }
    }
    Ok(())
}

fn read_bytes(
    vm: &impl VmCalls,
    env: *mut JNIEnv,
    array: jbyteArray,
    length: usize,
    maximum: usize,
) -> Result<Vec<u8>, BoundaryError> {
    if length > maximum || length > jsize::MAX as usize {
        return Err(BoundaryError::LengthExceeded);
    }
    let mut output = vec![0_u8; length];
    if !output.is_empty() {
        vm.get_bytes(env, array, &mut output);
    }
    ensure_no_exception(vm, env)?;
    Ok(output)
}

fn write_bytes(
    vm: &impl VmCalls,
    env: *mut JNIEnv,
    array: jbyteArray,
    capacity: usize,
    bytes: &[u8],
) -> Result<(), BoundaryError> {
    if bytes.len() > capacity || bytes.len() > jsize::MAX as usize {
        return Err(BoundaryError::LengthExceeded);
    }
    if !bytes.is_empty() {
        vm.set_bytes(env, array, bytes);
    }
    ensure_no_exception(vm, env)
}

fn write_ints(
    vm: &impl VmCalls,
    env: *mut JNIEnv,
    array: jintArray,
    values: &[jint],
) -> Result<(), BoundaryError> {
    if values.len() > jsize::MAX as usize {
        return Err(BoundaryError::LengthExceeded);
    }
    vm.set_ints(env, array, values);
    ensure_no_exception(vm, env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;
    use std::ptr;

    const BYTES: usize = 1;
    const INTS: usize = 2;
    const OTHER: usize = 3;

    #[derive(Default)]
    struct FakeVm {
        exception: Cell<bool>,
        raise_on_next_call: Cell<bool>,
        bytes: RefCell<BTreeMap<usize, Vec<u8>>>,
        ints: RefCell<BTreeMap<usize, Vec<jint>>>,
        aliases: RefCell<Vec<(usize, usize)>>,
    }

    impl FakeVm {
        fn raise_if_requested(&self) {
            if self.raise_on_next_call.replace(false) {
                self.exception.set(true);
            }
        }

        fn key(reference: jobject) -> usize {
            reference as usize
        }
    }

    impl VmCalls for FakeVm {
        fn exception_pending(&self, _env: *mut JNIEnv) -> bool {
            self.exception.get()
        }

        fn array_length(&self, _env: *mut JNIEnv, array: jarray) -> jsize {
            self.raise_if_requested();
            let key = Self::key(array);
            self.bytes.borrow().get(&key).map_or_else(
                || {
                    self.ints
                        .borrow()
                        .get(&key)
                        .map_or(-1, |v| v.len() as jsize)
                },
                |v| v.len() as jsize,
            )
        }

        fn same_object(&self, _env: *mut JNIEnv, left: jobject, right: jobject) -> bool {
            self.raise_if_requested();
            let pair = (Self::key(left), Self::key(right));
            left == right
                || self
                    .aliases
                    .borrow()
                    .iter()
                    .any(|alias| *alias == pair || *alias == (pair.1, pair.0))
        }

        fn get_bytes(&self, _env: *mut JNIEnv, array: jbyteArray, output: &mut [u8]) {
            self.raise_if_requested();
            if let Some(input) = self.bytes.borrow().get(&Self::key(array.cast())) {
                output.copy_from_slice(&input[..output.len()]);
            }
        }

        fn set_bytes(&self, _env: *mut JNIEnv, array: jbyteArray, input: &[u8]) {
            self.raise_if_requested();
            if let Some(output) = self.bytes.borrow_mut().get_mut(&Self::key(array.cast())) {
                output[..input.len()].copy_from_slice(input);
            }
        }

        fn set_ints(&self, _env: *mut JNIEnv, array: jintArray, input: &[jint]) {
            self.raise_if_requested();
            if let Some(output) = self.ints.borrow_mut().get_mut(&Self::key(array.cast())) {
                output[..input.len()].copy_from_slice(input);
            }
        }
    }

    fn reference(id: usize) -> jobject {
        id as jobject
    }

    fn byte_array(id: usize) -> jbyteArray {
        reference(id).cast()
    }

    fn int_array(id: usize) -> jintArray {
        reference(id).cast()
    }

    fn fake() -> FakeVm {
        let vm = FakeVm::default();
        vm.bytes.borrow_mut().insert(BYTES, vec![1, 2, 3]);
        vm.bytes.borrow_mut().insert(OTHER, vec![0; 3]);
        vm.ints.borrow_mut().insert(INTS, vec![0; 2]);
        vm
    }

    #[test]
    fn rejects_null_environment_and_table_without_calling_the_vm() {
        assert_eq!(
            unsafe { with_environment(ptr::null_mut(), |_| ()) },
            Err(BoundaryError::InvalidEnvironment)
        );
        let mut table: JNIEnv = ptr::null();
        assert_eq!(
            unsafe { with_environment(&mut table, |_| ()) },
            Err(BoundaryError::InvalidEnvironment)
        );
    }

    #[test]
    fn contains_panics_with_the_typed_fallback() {
        assert_eq!(contain(17, || panic!("contained")), 17);
        assert_eq!(contain(4, || 9), 9);
    }

    #[test]
    fn checks_pending_and_new_vm_exceptions() {
        let vm = fake();
        vm.exception.set(true);
        assert_eq!(
            ensure_no_exception(&vm, ptr::null_mut()),
            Err(BoundaryError::VmException)
        );
        vm.exception.set(false);
        vm.raise_on_next_call.set(true);
        assert_eq!(
            array_length(&vm, ptr::null_mut(), reference(BYTES).cast()),
            Err(BoundaryError::VmException)
        );
    }

    #[test]
    fn rejects_invalid_lengths_and_aliases() {
        let vm = fake();
        assert_eq!(
            array_length(&vm, ptr::null_mut(), reference(99).cast()),
            Err(BoundaryError::InvalidLength)
        );
        assert_eq!(
            require_distinct(&vm, ptr::null_mut(), &[reference(BYTES), reference(BYTES)]),
            Err(BoundaryError::AliasedReference)
        );
        vm.aliases.borrow_mut().push((BYTES, OTHER));
        assert_eq!(
            require_distinct(&vm, ptr::null_mut(), &[reference(BYTES), reference(OTHER)]),
            Err(BoundaryError::AliasedReference)
        );
    }

    #[test]
    fn handles_zero_exact_and_excessive_bounded_reads() {
        let vm = fake();
        assert_eq!(
            read_bytes(&vm, ptr::null_mut(), byte_array(BYTES), 0, 3),
            Ok(Vec::new())
        );
        assert_eq!(
            read_bytes(&vm, ptr::null_mut(), byte_array(BYTES), 3, 3),
            Ok(vec![1, 2, 3])
        );
        assert_eq!(
            read_bytes(&vm, ptr::null_mut(), byte_array(BYTES), 4, 3),
            Err(BoundaryError::LengthExceeded)
        );
    }

    #[test]
    fn bounds_writes_and_publishes_exact_integer_metadata() {
        let vm = fake();
        write_bytes(&vm, ptr::null_mut(), byte_array(OTHER), 3, &[7, 8, 9]).unwrap();
        assert_eq!(vm.bytes.borrow()[&OTHER], [7, 8, 9]);
        assert_eq!(
            write_bytes(&vm, ptr::null_mut(), byte_array(OTHER), 2, &[1, 2, 3]),
            Err(BoundaryError::LengthExceeded)
        );
        write_ints(&vm, ptr::null_mut(), int_array(INTS), &[11, 12]).unwrap();
        assert_eq!(vm.ints.borrow()[&INTS], [11, 12]);
    }

    #[test]
    fn reports_null_references_and_new_copy_exceptions() {
        let vm = fake();
        let environment = |references: &[jobject]| {
            if references.iter().any(|reference| reference.is_null()) {
                Err(BoundaryError::NullReference)
            } else {
                Ok(())
            }
        };
        assert_eq!(
            environment(&[reference(BYTES), ptr::null_mut()]),
            Err(BoundaryError::NullReference)
        );
        vm.raise_on_next_call.set(true);
        assert_eq!(
            read_bytes(&vm, ptr::null_mut(), byte_array(BYTES), 3, 3),
            Err(BoundaryError::VmException)
        );
    }
}
