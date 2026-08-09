# Explicit physical host checkpoints

Recorded on 2026-08-09 for task 9.6. Platform transport identifiers and
loopback tokens are intentionally omitted.

| Checkpoint | iOS | macOS | Android |
| --- | --- | --- | --- |
| Default-MTU BLE writes | pass — firmware and telemetry responses arrived in 20-byte fragments | pass — full 16-scenario fixture matrix | pass — retained 16-scenario fixture matrix |
| Notification setup | pass — physical fixture notifications received | pass — notification enablement and raw notifications observed | pass — CCCD setup and raw notifications observed |
| Saved-device reconnect | pass — identity/reconnect reports | pass — identity and reconnect scenarios | pass — identity/reconnect matrix plus initial-failure retry |
| Background BLE logging | pass — 60-second dwell retained one runtime/session and reached 140 telemetry responses | pass — normal process/window lifecycle retains runtime | pass — foreground service retained runtime/session |
| Background GPS logging | pending coordinate — authorization and background capture are available, but stationary 60-second dwell persisted no row | not a mobile background-service capability; physical authorization/provider transition and focused observation tests pass | pass — confirmed independently by the user on the physical device |
| Frontend replacement | pass — replacement WebView recovered active connection/session | pass — WebContent replacement retained runtime | pass — Activity/WebView recreation retained runtime |
| Stale callback rejection | pass — reconnect generation scenarios and focused adapter evidence | pass — unexpected-disconnect and focused generation evidence | pass — unexpected-disconnect and focused generation evidence |
| Native documents | pending human iOS picker/share roundtrip | pass — real open/save panels and 15-sample roundtrip | pass — real SAF/share surfaces and 15-sample roundtrip |
| Live appearance | pass | pass | pass |

Evidence files:

- `ios-physical-matrix.json`, `ios-physical-identity-reconnect.json`, and
  `ios-physical-background-60s.json`;
- `macos-physical-matrix.json` and `native-document-roundtrips.md`;
- `android-physical-matrix.json`,
  `android-physical-background-observation.md`, and
  `android-physical-document-seed.json`;
- `appearance-verification.md` for the complete three-host appearance matrix.
- `macos-location-capability.md` for embedded usage metadata and the physical
  authorization/provider transition.
