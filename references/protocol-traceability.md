# First-slice protocol traceability

| Behavior | Canonical reference | Supplementary evidence | Fixture |
| --- | --- | --- | --- |
| VESC short/long framing and CRC16-CCITT | VESC Tool `packet.cpp`, `packet.h` at pinned revision | Legacy Swift/Kotlin framing (CRC validation was incomplete) | `fixtures/vesc/framing.json` |
| Firmware version response | VESC Tool `commands.cpp`, `datatypes.h` | Legacy `VescCommandParser` | `fixtures/vesc/firmware-6.06.json` |
| VESC Express transport firmware identity | VESC Tool `commands.cpp`, `datatypes.h` | Sanitized Express 6.05 hardware name/type observations | `fixtures/vesc/firmware-express-6.05.json` |
| `COMM_GET_VALUES` normalized telemetry | VESC Tool `commands.cpp`, `datatypes.h` | Floaty and legacy parser field scaling | `fixtures/vesc/realtime-values.json` |
| BLE UART service and characteristic UUIDs | VESC Tool `bleuart.cpp`, `bleuart.h` | Legacy CoreBluetooth/GATT adapters | `fixtures/vesc/gatt-profile.json` |
| Refloat package INFO v2, realtime field IDs, state/footpad flags and float16 telemetry | Refloat v1.2.0 `INFO.md`, `REALTIME_DATA.md`, `REALTIME_DATA_IDS.md`, `float16.md`, `src/main.c`, and `src/rt_data.h` at `747a2f4` | Android 10 read-only capture from Thor300 VESC 6.06 / Refloat 1.2.3 through VESC Express; Floaty legacy behavior | `fixtures/refloat/rtdata-1.2.3-android-hardware.json`; `fixtures/refloat/rtdata-1.2.json` remains a legacy synthetic compatibility fixture |
| Refloat deterministic ride and footpad-state transitions | Refloat v1.2.0 realtime-data bit layout at pinned revision | State/footpad-only mutations of the sanitized Refloat 1.2.3 capture with regenerated CRCs | `fixtures/refloat/ride-states-1.2.3.json` |

The fixtures validate framing, chunk reassembly, CRC, scaling, and normalized
model parity. The old Refloat payload is deliberately marked synthetic and
represents the legacy command. Modern Refloat 1.2 INFO/field-ID/realtime behavior
is exercised in Swift and Kotlin using the sanitized Android hardware capture.
Its controller is known to report invalid sensor values, so the fixture proves
protocol interpretation and parity but deliberately makes no numeric
plausibility claim.
