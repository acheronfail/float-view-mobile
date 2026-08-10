## 1. Lock native regression behavior

- [x] 1.1 Add an Apple application-service test covering flywheel enable confirmation, repeated flywheel telemetry during disable, and normal-mode disable confirmation.
- [x] 1.2 Add the equivalent Android application-service test and assert neither platform reports a false confirmation timeout.
- [x] 1.3 Extend Apple and Android coordinator tests so generation-correct failure is a no-op when no Refloat action is pending.

## 2. Correct cross-platform confirmation ordering

- [x] 2.1 Reorder Apple authoritative telemetry handling so confirmation occurs before flywheel cleanup and only a remaining eligible flywheel action is preserved.
- [x] 2.2 Apply the equivalent Android ordering and pending-action behavior.
- [x] 2.3 Tighten both mutation coordinators and service failure paths against duplicate or absent pending completion.

## 3. Verify software

- [x] 3.1 Run focused Apple and Android native tests plus strict OpenSpec validation.
- [x] 3.2 Run the complete `just verify` software gate.

## 4. Verify physical control regression

- [x] 4.1 Run focused `control-direct` and `control-can` scenarios on the connected Android device against the ESP32 fixture.
- [x] 4.2 Run the same focused scenarios on the connected iOS device against the ESP32 fixture.
- [x] 4.3 Run the same focused scenarios on the macOS host against the ESP32 fixture.
