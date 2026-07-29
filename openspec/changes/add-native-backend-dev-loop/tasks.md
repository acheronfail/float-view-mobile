## 1. Native Port Registry

- [x] 1.1 Define the macOS `11010`, iOS `11011`, and Android `11012` loopback defaults at their owning native hosts, remove the superseded `8765` defaults, and keep every listener bound only to `127.0.0.1`.
- [x] 1.2 Make normal macOS, iOS, and Android startup request the assigned platform port without an ephemeral fallback.
- [x] 1.3 Add focused Swift and Kotlin coverage proving each host selects its assigned port and reports a clear startup failure when that address is unavailable.

## 2. Frontend Development Transport

- [x] 2.1 Configure Vite to bind strictly to `127.0.0.1:11002`, select either mock port `11003` or one explicit native loopback target, and start the mock only for mock-backed development.
- [x] 2.2 Route both bootstrap and WebSocket development traffic through Vite and rewrite only the upstream WebSocket Origin to the selected backend origin.
- [x] 2.3 Normalize backend WebSocket endpoints to the current Vite origin only in development builds while preserving path, query, launch token, and production bootstrap behavior.
- [x] 2.4 Add frontend tests for mock/native proxy selection, development WebSocket normalization, production endpoint preservation, and invalid-token rejection through the proxy.

## 3. macOS Browser Development

- [x] 3.1 Add a macOS browser-host launch mode that starts `NSApplication`, `MacBackendRuntime`, port `11010`, readiness reporting, and `MacExportPresenter` without creating a WKWebView.
- [x] 3.2 Add bounded `just dev mock` and `just dev macos` orchestration that waits for backend then Vite readiness, exposes `http://127.0.0.1:11002`, supervises both children, and cleans them up on interruption or failure.
- [x] 3.3 Verify browser reload and Vite hot replacement reconnect to preserved macOS CoreBluetooth and backend state, and verify native permission/export presentation remains host-owned.

## 4. Android Browser Development

- [x] 4.1 Add `just dev android [target]` setup that selects one device, builds and installs the Debug application, preserves its storage and permissions, and launches its production Android backend on device port `11012`.
- [x] 4.2 Create and verify `adb forward tcp:11012 tcp:11012`, wait for forwarded bootstrap readiness before starting Vite, and remove only that owned forwarding rule during bounded cleanup.
- [x] 4.3 Add focused tests for Android development command arguments, explicit multi-device target selection, readiness failure, forwarding collision, child failure, and cleanup behavior.

## 5. iOS Browser Development

- [x] 5.1 Add `just dev ios [target]` setup that selects a simulator or physical device, performs the required Debug build/install/launch without resetting app data or authorization, and starts the iOS backend on device port `11011`.
- [x] 5.2 Start and monitor `iproxy 11011:11011` for a physical target, wait for forwarded bootstrap readiness before starting Vite, and terminate only the owned proxy during bounded cleanup.
- [x] 5.3 Add focused tests for iOS development build, launch, target selection, proxy supervision, readiness failure, child failure, and cleanup argument construction.

## 6. Conformance Port Migration

- [x] 6.1 Update macOS conformance readiness to use `11010`, iOS launch and same-number forwarding to use `11011`, Android launch and same-number forwarding to use `11012`, and standalone mock startup to use `11003`.
- [x] 6.2 Update conformance target tests to assert the platform registry, exact expected Origin, same-number mobile forwarding, and removal of only the adapter-owned rule.
- [x] 6.3 Remove all remaining active `3001`, `5173`, `8765`, and `8766` development or conformance defaults and add an audit assertion preventing their reintroduction.

## 7. Verification

- [x] 7.1 Run focused frontend proxy/bootstrap tests, Swift backend and macOS host tests, Kotlin host/server tests, and conformance target tests.
- [x] 7.2 Smoke-test `just dev mock` and `just dev macos`, including HMR, browser reload, authentication, occupied-port failure, and interrupt cleanup.
- [x] 7.3 Smoke-test Android ADB and iOS `iproxy` browser development on available targets without clearing application data or permissions.
- [x] 7.4 Run `just verify` as the final software gate.
