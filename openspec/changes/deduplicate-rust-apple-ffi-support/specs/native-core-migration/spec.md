## ADDED Requirements

### Requirement: Narrow Apple FFI support ownership

Shared Rust Apple FFI support SHALL own only protocol-neutral C-boundary mechanics common to every selected Apple bridge. VESC and Refloat framing, commands, decoding, encoding, record schemas, status mappings, compatibility, routing, scheduling, application behavior, persistence, transport, and OS capabilities SHALL remain with their existing owners, and Android JNI support SHALL remain independent until a separate change.

#### Scenario: Maintain downward dependencies

- **WHEN** an Apple bridge uses shared boundary support
- **THEN** the dependency graph remains host bridge to support and domain crate, support depends on neither, and no domain or host framework dependency points upward into a bridge

#### Scenario: Add a later Apple bridge

- **WHEN** a later protocol candidate receives an Apple C bridge
- **THEN** it can reuse the bounded support primitives without adding protocol knowledge, exported symbols, headers, generated bindings, or platform behavior to the support crate

#### Scenario: Preserve Android isolation

- **WHEN** Apple FFI support is built, tested, packaged, and executed
- **THEN** Android JNI crates, libraries, symbols, loaders, Gradle inputs, and production calls remain unchanged and do not link the Apple support component
