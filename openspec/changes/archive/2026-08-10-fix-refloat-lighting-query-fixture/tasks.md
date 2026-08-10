## 1. Lock source semantics

- [x] 1.1 Add deterministic legacy and stable zero-mask lighting query coverage, including exact responses and unchanged state.
- [x] 1.2 Add direct/CAN and active-flywheel regression coverage that distinguishes read-only queries from nonzero-mask mutations.

## 2. Correct the shared fixture model

- [x] 2.1 Decode zero-mask command 202 and command 20 requests as explicit lighting status queries rather than LED mutations.
- [x] 2.2 Return current lighting state for queries without mutation while retaining route, firmware, shape, and flywheel mutation gates.

## 3. Verify software

- [x] 3.1 Run focused conformance Refloat-control tests and strict OpenSpec validation.
- [x] 3.2 Run the complete `just verify` software gate.

## 4. Verify physical control regression

- [x] 4.1 Run focused `control-direct` and `control-can` scenarios on the connected Android device against the ESP32 fixture.
- [x] 4.2 Run the same focused scenarios on the connected iOS device against the ESP32 fixture.
- [x] 4.3 Run the same focused scenarios on the macOS host against the ESP32 fixture.
