## ADDED Requirements

### Requirement: Rust Refloat configuration codec candidate parity

The bounded Rust candidate SHALL independently reproduce the selected native Refloat configuration schema and value semantics without becoming a supported backend or production runtime. It SHALL decode the exact Qt-compressed and expanded schema forms, validate and normalize the closed field/group model, calculate the canonical source fingerprint and wire signature, and decode or encode complete typed value sets with the same bounded deterministic results as the audited Swift and Kotlin owners. It MUST return closed payload-free failures, expose no partial result, forbid unsafe code in candidate-owned source, and retain no raw schema or configuration bytes after normalization.

#### Scenario: Decode every audited schema

- **WHEN** the candidate receives each immutable source-derived Refloat 1.0.0–1.2.3 Qt-compressed schema
- **THEN** it produces the exact fingerprint, signature, ordered fields, supported types and encodings, defaults, bounds, scale, choices, sanitized descriptions, groups, and fixture counts

#### Scenario: Round-trip every audited value family

- **WHEN** the candidate decodes each release's default, current, accepted-write, and expected-readback bytes against its exact schema
- **THEN** it validates the signature and complete serialized order and re-encodes the exact canonical bytes without a remainder or fabricated value

#### Scenario: Reject hostile schema input

- **WHEN** a compressed envelope, zlib stream, XML document, field definition, serialization order, grouping structure, metadata value, or resource extent violates a canonical bound or invariant
- **THEN** the candidate returns the corresponding closed failure without input-bearing diagnostics, partial schema publication, unbounded allocation, entity expansion, or host callback

#### Scenario: Reject hostile values

- **WHEN** values have a mismatched signature, truncation, remainder, invalid UTF-8 or terminator, non-finite or out-of-bounds number, unencodable fixed point, invalid enum/bitfield/boolean, wrong type, or missing or extra field
- **THEN** decode or encode fails before any partial value set or write payload is published

#### Scenario: Preserve production isolation

- **WHEN** the candidate is added to the locked workspace and complete software verification runs
- **THEN** Swift and Kotlin remain the only selected production codec owners and no Apple FFI, Android JNI, bridge catalogue entry, native package dependency, generated binding, exported symbol, or packaged candidate artifact exists

### Requirement: Bounded safe Refloat configuration dependencies

The candidate SHALL use only locked maintained safe Rust dependencies for zlib decompression, streaming XML processing, and SHA-256 hashing. The build SHALL use a pure-Rust decompression backend, SHALL work through the repository's offline locked verification path, and SHALL audit the exact direct dependency set and transitive graph. Candidate-owned source MUST implement no decompressor, XML tokenizer, cryptographic hash, host ABI, filesystem access, networking, scheduling, persistence, or OS integration.

#### Scenario: Verify locked dependency ownership

- **WHEN** workspace dependency and unsafe-code audits inspect the candidate and its locked graph
- **THEN** only the approved decompression, XML, and SHA-256 libraries and their audited transitive dependencies are present, candidate source forbids unsafe code, and no native zlib or platform framework is linked

#### Scenario: Build and test offline

- **WHEN** focused and complete repository gates run without dependency resolution or network access
- **THEN** the exact locked candidate builds, lints with warnings denied, and passes all source-derived and adversarial tests reproducibly
