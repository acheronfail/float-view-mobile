# Float View

Float View has one responsive SvelteKit frontend and a shared Rust backend with native OS hosts. The
frontend knows nothing about Apple or Android: every backend implements the
small hand-written contract in `frontend/src/lib/backend/contract.ts`.

## Repository

- `frontend/` — SvelteKit, TypeScript, Tailwind, the backend contract, client,
  deterministic mock, and UI tests. `npm run build` emits one self-contained
  `build/index.html`.
- `backends/apple/` — one Swift backend shared by the macOS executable and the
  thin iOS shell.
- `backends/android/` — one Android application module containing the backend
  and thin WebView shell.
- `backends/sql/` — the two current disposable-test-stage SQLite schemas.
- `backends/fixtures/` — immutable VESC and Refloat protocol fixtures.
- `conformance/` — the canonical black-box contract suite and programmable VESC
  simulator/orchestrator.
- `tools/vesc-test-device/` — pure ESP-IDF firmware, built with PlatformIO, that
  only bridges a serial line protocol to a fixed BLE GATT service.

Native hosts own Bluetooth, permissions, exports, lifecycle and the session
catalog. The shared Rust runtime owns protocol processing and durable session
work. `backends/core/loopback-server` owns authenticated HTTP/WebSocket transport,
complete state projection and historical responses through the existing C/JNI
binding families. Hosts provide embedded HTML and execute bounded OS capabilities; Rust owns
product storage, application policy, dispatch and complete state projection.

## Commands

```sh
just setup
just dev
just build
just run android|ios|macos [target]
just ipa # Build an IPA for AltStore; see backends/apple/README.md.
just ipa-copy # Build an IPA and copy it to VLC on a connected iOS device.
just test frontend|conformance|apple|android|core|fixture|all
just conformance direct|mock|android|ios|macos
just conformance-physical macos <fixture-port>
just conformance-physical android <fixture-port> <adb-serial>
just conformance-physical ios <fixture-port> <device-id>
just fixture setup|check|ports|build|flash|monitor|dev [port]
just verify
```

The fixture commands require PlatformIO Core (`pio`), but do not require a
separate ESP-IDF installation or a sourced ESP-IDF environment.

For manual native-Bluetooth iteration, flash the ESP32 and run
`just fixture dev [port]` beside `just dev <platform>`. The development-only
SvelteKit fixture controller is available at `http://127.0.0.1:11004`; it can
select direct or VESC Express topology, configure one CAN ID, choose the
THOR300 VESC/Refloat version composition, and switch power and riding state.
See `conformance/fixture-dev/README.md` for compatibility and ownership details.

## Verification

Choose checks for the changed behavior and affected consumers. The policy in
[AGENTS.md](AGENTS.md#verification) applies to implementation, review and OpenSpec
completion. Existing test coverage stays available; running every suite is not
required for each edit.

| Change                                      | Relevant checks                                                                                                                                                                                                                                                                                      |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Documentation or planning only              | Review links and commands, `git diff --check`, and strict OpenSpec validation when specs or plans change. No application tests or builds.                                                                                                                                                            |
| Frontend UI only                            | Focused Vitest tests (`cd frontend && npm test -- <test-file>`), `npm run check`, and relevant formatting/lint checks. `just test frontend` runs the whole frontend suite when useful. Add `npm run build` for bundle/build changes and affected browser tests for interaction or rendering changes. |
| Shared Rust logic or persistence            | From `backends/core`, run `cargo test --locked --offline -p <crate>` for affected crates and consumers, plus relevant formatting and warnings-denied Clippy checks. Use `just conformance direct` for affected public service behavior.                                                              |
| Contract, client/mock or shared integration | Update the contract, client/mock, shared implementation, both host integrations and canonical scenarios together. Run affected frontend and Rust tests, mock/direct conformance, and affected native integration coverage.                                                                           |
| Apple or Android host                       | `just test apple` or `just test android`, or focused Swift/Gradle tests; add the affected native conformance target, startup tests or packaging checks when that boundary changes.                                                                                                                   |
| Conformance or fixture tooling              | Relevant TypeScript checks and focused runner/simulator tests; `just test conformance` for broader harness changes. `just test fixture` covers ESP32 firmware checks.                                                                                                                                |

For browser checks, first build the current frontend with `cd frontend && npm run build`,
then run the affected file from `conformance` with
`node --strip-types --test test/<name>.browser.ts` (with its Playwright browsers installed). Use native simulator/emulator
coverage when native integration is affected, not just because a frontend is
also shipped in a WebView. Package/build changes need the relevant build checks;
a pure component edit does not need clean packages for every architecture.

`just verify` retains the complete software check as an option for broad changes,
uncertain impact or an explicit full-regression request. `just test all` is also
available; neither is a default completion gate.

Physical testing is needed only for affected behavior that software cannot prove:
OS Bluetooth, permissions, lifecycle/background execution, radio timing or new
firmware interoperability. Select the relevant devices, routes and scenarios,
without rerunning unrelated physical matrices. Native visual checks do not
inherently require an ESP32/BLE run. Record missing required hardware and actual
results honestly; ordinary software-only changes need no hardware-skip report.
See [conformance/README.md](conformance/README.md) for checkpoint procedures.

Supported BMS identities are deliberately exact: stock VESC BMS 6.00/type 1
and VESC Express `VBMS32` 6.06/type 2. See `backends/README.md` for the
compatibility and future-version audit policy and `conformance/physical-checkpoints/`
for retained hardware observations.
