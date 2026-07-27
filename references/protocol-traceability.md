# First-slice protocol traceability

| Behavior | Canonical reference | Supplementary evidence | Fixture |
| --- | --- | --- | --- |
| VESC short/long framing and CRC16-CCITT | VESC Tool `packet.cpp`, `packet.h` at pinned revision | Legacy Swift/Kotlin framing (CRC validation was incomplete) | `fixtures/vesc/framing.json` |
| Firmware version response | VESC Tool `commands.cpp`, `datatypes.h` | Legacy `VescCommandParser` | `fixtures/vesc/firmware-6.06.json` |
| `COMM_GET_VALUES` normalized telemetry | VESC Tool `commands.cpp`, `datatypes.h` | Floaty and legacy parser field scaling | `fixtures/vesc/realtime-values.json` |
| BLE UART service and characteristic UUIDs | VESC Tool `bleuart.cpp`, `bleuart.h` | Legacy CoreBluetooth/GATT adapters | `fixtures/vesc/gatt-profile.json` |
| Refloat custom app ID and RT data packet | Refloat 1.2 hardware capture required before R6 | Floaty and legacy command enums/parsers | `fixtures/refloat/rtdata-1.2.json` |

The synthetic protocol fixtures below validate framing, chunk reassembly, CRC,
scaling, and normalized model parity. The Refloat payload is deliberately marked
synthetic; R6 must add a sanitized Thor/XRV/PintV hardware capture before claiming
hardware compatibility.
