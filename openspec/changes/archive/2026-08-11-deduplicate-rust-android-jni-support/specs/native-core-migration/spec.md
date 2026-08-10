## ADDED Requirements

### Requirement: Narrow Android JNI support ownership

Shared Rust Android JNI support SHALL own only protocol-neutral, primitive-array VM-boundary mechanics common to every selected Android bridge. VESC and Refloat framing, commands, decoding, encoding, record schemas, status mappings, compatibility, routing, scheduling, application behavior, persistence, transport, Kotlin host policy, and OS capabilities SHALL remain with their existing owners, and Apple FFI support SHALL remain independent.

#### Scenario: Keep domain and JNI ownership separate

- **WHEN** a selected Android bridge validates a call, invokes a domain component, maps its result, and publishes primitive-array output
- **THEN** shared support performs only call-local JNI safety mechanics while the explicit bridge owns its symbol, ABI, limits, status mapping, record, and publication policy

#### Scenario: Add a later Android bridge

- **WHEN** a later Rust protocol candidate receives an Android JNI bridge
- **THEN** it may reuse the same environment/reference/primitive-array support without moving domain behavior, host callbacks, JVM object lifetime, or application ownership into support

#### Scenario: Preserve OS capability ownership

- **WHEN** Android or Apple performs BLE, background, location, permission, file, theme, lifecycle, WebView, loopback, or persistence work
- **THEN** the retained native host remains the sole owner and neither JNI nor FFI support participates
