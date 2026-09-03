# Rust Android JNI Support Specification

## Purpose

Define the bounded JNI safety contract for Android's single coarse Rust
application-runtime binding. The specification identifier is retained to keep
the migration history addressable after the protocol bridge support crate is
removed.
## Requirements
### Requirement: Coarse Android runtime JNI safety

Android SHALL expose the unified application runtime through one callback-free
JNI crate that validates bounded call-local primitive arrays, contains panics,
uses nonzero opaque handles and fixed statuses, retains no JVM references, and
publishes complete caller-owned results without a protocol-specific ABI.

#### Scenario: Cross the final Android native boundary

- **WHEN** Kotlin submits a valid bounded command or copies a completed runtime result
- **THEN** the JNI boundary validates the call-local inputs and returns the exact host-runtime status and bytes without retaining JVM state or invoking a callback
