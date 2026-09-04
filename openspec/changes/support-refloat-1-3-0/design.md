## Context

Float View currently admits nine stable Refloat source releases. Refloat 1.0.x uses fixed legacy telemetry; 1.1.0–1.2.3 use the negotiated commands 31/32 protocol, with command 10 supplying slower values and active firmware-fault observations. The shared Rust compatibility, codecs, services, coordinator, and unified runtime own those production decisions for Apple and Android.

Refloat 1.3.0 retains INFO version 2 and the product-control commands Float View already permits, but adds documented selectable realtime command 33 and labels commands 31/32 internal. Command 33 uses echoed control flags and two masks, supports float16 or float32 values, and publishes the existing state/motor/IMU/control values plus battery state of charge, loop frequency, distance and energy counters, motor d-axis current, and capability-gated GNSS data. It does not publish the active firmware-fault code. Refloat 1.3.0 also changes the compact command-10 booster byte from current in amperes to normalized torque and has a distinct 172-field configuration schema with fingerprint `62681208dc609eb19420c89df65105f018499aa9f5741f295f00d62ea5a1bceb` and wire signature `0x191a6c1b`.

The backend contract already supports nullable stable telemetry and at most 32 finite numeric extensions. OS location remains a separate permission-aware native stream. This change must not reinterpret board GNSS as an OS location observation or introduce an unfinished alert-history model.

## Goals / Non-Goals

**Goals:**

- Admit only exact stable Refloat 1.3.0 and retain every older audited release unchanged.
- Make command 33 the sole primary realtime/slow-data protocol for 1.3.0 and consume every documented non-reserved field.
- Preserve current live alert reason, fatal latch, active firmware-fault, and fatal-clear behavior without adopting unstable alert history.
- Add exact 1.3.0 configuration, control, fixture, conformance, persistence, export, reconnect, and cross-host evidence.
- Keep all protocol work bounded, route-correct, generation-scoped, and owned by the existing shared Rust runtime.

**Non-Goals:**

- Command 35, alert history, alert names, or a new frontend alert contract.
- Commands 31/32 for 1.3.0, Refloat data recording/history, runtime tuning, Handtest, remote control, RC move, or arbitrary motor commands.
- Automatic conversion of saved Tunes between releases.
- Treating board GNSS as phone location, adding location permissions, or changing the OS location lifecycle.
- New third-party dependencies, generated bindings, or a database migration framework.

## Decisions

### Select telemetry by exact release profile

The compatibility catalogue will add exact wire version `1.3.0` with a selectable-command-33 telemetry profile and the existing stable command-20 lighting/fatal-clear control profile. Refloat 1.0.x and 1.1.0–1.2.3 keep their current profiles and request sequences. Discovery will not request command 32 for 1.3.0; an adjacent version or prerelease remains unsupported.

Extending commands 31/32 was rejected because upstream now documents them as internal and 1.3 changed their identifiers and semantics. Migrating old releases to command 33 was rejected because they do not implement it and their proven behavior must remain unchanged.

### Extend the existing safe dynamic codec

`refloat-dynamic-telemetry` will own command-33 request publication and response decoding alongside its existing 31/32 codecs. Command 33 is still dynamic telemetry, and extending the dependency-free safe component avoids another crate, bridge, build entry, and production boundary. The compatibility record retains its bounded extent and gains one closed telemetry-profile discriminator used only by same-build consumers.

Requests will use float32 and only source-defined non-reserved masks. Responses must echo the exact control flags and masks, have the exact mask-derived extent, contain no trailing bytes, and provide finite and range-valid values. Unsupported flags/bits, truncation, mask mismatch, non-finite values, invalid state encodings, or impossible GNSS coordinates fail the complete response without partial publication.

### Use fast and slow command-33 masks

The telemetry service will schedule a fast mask at its existing adaptive riding/idle cadence for state flags, core motor values, temperatures, IMU, footpad, rider input, setpoints, balance current, and loop frequency. A once-per-second slow mask will request battery state of charge, odometer, absolute distance, charging values, energy counters, and motor d-axis current. GNSS bits are added to the slow mask only when INFO advertises GNSS capability. Both requests use the selected direct/CAN route and the one bounded scheduler; neither creates a parallel lane or immediate retry.

The latest accepted slow response replaces the 1.3 slow cache. A later fast response merges that cache into one sample and assigns the normal monotonic sequence. Reset, reconnect, route replacement, and generation changes clear both caches and pending requests.

Requesting every field in every fast response was rejected because slow counters and GNSS would increase BLE traffic without improving their useful cadence. Omitting command-33-only fields was rejected because the change explicitly adopts the complete stable interface.

### Map semantics exactly and use bounded extensions for additive values

Command-33 fields with existing contract semantics map to their current typed fields. Battery SOC maps to `batteryLevel`; signed `motor_id` maps to the existing field-weakening/d-axis current field with corrected source documentation. Loop frequency, data-recording state, and board GNSS values use stable `refloat.*` extension names and flow through existing live, session, query, and export bounds. Board latitude/longitude are validated but remain ordinary package telemetry extensions and never create a `LiveLocation`, location table row, permission request, or location gap.

Fields unavailable from command 33 remain null for 1.3.0. In particular, internal ATR acceleration/boost values and booster torque are not fabricated. The normal 1.3 command-10 booster byte is ignored because it no longer represents amperes; `boosterCurrentA` remains null. Mapping torque into the current field was rejected as dimensionally incorrect.

### Preserve only the current alert surface

Command-33 state flags remain authoritative for alert reason and fatal latch. A bounded once-per-second command-10 mode-0 probe is retained only to observe its special active firmware-fault response; normal command-10 telemetry does not populate 1.3 values. Fatal clearing remains command 36 and still requires a latched fatal error with no active firmware fault.

Command 35 was rejected because upstream marks it unstable and Float View has no complete alert-history product contract. Commands 31/32 are not retained as a secondary 1.3 alert stream because that would preserve the internal dependency command 33 is intended to replace.

### Pin and regenerate exact source evidence

The `vendor/refloat` submodule remains pinned to stable commit `47a2c5ce69f0bba4ef634990a8e53a1c07e07e93`. The configuration generator allowlist will include 1.3.0 and regenerate immutable command-92/93/94/95 fixtures using the installed VESC Tool. Catalogues accept only the computed source fingerprint and wire signature; saved Tunes remain keyed to exact version, fingerprint, and signature.

The conformance fixture will model exact INFO, command-33 masks and responses, the limited command-10 fault probe, existing controls, configuration, direct/CAN routing, malformed responses, reconnect, persistence, and export. The software gate remains `just verify`; real iOS, macOS, Android, and flashed-fixture coverage remains the explicit physical checkpoint.

## Risks / Trade-offs

- [Command 33 is new and variable-length] → Derive response extent from the echoed closed masks, test every supported mask/type combination, and fail closed on mismatch or trailing bytes.
- [Two command-33 cadences plus a fault probe increase scheduler pressure] → Keep one serialized scheduler, preserve foreground telemetry priority, use one-second bounded slow/fault deadlines, and do not retry immediately.
- [GNSS can be unavailable, stale, or internally inconsistent] → Request it only behind INFO capability, validate each complete response, retain source ticks as telemetry, and do not grant it OS-location authority.
- [Extension names become durable/exported identifiers] → Define one canonical `refloat.*` catalogue and test live, storage, query, and export projections across both hosts.
- [The 1.3 schema approaches existing byte bounds] → Pin generated sizes and digests, retain current compressed/expanded limits, and reject any future release independently rather than widening bounds speculatively.
- [Physical 1.3 hardware may not be immediately available] → Require complete source-derived software evidence first and keep the physical BLE checkpoint explicitly incomplete until an actual 1.3.0 package passes it.

## Migration Plan

Land the planning artifacts first. Implement source fixtures and the safe codec/profile slice, then service/runtime composition, configuration/control coverage, and conformance projections in coherent verified commits. Existing databases are disposable test data; if the stable field documentation or extension catalogue changes schema metadata, recreate from the current schema rather than adding migrations. Rollback is the coherent removal of the 1.3.0 catalogue entry, fixtures, and command-33 path while retaining all older profiles; no Tune is migrated or rewritten.

## Open Questions

None. The physical checkpoint records actual device availability but does not alter the selected protocol or scope.
