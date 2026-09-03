## Why

The Rust migration has produced tested portable protocol, coordination, and session-SQLite components, but production still pays for two native application implementations plus twenty temporary per-protocol host bridges. Before extracting anything else, the project needs one coarse runtime boundary that composes the proven components and creates a credible deletion path for Apple, Android, and later Windows/Linux hosts.

## What Changes

- Add one safe `application-runtime` Rust component that owns VESC stream framing, connection/application coordination, normalized telemetry, and bounded per-session SQLite operations behind platform-neutral commands, effects, and snapshots.
- Define one versioned coarse host contract for runtime lifecycle, borrowed inputs, owned outputs, ordered effects, bounded queues, cancellation, and payload-free failures; it is intentionally not a per-protocol API.
- Keep Bluetooth execution, monotonic and wall clocks, timers, location acquisition, lifecycle/background policy, permissions, native file presentation, theme, WebViews, and loopback transport in each platform host.
- Add exact composition, persistence, adversarial, and lifecycle evidence without selecting the runtime in Apple or Android yet.
- Record the baseline production LOC, crate/bridge count, build matrix, and explicit deletion inventory that later Apple and Android cutovers must improve.
- Freeze new sideways candidate and protocol-bridge extraction: subsequent migration changes must select this boundary for one host and delete superseded code.
- Non-goals: changing the frontend wire contract, adding Windows/Linux hosts now, moving OS-owned behavior into Rust, generating bindings, selecting either production host in this change, or deleting the temporary bridges before host cutover evidence exists.

## Capabilities

### New Capabilities

- `unified-rust-runtime`: Defines the single portable application-runtime boundary, its ownership, bounded command/effect model, persistence composition, evidence, and deletion economics.

### Modified Capabilities

- `native-core-migration`: Permits a bounded composition step after independently proven candidates, freezes further sideways extraction, and requires later host selection to delete native behavior and temporary protocol bridges through measured gates.

## Impact

- Adds one member under `backends/core/` that directly composes existing safe crates, plus focused fixtures and root verification/audits.
- Changes no frontend request/result/event shape and no production Apple or Android package in this change.
- Establishes the only supported future host-facing Rust seam; Apple and Android cutovers will add one narrow host adapter each and remove their per-protocol adapters, state machines, and bridge build entries.
- Uses the existing locked Rust dependency set, including bundled SQLite and serialization dependencies already present in `Cargo.lock`; no generated API layer or new OS dependency is introduced.
