# Contributing to Float View

Float View has one responsive SvelteKit frontend and a shared Rust backend with native OS hosts. The
frontend knows nothing about Apple or Android: every backend implements the
small hand-written contract in `frontend/src/lib/backend/contract.ts`.

## Repository

- `frontend/` — SvelteKit, TypeScript, Tailwind, the backend contract, client,
  deterministic mock, and UI tests. `npm run build` emits one self-contained
  `build/index.html`.
- `backends/apple/` — native macOS and iOS hosts for the shared Rust backend.
- `backends/core/` — shared Rust application runtime and loopback server.
- `backends/android/` — one Android application module containing the backend
  and thin WebView shell.
- `backends/sql/` — the canonical supported SQLite schemas.
- `backends/fixtures/` — immutable VESC and Refloat protocol fixtures.
- `conformance/` — the canonical black-box contract suite and programmable VESC
  simulator/orchestrator.
- `tools/vesc-test-device/` — pure ESP-IDF firmware, built with PlatformIO, that
  only bridges a serial line protocol to a fixed BLE GATT service.

Native hosts own Bluetooth, permissions, native file presentation and lifecycle.
The shared Rust service owns the session catalog, exports, protocol processing and
durable session work. `backends/core/loopback-server` owns authenticated HTTP/WebSocket transport,
complete state projection and historical responses through the existing C/JNI
binding families. Hosts provide embedded HTML and execute bounded OS capabilities; Rust owns
product storage, application policy, dispatch and complete state projection.

Project-owned contributions use AGPL-3.0-or-later. Preserve third-party licences
and attribution; see [licensing and notice maintenance](docs/licensing.md).

## Commands

```sh
just setup
just dev
just build
just run android|ios|macos [target]
just release patch|minor|major # Tag and push a deliberate stable release.
just release preview [ref] # Preview next-patch release notes locally (default: HEAD).
just ipa # Build an IPA for AltStore; see backends/apple/README.md.
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

Run frontend builds, Vitest and SvelteKit checks sequentially: they share generated
SvelteKit files. The build validates that the embedded app and startup IDs agree.
When running multiple browser files, use `--test-concurrency=1` because their mock
servers share a port.

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

## App releases

`just release preview` prints the next patch release's Markdown notes from the latest
local stable ancestor tag through `HEAD`. Pass a ref, such as `just release preview master`,
to inspect another commit. It uses the same note generator as publication, works offline
and on dirty or unpushed branches, and does not create tags or publish anything.
Only committed changes appear; fetch tags first if the local release history is stale.

`just release patch` fetches tags and creates/pushes the next annotated stable
version on a clean `master` matching `origin/master`. Use `minor` or `major` to
reset lower version components. The greatest numeric stable ancestor is the
base, so `v0.1.49` advances to `v0.1.50`, `v0.2.0` or `v1.0.0` respectively.
You can also push an explicit `vX.Y.Z` tag on master history. Published stable
releases and their assets are retained on reruns; a failed draft can be resumed.
If a helper push fails, its local tag remains for inspection and an explicit
`git push origin <tag>` retry. Tagging does not modify app source or package manifests.

Master pushes and manual master builds produce Nightly
prereleases targeting `v1.0.0-nightly.<count>.g<hash>` before the first stable
v1.0.0 ancestor. The count includes all reachable repository commits, so deleting
v0.x tags cannot renumber it. After v1.0.0, use the next patch and commit count
since the greatest stable ancestor, plus the source hash. Nightlies update one rolling
Nightly prerelease, attaching it to the full SemVer version tag so GitHub sorts it
above older stable versions. After successful nightly publication, superseded nightly
version tags are deleted; the current Nightly tag, release and asset set are retained.
Stable and unrelated tags are preserved. Nightly never replaces the stable Latest release. Unchanged successful
nightlies and reruns of older commits are skipped. Failed builds remain retryable.

The retained Build apps workflow counter plus its fixed offset of 60 supplies an independent numeric native
build identifier across both channels. The offset preserves the sequence from the former `ipa.yml` workflow.
The same preparation output feeds the IPA, APK, Mac ZIP and optional iOS store
job. Build records retain source, run ID, attempt and CI URL; GitHub release notes
link the number to that run. Build `N` maps to workflow run number `N - 60`.
The allocator rejects other workflow paths; keep `.github/workflows/build-apps.yml`
and its counter/offset through version and tag cleanup. Changing its display name
is safe; moving its path requires an explicit continuity migration. Reruns keep
the original number, while corrected submissions require a fresh workflow run.
Canceled, skipped or failed runs may leave gaps; numbers are never recycled.
The protected iOS delivery path queries Apple before considering an upload retry.
Release notes list non-merge commit titles, grouped by `app:`, `apple:`, `android:`,
`windows:` and `linux:` under App, Apple, Android, Windows and Linux.
`repo:` commits are excluded because they do not describe changes to the released apps.
Windows and Linux areas are reserved for the planned hosts; sections appear only when they have entries.
Unprefixed or unrecognized
titles appear under Other changes. Entries link to their commits and include a full
changelog link. Stable notes compare against the previous stable ancestor tag;
Nightly compares against the latest stable ancestor. A first release includes all
reachable non-merge commits except `repo:` entries. Ranges with no included changes are stated explicitly.
Nightly descriptions begin with a UTC date and time refreshed on each successful publication.
Hidden source/version comments support nightly skip and asset verification.
Stable tags select Release configuration, while nightlies use Debug for mobile.
Current downloads remain AltStore IPAs, signed APKs and notarized Developer ID
Mac ZIPs. Local iOS Release archive/export preparation is documented in the
[Apple host guide](backends/apple/README.md#ios-app-store-preparation); it neither
uploads to a store nor changes the GitHub packages. Android App Bundles remain
planned work. The optional `ios-store` manual
input builds an App Store-signed artifact in its own protected environment, using
the same run counter; it does not upload or gate the three-platform GitHub release.
The separate manual `ios-delivery=check` action verifies Apple authentication,
the exact app and consumed number without uploading. `ios-delivery=upload`
additionally requires a protected environment's explicit readiness setting and
owner review; it uploads once and retains a processing receipt. Neither action
invites testers or submits review. See the Apple host guide for gates and retries.

Run the focused release-tooling checks with
`node --strip-types --test .github/scripts/*.test.ts` and
`python3 -B .github/scripts/ios-store.test.py` and
`python3 -B .github/scripts/ios-upload.test.py`.
