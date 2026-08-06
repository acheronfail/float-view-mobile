# VESC BMS source-derived fixtures

`source-derived.json` is the immutable cross-platform byte source for stock VESC
BMS and VESC Express VBMS32 identities and command-96 values. Swift, Kotlin,
and conformance tests consume the same cases.

## Accepted identity audit

| Fixture                | Upstream source                                                         | Exact compatibility entry                                               |
| ---------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `vbms-firmware-6.00`   | `vendor/vesc_bms_fw/commands.c`                                         | version 6.00 and `HW_TYPE_VESC_BMS` (`1`)                               |
| `vbms32-firmware-6.06` | `vendor/vesc_express` commit `a9f496c48107ae507b2303157683843f3bbf236c` | hardware name `VBMS32`, version 6.06, and `HW_TYPE_CUSTOM_MODULE` (`2`) |

The VBMS32 vector follows `main/commands.c`: command and version, terminated
`HW_NAME`, six deterministic eFuse-MAC bytes plus six zero UUID bytes, pairing
and test markers, custom-module type and configuration count, four capability
bytes, an empty terminated firmware name, and a four-byte deterministic fixture
CRC. The UUID and CRC are test data, not a physical device capture.

At the pinned 6.06 revision, `main/commands.c` routes
`COMM_BMS_GET_VALUES` to `main/bms.c`. Its field order and encoders match the
retained legacy and extended values fixtures; VESC Tool owns decoding of the
optional data-version and status tail.

## VESC Express 7.00 audit boundary

The offline checkout was inspected at
`9b8370ddfcfa8793256d718f9d1908a513f04fee` (firmware 7.00). Its identity remains
a custom-module layout and its BMS dispatcher remains present, but the version
changed and the custom-configuration count can now be extended dynamically.
There is no retained physical 7.00 VBMS32 capture or cross-platform conformance
result. Float View therefore rejects VBMS32 7.00 as a BMS candidate.

Accepting it requires a new immutable identity and values audit, mirrored native
tests, canonical conformance, documentation update, and physical Apple/Android
checkpoint. Structural decoder acceptance alone is not compatibility approval.
