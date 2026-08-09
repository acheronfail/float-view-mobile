# Android physical background observation

Date: 2026-08-09  
Device: Samsung SM-G965F  
Fixture: ESP32 VESC test device

The physical `background` scenario established the production BLE connection,
completed notification setup, identified the Express/controller route, and
continued receiving fragmented telemetry after the Activity was backgrounded.
The fixture observed 38 telemetry responses across idle, riding, footpad
warning, post-ride-tail, and return-to-idle phases. Bringing the Activity back
to the foreground reused the process-owned runtime and loopback endpoint.

Android reported location authorization and background capture as available,
but the checkpoint persisted no coordinate. Read-only `dumpsys location`
evidence showed location mode and GPS/network providers enabled, no last-known
location, no live fix during the run, and an observed cold GNSS TTFF of about
40 seconds. The former 10-second physical dwell was therefore insufficient.
Coordinate persistence was subsequently confirmed independently by the user on
this Android device on 2026-08-09, completing the Android GPS/background-
location checkpoint. The automated physical-only dwell remains 60 seconds so
future runs have enough time to acquire a cold GNSS fix.

An earlier full-matrix attempt also produced transient Android GATT status 133
after repeated cold connections. The Kotlin adapter closed the failed GATT, but
Rust did not retry while identification was still `WaitingBluetooth`. The
shared Rust reconnect policy now handles initial failures as well as established
link loss. Five rapid direct-connection scenarios subsequently passed, followed
by the retained 16-scenario matrix in `android-physical-matrix.json`.
