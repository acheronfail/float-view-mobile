## Why

Frontend iteration currently uses only the deterministic mock or requires rebuilding and embedding the production HTML before launching a native host. A browser-based native development loop should retain Vite hot module replacement while exercising the real macOS, iOS, or Android Bluetooth, lifecycle, persistence, permission, and export paths.

## What Changes

- Add `just dev <platform> [target]` orchestration for the mock, macOS, iOS, and Android backends, with the frontend rendered by the host development browser.
- Run Vite and HMR on loopback port `11002`, use mock backend port `11003`, and assign stable native loopback ports `11010` for macOS, `11011` for iOS, and `11012` for Android. Reserve subsequent ports from `11013` for new supported backends.
- Configure Vite to proxy bootstrap and WebSocket traffic to the selected native backend while preserving the backend's existing launch-token authentication and exact Origin validation.
- Reuse ADB forwarding for Android and `iproxy` for iOS, forwarding the same platform-specific port on the development host and device.
- Make each native backend use its assigned fixed port by default and fail clearly when that address is unavailable rather than silently selecting another port.
- Update conformance targeting to use the same platform port registry and same-number host/device forwarding.
- Keep `npm run dev` and `just dev mock` as deterministic mock-backed hot-reload development.
- Remove the superseded `3001`, `5173`, `8765`, and `8766` development and conformance port defaults.
- Do not redirect native `/` routes, introduce Unix-domain sockets or a native JavaScript bridge, hot-reload native backend code, expose Vite beyond loopback, or add platform branches to the frontend.

## Capabilities

### New Capabilities

- `native-development-loop`: Browser-based Vite development against one selected real native backend, including process orchestration, forwarding, proxying, readiness, and cleanup.

### Modified Capabilities

- `shared-frontend-artifact`: Extend the conventional hot-reload loop from the deterministic mock to selected native backends while retaining one platform-neutral frontend.
- `platform-hosts`: Assign stable loopback ports to supported native hosts and define the debug launch seams needed by browser development.
- `backend-conformance`: Use the shared platform port registry and same-number device forwarding without changing black-box acceptance behavior.

## Impact

- Root `justfile` development, run, conformance, and verification commands.
- Vite configuration, mock startup, and development-only WebSocket endpoint normalization.
- macOS, iOS, and Android loopback-server construction and startup diagnostics.
- Android ADB and iOS `iproxy` target orchestration in development and conformance.
- Tests for port selection, proxy configuration, process cleanup, and unchanged authentication behavior.
- No wire-envelope, persistence-schema, Bluetooth, VESC, or production frontend-artifact change; no new runtime dependency is intended.
