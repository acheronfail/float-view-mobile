# Native document physical roundtrips

Recorded on 2026-08-09 for task 9.6.

## Android

- A physical Samsung Android host created a current 15-sample session against
  the ESP32 fixture.
- `Share Archive` presented the Android system chooser for the Rust-staged
  77.82 kB `.floatview` file.
- The exported content URI was copied to Downloads only for the checkpoint,
  selected through the system Storage Access Framework picker, and imported by
  Rust as a new session.
- The imported session contained the expected 15 samples. The generated
  Downloads test file was removed after verification.
- `android-physical-document-seed.json` records the physical identity and ride
  scenarios used to create the source session.

## macOS

- The first full-app attempt exposed a real containment mismatch: a configured
  `FLOAT_VIEW_STORAGE_ROOT` was passed to Rust while the native document host
  still validated against its default Application Support root. The host and
  Rust startup now share the exact configured durable root, with a focused
  regression test.
- The rebuilt full macOS application presented its real `NSOpenPanel`, selected
  the valid Android-generated archive, and Rust completed the operation as
  session `imported-6`.
- Exporting that imported session presented the real `NSSavePanel` and wrote
  `/private/tmp/float-view-macos-roundtrip.floatview`.
- The input and macOS export are valid Float View SQLite archives. Both contain
  two archive metadata rows, one session, and 15 samples, with zero location,
  event, riding-interval, and numeric-field rows.

## iOS

The focused Swift presenter and bounded security-scoped copy tests pass. The
physical picker/share roundtrip remains pending human confirmation because the
iOS system surfaces cannot be completed through the loopback conformance
driver.
