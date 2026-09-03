## ADDED Requirements

### Requirement: Apple selects the unified runtime independently

The shared iOS/macOS backend SHALL select the unified Rust runtime through one coarse audited Apple boundary only after deleting its superseded native application/protocol/session decisions and passing complete Apple software and physical gates. Android SHALL remain the production owner of its current application behavior and continue using its temporary JNI bridges until a separate deletion-bearing cutover.

#### Scenario: Complete Apple selection

- **WHEN** the Apple deletion, packaging, canonical conformance, and connected iOS fixture gates pass
- **THEN** Apple becomes the first production selector of the unified runtime while Android ownership is unchanged

#### Scenario: Preserve the final convergence gate

- **WHEN** only Apple has selected the runtime
- **THEN** the migration remains incomplete and bridge retirement cannot remove Android scaffolding until Android independently passes its cutover
