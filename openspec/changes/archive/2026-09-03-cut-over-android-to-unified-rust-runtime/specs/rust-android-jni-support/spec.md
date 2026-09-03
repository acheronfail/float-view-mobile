## ADDED Requirements

### Requirement: Coarse Android runtime JNI safety

Android SHALL expose the unified application runtime through one callback-free
JNI crate that validates bounded call-local primitive arrays, contains panics,
uses nonzero opaque handles and fixed statuses, retains no JVM references, and
publishes complete caller-owned results without a protocol-specific ABI.

#### Scenario: Cross the final Android native boundary

- **WHEN** Kotlin submits a valid bounded command or copies a completed runtime result
- **THEN** the JNI boundary validates the call-local inputs and returns the exact host-runtime status and bytes without retaining JVM state or invoking a callback

## REMOVED Requirements

### Requirement: Shared protocol-neutral Android JNI safety support

**Reason:** The support crate exists only for the ten temporary protocol JNI consumers, all of which are deleted by the coarse Android runtime cutover.

**Migration:** The thin runtime JNI crate performs its bounded call-local array validation against the shared safe host-envelope API and retains no protocol-specific consumer surface.

### Requirement: Independent Android JNI bridge ABI preservation

**Reason:** Separate protocol ABIs and artifacts are migration scaffolding superseded by the unified runtime.

**Migration:** Android calls the single versioned application-runtime JNI ABI and all old Kotlin bridge declarations are deleted.

### Requirement: Android JNI support regression evidence

**Reason:** Final convergence replaces temporary-bridge preservation gates with one-runtime packaging and conformance gates.

**Migration:** Focused shared-boundary/JNI tests, clean four-ABI packaging, `just verify`, and the recorded requested physical regression become authoritative.
