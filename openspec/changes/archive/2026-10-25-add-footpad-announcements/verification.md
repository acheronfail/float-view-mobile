# Verification

Verified on 2026-10-25 against this change's working tree.

- Rust: affected `float-view-product-store`, `float-view-backend-service` and
  `float-view-loopback-server` suites passed, including frozen baseline catalog
  and archive coverage. Existing ignored scale tests remained ignored. Host,
  Apple FFI and Android JNI consumer tests passed. Focused announcement policy
  tests also cover exact timing, independent sides, fixed/reverse speed gate,
  saved/guest ownership, continuity resets, shared cooldown, unavailable/muted/busy
  admission, contact recovery, late completions and missing-contact fallback.
- Rust formatting and warnings-denied Clippy passed for affected crates and
  targets. Baseline fixture files were not modified.
- Frontend: 534 unit tests passed across the ordinary suite and the four socket
  tests rerun outside the sandbox. Svelte check had zero errors/warnings; build,
  affected ESLint and Prettier checks passed.
- Canonical browser scenario passed on Chromium and WebKit with mouse, touch and
  keyboard input, disabled-delay retention, frontend reload, serialized saves and
  save-failure retention. WebKit touch range input required committing on primary
  pointer release because it can emit `input` without `change` or focus.
- Canonical direct automatic scenario passed using existing interactive telemetry
  and Refloat wire encoding: footpad-only effects, persistent repeats, combined
  load/speed effects, recovery and reconnect requalification. Software fixture
  and relay regression tests and conformance TypeScript/Svelte checks passed.
- Focused announcement contract scenario passed on direct, mock, macOS, iOS
  simulator and Android emulator targets. Run with
  `FLOAT_VIEW_CONFORMANCE_SCENARIOS=announcements npm run <target>`.
- Apple speech request/adapter tests passed (13 tests), including exact closed
  wording/order and footpad-only utterances through all terminal cleanup paths.
  Android speech request/executor tests passed, including closed key/value
  validation, ordering and footpad-only focus/cleanup paths.

Physical audibility, OS BLE and background playback were not revalidated. This
change adds typed phrase content and shared policy without changing the native
synthesis, audio-session/focus or background drivers, or introducing a telemetry
command. The scoped software checks establish the changed behavior; no additional
physical checkpoint was required. No claim of physical validation is made and the
ESP32 fixture was not used.

Strict OpenSpec validation and `git diff --check` passed. No pull-request edits or online changes were made.

## Sync and archive checks

The six footpad requirements were synced to the main ride-announcements spec and
the completed change archived on 2026-10-25. Strict change validation passed before
archive and `git diff --check` passed after sync. Strict main-spec validation
reports one pre-existing >500-character warning in “Native speech and graceful
audio fallback”; that requirement is byte-for-byte unchanged. Repository-wide
strict validation also reports existing long-requirement warnings in other specs.
