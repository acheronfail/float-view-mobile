# iOS Live Activity lifecycle checkpoint

The [native controller](../../backends/apple/ios/float-view/live-activity/RideActivityController.swift)
and [shared projection](../../backends/core/backend-service/src/live_activity.rs)
were checked on one physical iPhone with the production CoreBluetooth adapter and
the ESP32 fixture, using a guest connection.

Native ActivityKit submission checks passed for stationary readiness, about one
minute of background idle followed by riding, and a short fixture power cycle
followed by reconnect while the app remained backgrounded. The original activity
received unavailable telemetry during reconnect and fresh updates afterward;
there was no intervening activity end or replacement start in the captured logs.
The guest connection was disconnected afterward, the saved-board count was
unchanged, and the fixture scenario was restored.

The user also confirmed that the Live Activity was visible after backgrounding
the stationary fixture connection, on the Dynamic Island or Lock Screen. This
confirms background visibility, but does not visually establish freshness or
reconnect recovery. XCTest could not enable UI automation on this phone; visual
reconnect recovery, manual dismissal, and the explicit Shortcuts action remain
unverified.
The eight-hour system expiry and retry/dismissal decisions have software coverage;
this checkpoint does not establish hours-long background behavior or other iOS
versions. Detailed build/device evidence and the fixture advertising failure
encountered before the clean power-cycle check remain in ignored
`.local/physical-testing/live-activity-20261018/`.
