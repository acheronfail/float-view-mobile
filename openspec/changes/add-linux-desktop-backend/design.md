# Design

## Context

See [proposal.md](proposal.md) for motivation and scope. `backend-service::Process` already owns one runtime and product store; `loopback-server::Backend::with_service` and `Server::with_backend` accept that owner directly. `conformance/rust/shared-service.rs` demonstrates safe composition, but is a test relay, not a production Linux adapter.

The closed OS exchange in `backend-service/src/os.rs` already covers Bluetooth, timers, permission observations, file presentation, theme and lifecycle. Private platform enums in `backend-service/src/permissions.rs` and `loopback-server/src/protocol.rs` currently enumerate Apple/Android only. Shared BLE policy owns profile validation, segmentation, write ordering and reconnect decisions. The core workspace has 28 members and its ownership audit counts only Rust core, Apple and Android sources.

## Goals / Non-Goals

**Goals:** Add Linux OS execution around the production shared owner, preserve canonical frontend behavior, and expose deterministic seams that exercise the selected Linux host. Keep blocking desktop and D-Bus work outside service locks.

**Non-Goals:** See the proposal. In particular, a browser-development mode is a supervised developer seam requiring a desktop session, not a supported headless deployment. No additional API, protocol codec, product store, application FFI family or frontend fork is introduced.

## Decisions

### 1. One Rust desktop host outside the core workspace

Add a single package/workspace at `backends/linux/` with a committed lockfile and path dependencies on the existing core crates. Keep the core's 28-member ceiling and its two application binding families. The separate host workspace prevents GTK dependencies entering Apple/Android workspace resolution and avoids classifying an OS adapter as portable core code. Its trade-off is a second Rust lockfile; check compatible shared dependency resolution and build both affected consumer graphs whenever a shared dependency changes.

Use `Process::open`, its existing service handle, and the shared server directly. Use the existing typed fact/effect exchange with lifetime, sequence, monotonic/epoch time, connection generation and operation identity. Do not route through the Apple C ABI or copy the test relay's stdin protocol. GTK owns the UI thread; a bounded executor owns OS operations. A single ordered ingress assigns fact sequence numbers. Drain effects under short locks, release the locks, then perform OS work. Bound workers, in-flight D-Bus operations, notifications, timers and completion queues; reserve cleanup capacity and terminate a saturated BLE stream rather than silently drop bytes. Use the shared existing bounds where applicable and document/test every Linux-specific bound.

### 2. GTK4 and WebKitGTK desktop shell

Use `gtk4` and `webkit6` bindings, with GLib/GIO only where directly needed. These replace hand-written GTK/WebKit FFI, event-loop integration, native dialogs, and window/theme plumbing. WebKitGTK serves the existing embedded HTML from the loopback server; it does not introduce a JavaScript-to-Rust product bridge. The primary alternatives were Tauri/Wry (additional shell/configuration or toolkit integration without a need for their IPC) and an external browser (does not satisfy the requested desktop app).

Build the same frontend artifact, copy it into ignored Linux build inputs, and embed it in the executable; fail if missing. Use system GTK4/WebKitGTK libraries and pin a mutually compatible crate set within the repository Rust toolchain. Initial acceptance is a source-built x86_64 Linux desktop with a system/session bus and BlueZ. Record the tested distribution and actual dependency versions; other architectures and distribution packages are not acceptance claims.

Restrict WebView top-level navigation to the app origin, deny unsolicited popups and remote document navigation, and handle existing external-link behavior through a narrow OS URL opener. Preserve the shared token/Origin checks and loopback-only bind. Native startup errors remain visible if the server cannot start. Use the existing light/dark canvas values and shared theme effects; do not add a splash delay. The user can minimize the window without changing service lifetime. Window close or explicit quit performs bounded shutdown; do not silently leave a background process running. WebView crash/recreation alone retains the owner and current recording.

### 3. BlueZ adapter using BlueR

Use `bluer` for adapter/device enumeration and remote GATT operations. It replaces custom D-Bus proxy definitions and signal plumbing; do not add `btleplug` or another BLE abstraction alongside it. [BlueR upstream](https://github.com/bluez/bluer) documents remote GATT, discovery and adapter change events. The shell choices are supported by [GTK Rust bindings](https://gtk-rs.org/gtk4-rs/) and [WebKitGTK Rust bindings](https://world.pages.gitlab.gnome.org/rust/webkit6-rs/stable/latest/docs/webkit6/index.html).

Select one powered adapter deterministically by object path at startup; if none is powered, retain a deterministic available adapter for status observation. Do not switch an active connection to another adapter implicitly. Report adapter removal, daemon loss, denied access and power changes as OS facts, invalidate the affected connection, and let shared policy decide subsequent reconnect. Re-enumerate after service recovery without an unbounded busy loop. Scope discovery sessions and cleanup to this application; do not change global discoverability, unpair devices, install policy rules or require root.

Pass advertised capabilities and discovered GATT properties to the shared BLE policy. Resolve the existing UART profile, establish notifications before ready, and execute only shared ordered writes. Start with a conservative 20-byte safe payload; increase it only with proven supported capability data, never above the shared 244-byte ceiling. Map response writes to their actual completion and no-response writes to local submission acceptance, never remote acknowledgement. Confirm the selected BlueR API's flow-control semantics in focused integration work before claiming no-response support. If a mode cannot provide bounded safe progress, report that capability unavailable rather than inventing completion or adding a second write queue. Keep per-device OS identity in the existing private association model and out of normal diagnostics. Cover reconnect after process restart and adapters disappearing/reappearing.

### 4. Honest permissions and interruption behavior

Add `Linux` to platform enums and matching shared permission projection. Bootstrap serializes the server platform publicly, so add `linux` to frontend bootstrap typing and desktop Save labeling; verify existing Apple/Android bootstrap behavior remains unchanged. This is the only planned public metadata extension. Do not renumber existing host identifiers or add Linux C/JNI exports. Bluetooth access reflects BlueZ/D-Bus availability and authorization; no adapter, powered-off adapter, absent daemon and permission denial are distinct facts. Report only actionable recovery buttons: the initial host does not promise a universal Linux settings panel or permission prompt. Use existing generic guidance and null actions where no supported recovery exists, with bounded native diagnostics/documentation explaining prerequisites. Restore availability when external OS state changes.

Live location acquisition is unavailable, with no request/settings action and no fabricated coordinates or background authorization. If recording is disabled by user preference, preserve the canonical disabled state; enabling it reveals unavailable capture. Shared history/import/review still handles existing location samples. Bluetooth readiness and ride recording must not depend on an unavailable optional location provider.

Observe desktop suspend/resume through the system bus using GIO rather than a new general D-Bus dependency solely for lifecycle. Suspend invalidates active OS operations and reports link interruption; resume refreshes clocks/radio/permissions and allows shared reconnect policy to act. Keep a suspend-aware elapsed-time source, bound timer delivery, and never deliver a burst of expired writes or invent samples for sleep. If suspend notification is missed, transport failure and elapsed-gap handling still reject old-generation work. Do not inhibit system suspend or promise capture during sleep. Close/quit cancels OS work, completes/relinquishes presentation leases, stops the listener and shuts down the process within documented deadlines. OS shutdown can interrupt this sequence; existing durable recovery handles the next launch.

### 5. Existing storage and file ownership

Use a private user-owned data root at `$XDG_DATA_HOME/float-view`, falling back to `$HOME/.local/share/float-view` when the XDG value is absent/invalid. Require an absolute usable root, create it with user-only permissions, and retain the shared exclusive root lock. Never recover/delete another process's storage or kill a port owner. A second launch either activates the existing desktop instance through normal application registration or fails clearly without opening a second service owner.

Use shared product/session storage and the current format marker unchanged. File dialogs present/copy only the service-issued artifact and retain its lease through completion or cancellation. Use GTK/GIO native presentation, including desktop portal support provided by the toolkit when available; don't add a second archive parser. Import retains the current frontend upload/shared validation route and WebView file chooser integration. Reject unsupported destinations or copy failures cleanly and release leases exactly once. Ordinary I/O failure never triggers a storage reset. No migration or historical reader is needed.

### 6. Tooling and acceptance are Linux-aware

Add `just build-linux`, `just run linux`, `just test linux`, and `just dev linux` without making Linux commands build Apple/Android. Keep frontend artifact construction shared. Extend the current TypeScript dev launcher and Vite target routing for port `11013`; the Linux development process keeps native file presentation and a desktop event loop while omitting the embedded WebView. Keep readiness and process cleanup bounded and tokens confined to the existing protected orchestration channel.

Add a minimal Linux target in `conformance/` and reuse canonical scenarios and software peripheral logic. Test mode requires both the `software-bluetooth` Cargo feature and explicit launch argument, uses isolated data and cannot accidentally replace production Bluetooth. The optional reqwest dependency replaces handwritten HTTP/authenticated fixture-relay transport, and optional base64 encodes the existing test relay byte envelope; neither enters the normal desktop build. Unit tests validate adapter translation using controlled OS events; software target coverage validates Linux composition and public behavior; physical Linux fixture coverage validates actual BlueZ/GATT behavior. A direct Rust test host is not evidence that the Linux adapter works.

Extend ownership audits to count Linux production source, reject duplicated product/protocol policy, and preserve the two-family/core-member constraints. Establish and document a small explicit Linux source budget after the adapter slice is measured; do not hide Linux source from the audit or raise the existing portable budget to accommodate a GUI. Update AGENTS, backend READMEs, root verification commands and OpenSpec context to recognize the new host while preserving historical evidence.

## Risks / Trade-offs

- System toolkit versions vary → pin/test one supported source-build baseline and provide exact prerequisites; do not claim universal binaries.
- BlueZ write completion differs from mobile APIs → conservatively report capabilities, test both available write modes and stale results, and require the Linux physical checkpoint.
- Bluetooth addresses/paths can change → use existing private association and rediscovery semantics; require re-association when identity cannot safely be resolved, never guess by display name.
- GTK and asynchronous Rust runtimes can deadlock → keep UI work and OS waits outside the service lock and test shutdown under load.
- Suspend or daemon restart can lose transport events → invalidate operations, refresh OS state and apply shared reconnect policy without replaying uncertain mutations.
- No live location or unattended service in v1 → expose the limitation through canonical capability state and desktop documentation.

## Migration Plan

Commit this planning set before implementation. Build successive verifiable slices: private platform integration, desktop composition, Bluetooth execution, desktop effects, then tooling/conformance. No user data conversion is needed and existing hosts keep their binding paths. Rollback removes the Linux host and its platform/tooling registrations without touching Apple/Android data or rewriting archives. After affected software checks and the Linux physical/desktop checkpoints pass, sync delta specs and archive together. Missing required hardware evidence remains an incomplete task, not a claimed pass.

## Open Questions

- Which available Linux machine, Bluetooth adapter and fixture will supply acceptance evidence? Record exact versions and available write modes when running the checkpoint; this does not change the architecture or required scenarios.
