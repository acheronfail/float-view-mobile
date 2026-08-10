//! Protocol-agnostic safety primitives for Float View's hand-written Apple C ABIs.
//!
//! This crate exports no C symbols and retains no caller pointer. Domain bridge
//! crates remain responsible for their public statuses, limits, records, and
//! operation-specific validation.

#![deny(unsafe_op_in_unsafe_fn)]

use std::mem::{align_of, size_of};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{ptr, slice};

/// A bounded C-region validation failure with no address or payload data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryError {
    Null,
    Misaligned,
    AddressOverflow,
    Overlap,
}

/// A checked half-open address range. Empty ranges never overlap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Region {
    start: usize,
    end: usize,
}

impl Region {
    /// Construct a checked region from an integer address and byte length.
    pub fn from_address(start: usize, byte_length: usize) -> Result<Self, BoundaryError> {
        let end = start
            .checked_add(byte_length)
            .ok_or(BoundaryError::AddressOverflow)?;
        Ok(Self { start, end })
    }

    /// Construct an optional byte region. Null is accepted only for zero bytes.
    pub fn optional<T>(pointer: *const T, byte_length: usize) -> Result<Self, BoundaryError> {
        if pointer.is_null() && byte_length != 0 {
            return Err(BoundaryError::Null);
        }
        Self::from_address(pointer as usize, byte_length)
    }

    /// Construct one required, naturally aligned scalar region.
    pub fn required<T>(pointer: *const T) -> Result<Self, BoundaryError> {
        if pointer.is_null() {
            return Err(BoundaryError::Null);
        }
        if !(pointer as usize).is_multiple_of(align_of::<T>()) {
            return Err(BoundaryError::Misaligned);
        }
        Self::from_address(pointer as usize, size_of::<T>())
    }

    fn overlaps(self, other: Self) -> bool {
        self.start != self.end
            && other.start != other.end
            && self.start < other.end
            && other.start < self.end
    }
}

/// Require every declared C memory region to be pairwise disjoint.
pub fn validate_disjoint(regions: &[Region]) -> Result<(), BoundaryError> {
    for left in 0..regions.len() {
        for right in left + 1..regions.len() {
            if regions[left].overlaps(regions[right]) {
                return Err(BoundaryError::Overlap);
            }
        }
    }
    Ok(())
}

/// Contain a panic and preserve caller-owned success and failure status values.
pub fn contain_status<Status: Copy>(
    success: Status,
    internal_failure: Status,
    operation: impl FnOnce() -> Result<(), Status>,
) -> Status {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => success,
        Ok(Err(status)) => status,
        Err(_) => internal_failure,
    }
}

/// Borrow a previously validated input region for only the caller-chosen lifetime.
///
/// # Safety
///
/// For nonzero `byte_length`, `pointer` must be valid and initialized for reads
/// of exactly that many bytes. The region must remain live and unmodified for
/// the returned borrow, and the caller must have established all required
/// aliasing constraints. For zero length, null is permitted.
pub unsafe fn input_bytes<'a>(pointer: *const u8, byte_length: usize) -> &'a [u8] {
    if byte_length == 0 {
        &[]
    } else {
        // SAFETY: required by this function's caller contract.
        unsafe { slice::from_raw_parts(pointer, byte_length) }
    }
}

/// Publish the required byte length and then copy a complete result if it fits.
///
/// Returns `Err(required_length)` when capacity is insufficient. In that case
/// only the required-length scalar is written and the byte output is untouched.
///
/// # Safety
///
/// `output_length` must be a valid aligned scalar output. `output` must be valid
/// for `output_capacity` writable bytes when nonzero. Those regions and `bytes`
/// must be disjoint for the duration of the call.
pub unsafe fn publish_bytes(
    bytes: &[u8],
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> Result<(), usize> {
    // SAFETY: required by this function's caller contract.
    unsafe { output_length.write(bytes.len()) };
    if output_capacity < bytes.len() {
        return Err(bytes.len());
    }
    if !bytes.is_empty() {
        // SAFETY: required by the caller contract and capacity check above.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) };
    }
    Ok(())
}

/// Write one scalar to a previously validated required output region.
///
/// # Safety
///
/// `output` must be non-null, aligned, valid for one `T`, disjoint from every
/// live input, and live for the write.
pub unsafe fn write_scalar<T>(output: *mut T, value: T) {
    // SAFETY: required by this function's caller contract.
    unsafe { output.write(value) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_region_accepts_only_zero_length_null() {
        assert_eq!(
            Region::optional(std::ptr::null::<u8>(), 0),
            Ok(Region { start: 0, end: 0 })
        );
        assert_eq!(
            Region::optional(std::ptr::null::<u8>(), 1),
            Err(BoundaryError::Null)
        );
    }

    #[test]
    fn required_region_rejects_null_and_misalignment() {
        assert_eq!(
            Region::required(std::ptr::null::<usize>()),
            Err(BoundaryError::Null)
        );
        assert_eq!(
            Region::required(
                std::ptr::dangling::<usize>()
                    .cast::<u8>()
                    .wrapping_add(1)
                    .cast::<usize>(),
            ),
            Err(BoundaryError::Misaligned)
        );
    }

    #[test]
    fn region_rejects_address_wraparound() {
        assert_eq!(
            Region::from_address(usize::MAX, 1),
            Err(BoundaryError::AddressOverflow)
        );
    }

    #[test]
    fn disjointness_rejects_overlap_but_ignores_empty_regions() {
        let left = Region::from_address(10, 4).unwrap();
        let overlapping = Region::from_address(12, 4).unwrap();
        let adjacent = Region::from_address(14, 3).unwrap();
        let empty = Region::from_address(11, 0).unwrap();
        assert_eq!(
            validate_disjoint(&[left, overlapping]),
            Err(BoundaryError::Overlap)
        );
        assert_eq!(validate_disjoint(&[left, adjacent, empty]), Ok(()));
    }

    #[test]
    fn containment_preserves_status_and_contains_panics() {
        assert_eq!(contain_status(0, 5, || Ok(())), 0);
        assert_eq!(contain_status(0, 5, || Err(3)), 3);
        assert_eq!(
            contain_status(0, 5, || -> Result<(), i32> { panic!("contained") }),
            5
        );
    }

    #[test]
    fn validated_input_borrow_preserves_bytes_and_accepts_empty_null() {
        let input = [1_u8, 2, 3];
        // SAFETY: the array is initialized and live for the returned borrow.
        assert_eq!(unsafe { input_bytes(input.as_ptr(), input.len()) }, input);
        // SAFETY: zero-length input explicitly permits null.
        assert!(unsafe { input_bytes(std::ptr::null(), 0) }.is_empty());
    }

    #[test]
    fn byte_publication_is_complete_or_length_only() {
        let bytes = [4_u8, 5, 6];
        let mut exact = [0_u8; 3];
        let mut exact_length = 0;
        // SAFETY: outputs are valid, aligned, disjoint, and exactly sized.
        assert_eq!(
            unsafe { publish_bytes(&bytes, exact.as_mut_ptr(), exact.len(), &mut exact_length) },
            Ok(())
        );
        assert_eq!(exact, bytes);
        assert_eq!(exact_length, bytes.len());

        let mut short = [9_u8; 2];
        let mut required = 0;
        // SAFETY: outputs are valid, aligned, disjoint, and capacity is declared exactly.
        assert_eq!(
            unsafe { publish_bytes(&bytes, short.as_mut_ptr(), short.len(), &mut required) },
            Err(bytes.len())
        );
        assert_eq!(short, [9, 9]);
        assert_eq!(required, bytes.len());
    }

    #[test]
    fn zero_byte_publication_accepts_null_output() {
        let mut length = usize::MAX;
        // SAFETY: null is permitted for zero output capacity and length is valid.
        assert_eq!(
            unsafe { publish_bytes(&[], std::ptr::null_mut(), 0, &mut length) },
            Ok(())
        );
        assert_eq!(length, 0);
    }

    #[test]
    fn scalar_publication_writes_exact_value() {
        let mut value = 0_u64;
        // SAFETY: `value` is valid, aligned, and exclusively borrowed.
        unsafe { write_scalar(&mut value, 0x0102_0304_0506_0708) };
        assert_eq!(value, 0x0102_0304_0506_0708);
    }
}
