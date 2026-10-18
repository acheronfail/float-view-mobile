# session-connection-context Specification

## Purpose

Preserve the firmware and connection path that produced a saved ride, so session review and portable archives retain accurate provenance across reconnects and later board changes.

## Requirements

### Requirement: Recorded firmware and connection path

Saved sessions SHALL retain typed `connectionContext` events containing the observed Refloat version, selected controller VESC version and test version, nullable public hardware/firmware names, Bluetooth peer firmware facts, and direct/CAN controller route. CAN routes SHALL include the selected CAN ID. Context SHALL exclude addresses, UUIDs, private device identifiers and unrelated discovered nodes.

#### Scenario: Direct connection

- **WHEN** the selected motor controller is the Bluetooth peer
- **THEN** the event records a direct route and independently describes the peer and controller firmware

#### Scenario: Express or controller gateway

- **WHEN** the selected motor controller is reached through a Bluetooth peer over CAN
- **THEN** the event records that peer's public firmware facts separately from the selected motor's firmware and CAN ID, including when the peer is VESC Express

#### Scenario: Missing optional names

- **WHEN** identification provides versions and routing but no optional public firmware name
- **THEN** the name remains null without guessing from advertisements or current board settings

### Requirement: Context capture follows activation history

Each successful saved-session protocol activation SHALL durably record one context before its telemetry is persisted or readiness is published. Every reconnect/resume SHALL record a new context even if values match. Context SHALL have observed time, session elapsed time and stable event sequence. Duplicate handling of one activation SHALL not append duplicates; guest and unsuccessful identification SHALL create no context.

#### Scenario: Initial activation without telemetry

- **WHEN** identification succeeds for a new saved session before any telemetry sample arrives
- **THEN** context is committed independently of a sample and is available for review

#### Scenario: Unchanged reconnect

- **WHEN** the same controller resumes the same session with unchanged firmware and route
- **THEN** another timestamped context records the successful reconnect without replacing the previous event

#### Scenario: Changed route or firmware

- **WHEN** an eligible session resumes through another route or with changed observed firmware
- **THEN** the new context contains the fresh facts and earlier context remains unchanged

#### Scenario: Process and WebView lifecycle

- **WHEN** a WebView is replaced without protocol activation
- **THEN** no context is appended
- **WHEN** a process restarts and successfully reactivates an eligible recovered session
- **THEN** previous contexts remain and a new context uses the restored session timeline

#### Scenario: Duplicate or stale activation

- **WHEN** the same activation completion is processed twice or a stale completion arrives
- **THEN** the same activation creates at most one context and stale facts do not enter the active session

#### Scenario: Guest or failed identification

- **WHEN** a guest connects or saved identification fails before activation
- **THEN** no durable context is created for that attempt

### Requirement: Bounded durable context admission

Context SHALL be limited to 1,000 events per session and 4,096 UTF-8 bytes per payload, with each public name at most 128 UTF-8 bytes. Context and its published commit boundary SHALL be atomic. Invalid facts, capacity exhaustion or storage failure SHALL fail saved activation actionably without silently omitting context, evicting history, resetting data or publishing recording readiness.

#### Scenario: Storage or capacity failure

- **WHEN** the context write fails or the session has reached its context limit
- **THEN** the existing bounded failure flow reports the failure, preserves prior committed data and does not persist telemetry for the unrecorded activation

#### Scenario: Interrupted commit

- **WHEN** the process exits during context persistence
- **THEN** recovery exposes either the complete committed context and boundary or neither, without a partially published event

### Requirement: Session details show recorded context history

Session details SHALL show recorded Refloat and controller firmware versions, test-version information and connection path, with chronological initial/reconnect entries and elapsed times. Review SHALL use recorded facts independently of the current board. History navigation and retained UI collections SHALL be bounded and usable with touch, pointer, keyboard and assistive activation.

#### Scenario: Review a gateway session

- **WHEN** a context records Express firmware and a CAN motor route
- **THEN** details distinguish peer and motor versions and describe the path as via VESC Express and the selected CAN ID

#### Scenario: Older session

- **WHEN** a supported session has no context events or predates capture until a later resume
- **THEN** details show Not recorded for the missing history without backfilling from a live connection or implying a later context applied earlier

#### Scenario: Active session refresh

- **WHEN** a new context commits while the active session is being reviewed
- **THEN** ordinary metadata refresh makes it available without clearing chart/map inspection or mixing pages from different published snapshots

#### Scenario: Long history or late response

- **WHEN** history exceeds one page or the viewer changes session while a page is pending
- **THEN** bounded navigation reaches every context and obsolete responses cannot replace the current session's details

### Requirement: Context survives session maintenance and portability

Committed context SHALL survive disconnect, restart/recovery, telemetry/location pruning and native archive export/import with payload, identity, ordering and timestamps intact. Context alone SHALL not exempt a session from existing automatic discard rules. Explicit session/board deletion SHALL remove it. CSV SHALL remain telemetry-only.

#### Scenario: Prune and archive round trip

- **WHEN** a session is pruned and exported then imported on another supported platform
- **THEN** all committed contexts remain exact even when adjacent telemetry samples were removed

#### Scenario: Supported baseline and malformed import

- **WHEN** a supported baseline archive has no context
- **THEN** it imports with empty context history
- **WHEN** imported context has invalid types, versions, route IDs, ordering, size or count
- **THEN** import fails without partially publishing a session or changing existing sessions

#### Scenario: Normal session retention

- **WHEN** an otherwise discardable session contains only connection context
- **THEN** existing no-riding and minimum-distance policies still apply
- **WHEN** a user explicitly deletes a retained session or forgets its board
- **THEN** its context is deleted with the owned session data
