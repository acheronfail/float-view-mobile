# Float View agent guide

Float View has one SvelteKit frontend and a shared Rust backend with native
Apple and Android hosts. Keep the repository small and delete superseded paths;
there is no generated API layer or platform-specific frontend code.

## When to use OpenSpec

Use OpenSpec for new core capabilities or material/risky changes to functionality,
public contracts, persistence, architecture or cross-platform policy. Judge impact,
not diff size; reuse a relevant active change. Low-risk documentation, tests,
formatting, UI polish, behavior-preserving refactors and fixes restoring specified
behavior may proceed directly. If scope or risk grows, create/update a change
before expanding implementation. Explicit requests to use OpenSpec take precedence.

## Completing work

Implementation includes relevant checks, fixes for failures caused by the change,
and task/evidence updates. Sync and archive completed OpenSpec changes unless
asked to leave them open; planning-only requests and unfinished drafts do not
authorize implementation or archival. Keep changes open while required checks
remain, reporting the specific checkpoint; checked task boxes alone are not proof.
Routine decisions and artifact updates within scope need no repeated confirmation.
Ask about missing information or material scope decisions that prevent progress.
Completion never authorizes commits, pushes, releases or other online changes.
See [OpenSpec commands and validation](docs/development.md#openspec).

## Documentation

Keep READMEs to ownership, non-obvious constraints and commands. Observable
behavior belongs in OpenSpec; implementation mechanics/constants belong with
source and tests. Maintain each policy/procedure once and link to it. Do not
create documents merely to relocate duplicated feature descriptions or limits.

## Supported data compatibility policy

Preserve owned-storage baseline 100 and subsequent supported data regardless of
marketing version, channel or tag availability. Only recognized pre-baseline
markers/layouts authorize bounded, restartable deletion of validated app-owned
development data. Unknown/newer/malformed markers, nonempty roots without markers,
corruption and operational failures preserve data and fail actionably. Exported
user files are outside resets.

Later incompatible storage changes require an incremented marker and an explicit,
crash-safe tested upgrade. Add no general migration framework without a published
transition. Keep archive numbering independent, supported baseline imports and
frozen fixtures. Do not retain readers, aliases or fallbacks solely to recover
pre-baseline data. This does not remove supported VESC/Refloat wire families. See
[storage-baseline.md](docs/storage-baseline.md) for the inventory and upgrade rules.

## Researching definitive upstream source

Prefer offline upstream sources in `vendor/`: `bldc`, `refloat`, `vesc_tool`,
`vesc_bms_fw`, `vesc_express` and `float-view`.

## Contract and ownership

[contract.ts](frontend/src/lib/backend/contract.ts) is the wire source of truth:
authenticated request/result/error envelopes and complete, monotonic
`state.changed` snapshots. Contract changes update client/mock, shared server,
both host integrations and canonical conformance together. Follow the
[frontend ownership guide](frontend/src/lib/README.md).

The shared Rust service owns product CRUD, catalog/archive policy, protocol,
persistence, reconnect decisions, waiters and public dispatch/snapshots. Hosts own
OS Bluetooth, location, lifecycle/background execution, permissions, file
presentation, theme and UI integration through the existing C/JNI bindings.
Adapters exchange only closed OS facts, effects and completions.
WebViews and transport workers reuse the process owner; read-only workers use
bounded independent readers without creating another runtime. Bluetooth/protocol
code emits typed events without knowing WebSockets or views.

Bind only to loopback, validate origin and launch token, and bound messages,
queues, collections, retries and files. Never log addresses, tokens or full
telemetry by default. See [backend ownership](backends/README.md) for entry points.

## Frontend interaction

Use the same controls on desktop and touch. Prefer Pointer Events with primary
pointer release/cancellation and a guarded keyboard/assistive click path;
frequently updating WebViews must not rely solely on synthesized clicks or
activate twice. Keep native button semantics, `type="button"` outside submissions
and appropriate `touch-action`. Test primary, non-primary and keyboard activation.

## Retained paths

App/tool roots are `frontend/`, `backends/apple/`, `backends/android/`,
`backends/core/`, `backends/sql/`, `backends/fixtures/`, `conformance/`,
`tools/vesc-test-device/` and `openspec/`. The ESP32 is pure ESP-IDF byte-bridge
firmware; scenario logic, serial orchestration and VESC interpretation belong
only in `conformance/`.

## Verification

Start with focused checks for changed behavior and affected consumers; broaden
only when impact or failures warrant it. Use the
[verification matrix](docs/development.md#verification) for commands. Documentation
needs link/command/format validation, not app builds. Frontend-only work needs
frontend checks and relevant browser/mock coverage; Rust, host and contract work
needs affected crate/consumer, native-boundary and conformance checks respectively.
`just verify` is optional comprehensive coverage, never a routine completion gate.

On macOS, run Playwright `conformance/test/*.browser.ts` directly outside the
Codex sandbox (`sandbox_permissions: "require_escalated"`); sandboxed Chromium
and WebKit cannot launch reliably. Build the frontend first inside the sandbox.
Follow the development guide's serialization rules for shared build files/ports.

Require physical checks only for affected behavior software cannot establish:
OS Bluetooth, permissions, background execution, radio timing or new firmware
interoperability. Select relevant platforms/routes/scenarios and preserve fixture
safety and verified restoration. UI-only work needs no physical BLE checkpoint.
Report actual results and missing required coverage; software checks are not
physical evidence. Ordinary software-only edits need no hardware-skip report.
Historical full-suite/all-platform migration gates apply only to those changes.

Keep detailed hardware history, device/OS/firmware inventories and raw reports
in ignored `.local/physical-testing/` (or `.local/ios-store/readiness/` for
distribution). Public docs retain reusable procedures, brief build/source-linked
outcomes and material limits. Preserve detailed evidence locally before shortening
public records. Never force-add local evidence or publish private identifiers/routes.
