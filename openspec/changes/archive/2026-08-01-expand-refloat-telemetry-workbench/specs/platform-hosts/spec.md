## ADDED Requirements

### Requirement: Mirrored detailed telemetry ownership

Apple and Android SHALL retain mirrored owners for Refloat family decoding, telemetry scheduling, semantic normalization, application state, durable persistence, session retrieval, and serialization. VESC command routing SHALL remain below Refloat polling, and neither protocol owner SHALL depend on Bluetooth, storage, loopback transport, or frontend code.

#### Scenario: Add a known telemetry field

- **WHEN** support for an existing negotiated Refloat field is added
- **THEN** the matching Swift and Kotlin decoder, normalized model, persistence mapping, serialization, fixtures, and tests are updated without adding platform-specific frontend logic

#### Scenario: Route ownership

- **WHEN** a detailed Refloat request targets a CAN controller
- **THEN** the Refloat owner supplies the command and the VESC routing owner applies CAN forwarding exactly once

### Requirement: Equivalent native detailed persistence

The Apple and Android current schemas SHALL durably store the same detailed typed values in bounded `detail_json`, unknown negotiated values in bounded `extension_json`, and discrete transitions in the existing `session_event` table with an independent monotonic event sequence, observation time, elapsed time, and single-session ownership. Multiple transitions from one sample SHALL be inserted in stable catalog order. Existing summary/query fields SHALL remain columns. Incompatible test-stage databases SHALL be deleted and recreated from the current schema rather than migrated.

#### Scenario: Equivalent stored sample

- **WHEN** both native backends accept equivalent normalized telemetry
- **THEN** session retrieval and export produce semantically equivalent numeric records and transition events despite native storage implementation differences

#### Scenario: Old disposable schema

- **WHEN** a development database has the previous incompatible telemetry schema
- **THEN** normal disposable-schema handling recreates it without a migration framework or partial column fallback

### Requirement: Native lifecycle owns detailed polling

Detailed realtime polling, the one-second compact due time, normalization, and durable commits SHALL remain native when the WebView is absent or recreated. Each host SHALL retain one in-flight request and no separate telemetry work queue. Background and reconnect behavior SHALL follow the existing platform lifecycle rules and prevent stale responses or a replacement controller from contributing detailed samples or events to an active session.

#### Scenario: WebView replacement

- **WHEN** the frontend disappears while native detailed telemetry continues
- **THEN** accepted samples remain durably ordered and a replacement frontend recovers the latest committed sample

#### Scenario: Controller changes

- **WHEN** reconnect validation reports a replacement controller
- **THEN** detailed polling stops, the old session ends, route-scoped compact state is cleared, and no replacement-controller fields or events are appended to the old session
