# Apple background capability evidence

Verified on 2026-08-09 for task 12.3.

- iOS declares `bluetooth-central` and `location` in `ios/App-Info.plist`, with
  the required always-and-when-in-use location purpose string.
- `FloatViewBluetooth.make` constructs the production `CoreBluetoothAdapter`
  with the stable restoration identifier
  `fail.acheron.float-view.rust-bluetooth-central`; its restoration delegate
  retains only the private peripheral binding for a later Rust-owned connect.
- `CoreLocationAdapter` disables automatic pausing, enables background updates
  on iOS, and sends delegate observations to the Rust bridge independently of
  any WebView.
- `ios-physical-background-60s.json` records an active session accumulating BLE
  samples through a 60-second background dwell, followed by the same device and
  active session recovering through a replacement WebView. The fixture reached
  140 telemetry responses. Location authorization and effective background
  availability were also observed; no coordinate arrived during the stationary
  indoor dwell, so physical coordinate persistence remains explicit rather than
  inferred from authorization.
- macOS uses normal application-process eligibility: closing or inactivating a
  window does not create a mobile service. Its explicit mobile-service stub
  returns unavailable while lifecycle eligibility remains true.

Rust remains the owner of durable logging intent, session state, observation
policy, and persistence on both platforms.
