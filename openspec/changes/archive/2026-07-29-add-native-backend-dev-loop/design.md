## Context

The production frontend is one inline SvelteKit artifact served from each native loopback backend. The conventional Vite loop currently starts a deterministic mock on port `3001` and proxies `/api` from Vite's default port. Native development instead rebuilds the inline artifact before each launch, so small CSS and component changes pay the full native build/start cost.

The native contract already has the right security boundary for browser development: bootstrap returns a random launch token, and WebSocket upgrade requires both that token and the backend's exact Origin. Android conformance already exposes a fixed device port with ADB forwarding, iOS conformance injects a fixed port and tunnels it with `iproxy`, and the macOS test host exposes readiness while using the production backend stack. The development loop can compose those existing mechanisms without changing `/`, adding CORS, or adding another frontend transport.

The repository has one active frontend routing change. This change must preserve unrelated work in the dirty tree and keep development orchestration centered on the root `justfile`.

## Goals / Non-Goals

**Goals:**

- Make `just dev mock|macos|ios|android [target]` the fast frontend loop.
- Render the frontend in a host browser with Vite HMR while one selected real native process owns Bluetooth, application state, persistence, permissions, lifecycle, and exports.
- Give every supported native backend a stable default loopback port and use the same port on both sides of mobile forwarding.
- Preserve the existing launch-token and exact-Origin checks by sending browser API traffic through Vite.
- Bound startup, readiness, process supervision, failure output, and cleanup without adding a process-runner dependency.
- Keep production HTML embedding, wire envelopes, contract behavior, and canonical conformance assertions unchanged.

**Non-Goals:**

- Redirect a native backend's `/` route to Vite.
- Display the hot-reloaded frontend inside physical-device WebViews.
- Hot-reload Swift or Kotlin.
- Add a Unix-domain transport, custom URL scheme, or native JavaScript bridge.
- Expose Vite on a LAN interface.
- Run multiple selected native development backends in one `just dev` invocation.
- Add database migrations or change persisted data.

## Decisions

### Use one documented port registry

The repository will assign:

| Endpoint                   |    Port |
| -------------------------- | ------: |
| Vite and HMR               | `11002` |
| Deterministic mock backend | `11003` |
| macOS backend              | `11010` |
| iOS backend                | `11011` |
| Android backend            | `11012` |
| Next supported backend     | `11013` |

Each native server will request its platform port by default in normal, development, and conformance launches. A bind failure is terminal and names the unavailable endpoint; there is no ephemeral fallback. Keeping distinct host ports allows installed mobile backends and macOS to coexist without remapping or changing expected origins.

Alternatives considered:

- Ephemeral production ports avoid rare collisions but require readiness discovery and make every development target configure a dynamic proxy.
- One shared native port is simple per device but collides when macOS and forwarded mobile targets coexist on the host and requires host/device Origin translation.
- Unix sockets avoid TCP port allocation but are not reachable through the browser WebSocket API and are not symmetric across ADB and `iproxy`.

### Keep the browser and Vite on the development host

`just dev` starts the selected backend, waits for bootstrap readiness, starts Vite on `127.0.0.1:11002`, waits for Vite readiness, and exposes the local browser URL. Vite remains loopback-only. The native embedded frontend remains unchanged; mobile applications may keep their normal WebView alive while the development browser is an additional disposable client.

For macOS, add a focused browser-development host mode that starts `NSApplication`, the production `MacBackendRuntime`, and the native export presenter without creating a WKWebView. This avoids an extra embedded frontend window while retaining CoreBluetooth, SQLite, permissions, and save-panel behavior.

Alternatives considered:

- Redirecting `/` makes the native WebView depend on Vite and requires a development Origin mode in both native servers.
- Loading Vite over the LAN is needed for a physical iOS WebView and unnecessarily exposes bootstrap credentials and proxy access beyond the development host.

### Proxy both bootstrap and WebSocket traffic through Vite

The selected platform gives Vite a fixed target origin. Bootstrap remains a relative `/api/bootstrap` fetch. During Vite development, the frontend normalizes the bootstrap WebSocket endpoint to the current Vite origin while retaining its path, query, and newly supplied launch token. Vite proxies `/api/ws` to the selected backend and rewrites the upstream WebSocket Origin to that backend's exact origin.

For production builds, endpoint normalization is absent and bootstrap behavior remains unchanged. For mock development, the same Vite-origin socket path continues to target the mock.

This keeps backend authentication unchanged:

```text
browser Origin http://127.0.0.1:11002
        │
        ▼
Vite /api proxy
        │ rewrite WebSocket Origin to selected backend origin
        ▼
backend token + exact Origin validation
```

Alternatives considered:

- Connecting directly to the absolute backend WebSocket URL fails exact-Origin validation.
- Relaxing backend Origin checks or adding CORS weakens the production boundary.
- Rewriting bootstrap JSON inside the proxy is more fragile than a small development-only endpoint normalization in the existing bootstrap client.

### Forward identical mobile port numbers

Android development and conformance use `adb forward tcp:11012 tcp:11012`. iOS development and conformance use `iproxy 11011:11011`. Because host and device endpoints match, Vite's selected target origin is also the Origin expected by the backend. Target selection remains explicit when multiple Android or iOS devices are connected.

The existing signed iOS and Android Debug application flows remain responsible for build, install, launch, permissions, and foreground lifetime. The host browser does not replace native lifecycle ownership.

### Keep orchestration dependency-free and bounded

The root `justfile` remains the public interface and owns small platform cases, readiness deadlines, process identifiers, traps, and cleanup. Existing bounded ADB, `devicectl`, and `iproxy` argument builders and lifecycle patterns should be reused or extracted rather than copied with divergent behavior. Vite selects mock versus native proxy target through process environment supplied by `just`.

On interruption or child failure, orchestration stops Vite, removes only the forwarding rule it created, terminates any proxy it created, and reports the first failure. It must not reset app storage, revoke permissions, or uninstall applications during ordinary development.

### Update conformance ports without changing acceptance semantics

Canonical scenarios and contract clients remain unchanged. Only target endpoint constants, launch configuration, and forwarding rules move from `8765/8766` to `11010/11011/11012`. Mock conformance uses the new mock port where it starts the standalone mock. `just verify` remains the complete software gate.

## Risks / Trade-offs

- **[Risk] Another process already owns a fixed port.** → Fail before opening the browser with the platform and port in the diagnostic; never connect to or silently replace an unknown listener.
- **[Risk] Vite Origin rewriting can conceal an authentication regression.** → Limit rewriting to the configured loopback proxy, retain token validation, and keep canonical direct-backend conformance authentication tests.
- **[Risk] Two frontend clients can act on one mobile backend.** → Treat both as disposable clients of the same authoritative backend and avoid interacting with the embedded device UI during browser development.
- **[Risk] Mobile setup duplicates fragile conformance plumbing.** → Reuse bounded command builders and cleanup helpers while keeping contract assertions exclusively in conformance.
- **[Risk] A child process or USB tunnel exits unnoticed.** → Supervise every started process and terminate the complete development session on the first child failure.
- **[Risk] Development-only WebSocket normalization leaks into production.** → Gate it on Vite development compilation and test production bootstrap URL preservation.

## Migration Plan

1. Introduce and test the port registry, update native defaults, and update conformance forwarding together so no target remains on the superseded ports.
2. Make Vite's mock/native proxy selection explicit and add development-only WebSocket endpoint normalization.
3. Add bounded macOS browser-host orchestration, then Android ADB and iOS `iproxy` development targets.
4. Remove the old port constants and run `just verify`.

Rollback is a single source revert because there is no persisted-data or wire-format migration.

## Open Questions

None. The browser is the development frontend for every selected backend, fixed ports are intentional defaults, and native WebView hot reload is out of scope.
