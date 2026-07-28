# T2 host protocol and scenario engine evidence

Date: 2026-07-28

Status: complete.

## Implemented

- Version 1 JSON-line USB client with monotonic command sequences, exact
  acknowledgement correlation, bounded timeouts, reset handling, stale-event
  handling, and explicit close behavior.
- JSON Schema 2020-12 scenario validation with a versioned, declarative first
  identity scenario.
- Independent VESC short/long frame encoding, CRC16-CCITT validation, partial
  stream assembly, combined-frame handling, noise recovery, and bounded input.
- Exact read-only request matching for direct firmware, CAN ping, forwarded
  firmware, Refloat INFO, dynamic telemetry IDs, and telemetry.
- Default-deny safety handling: known motor/configuration/firmware commands are
  classified as mutating; all other non-whitelisted commands are unknown; both
  are rejected and recorded.
- Fixture-backed response and notification chunk planning for VESC Express 6.05,
  Thor300 6.06, Refloat 1.2.3 INFO, its captured dynamic field layout, and
  deterministic idle telemetry.
- Bounded transcripts, deterministic rule counts/order, first-violation
  reporting, and non-overwriting machine-readable evidence output.
- An opt-in macOS verifier that compiles the production CoreBluetooth adapter
  and VESC codec and orchestrates the ESP over the acknowledged USB transport.

## Host verification

Run from `tools/vesc-test-device/host`:

```text
npm run check
  PASS

npm test
  PASS: 20 tests
```

Covered adversarial cases include partial and combined frames, noise, CRC
corruption and recovery, long frames, invalid scenario documents, unknown and
mutating commands, out-of-order and duplicate requests, exact fixture
reproduction, host timeout, ESP reset, stale acknowledgements, mismatched
acknowledgement kinds, and evidence overwrite refusal.

The macOS Swift verifier sources also pass compile-only validation. Compile-only
validation does not arm, advertise, or connect the ESP.

## Physical macOS/ESP verification

After explicit operator approval, the physical gate ran with:

```sh
cd tools/vesc-test-device/host
npm run verify:macos -- --port "$FLOAT_VIEW_ESP_PORT"
```

The production macOS CoreBluetooth adapter discovered the ESP by the canonical
VESC service, connected, enabled notifications, and used the production VESC
codec for the entire sequence. The generated ignored evidence reported:

```text
scenarioId: express-thor-refloat-identity
passed: true
firstViolation: null
transportFirmware: 1
canPing: 1
primaryFirmware: 1
refloatInfo: 1
refloatTelemetryIds: 1
refloatTelemetry: 1
transcriptEntries: 43
transcriptTruncated: false
```

The transcript contains six accepted client requests in the declared order,
37 notification chunks, and zero rejected requests. Generated local artifacts
are stored under `artifacts/verification/vesc-test-device/` and remain ignored.
