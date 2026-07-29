## Context

The ESP32 test device is intentionally a pure ESP-IDF byte bridge. It exposes the real VESC UART BLE service and forwards exact bytes over a bounded serial protocol, while `/conformance` owns the serial client, VESC framing and request classification, board identity, Refloat fixtures, telemetry phases, and safety policy. Canonical physical conformance currently wraps those pieces in a deterministic `FixtureRunner`, but there is no long-lived manual controller that can run beside `just dev <platform>`.

The new controller must keep servicing BLE writes independently of browser reloads, because the selected native backend—not the browser—owns Bluetooth and may poll telemetry continuously. The control surface is only for local development. It has no production build, adapter, deployment, persistence, or compatibility obligation.

## Goals / Non-Goals

**Goals:**

- Provide `just fixture dev [port]` as one development-only process containing a Svelte control surface and its Node serial coordinator.
- Exercise production native Bluetooth through the unchanged ESP32 firmware while allowing manual power and riding transitions.
- Select a runtime composition of controller VESC 6.02, 6.05, or 6.06 and exact Refloat 1.0.0, 1.1.0, or 1.2.3 from the control surface.
- Present the selected controller either as the directly connected BLE VESC or behind a VESC Express transport at a configurable CAN ID.
- Reuse conformance-owned identities, framing, classification, serial transport, and request allowlist.
- Reproduce the selected upstream versions' actual read-only request and response formats.
- Produce smooth, plausible, reproducible Refloat riding telemetry and immediate idle telemetry on request.
- Survive browser UI hot replacement and reload without duplicating or interrupting serial ownership.
- Keep transport, state, diagnostics, requests, cleanup, and startup bounded and loopback-only.

**Non-Goals:**

- Building, adapting, packaging, deploying, previewing, or serving the fixture UI in production.
- Replacing canonical automated conformance or changing its deterministic scenarios.
- Moving VESC or Refloat interpretation into the ESP32 firmware.
- Supporting motor-control, firmware-write, configuration-write, or other mutating VESC commands.
- Expanding the product's current compatibility decision beyond Refloat 1.2.x; older codecs expose current unsupported or connected-without-telemetry behavior and prepare the simulator for later compatibility work.
- Adding direct-controller telemetry support to the native backends; direct mode exposes identity and Refloat discovery, but the current backends do not request its telemetry.
- Simulating a motor-controller model other than THOR300.
- Sharing product UI components, adding platform-specific product frontend code, or changing the product backend contract.
- Persisting simulator state across controller processes.
- Simulating more than one CAN-connected motor controller in the initial version.

## Decisions

### Use a development-only SvelteKit application inside `/conformance`

The fixture application will use SvelteKit routes for both the browser UI and its server API, launched only through Vite development mode. Its source will live in `/conformance/fixture-dev`, use the existing conformance package and lockfile, and import reusable fixture modules from `/conformance/fixture`. The root `just fixture dev [port]` action will launch it at fixed address `127.0.0.1:11004`, adjacent to the established development ports `11002` and `11003` but distinct from the product frontend and native backend ports.

SvelteKit and Vite development dependencies will align with the versions used by `/frontend`. They replace a custom HTTP server, browser bundler, templating layer, and reload mechanism. There will be no fixture build, preview, adapter, output directory, or deploy script.

The fixture source will follow the product frontend's ownership-first conventions:

- `/conformance/fixture` owns reusable VESC framing, serial transport, THOR300 identity, firmware variants, Refloat codecs, topology strategies, normalized telemetry, simulator behavior, and deterministic physical scenarios. It has no Svelte imports.
- `/conformance/fixture-dev/src/lib/coordinator` owns authoritative process state, serial lifecycle, actions, and coordinator resolution. Server-only modules use `.server.ts`.
- `/conformance/fixture-dev/src/lib/control` owns the browser client, reactive view state, configuration form, and `ControlPanel.svelte`.
- `/conformance/fixture-dev/src/lib/contract.ts` is the small shared state/action boundary between those owners.
- `/conformance/fixture-dev/src/routes` contains only thin SvelteKit page and API adapters.
- Unit tests live beside their owner; cross-owner simulator and native-decoder coverage remains in `/conformance/test`.

The fixture-development tree will include its own ownership README and Svelte/Vite configuration but no second `package.json` or lockfile. It will avoid generic `components`, `services`, `types`, `utils`, and barrel directories.

Alternatives considered:

- A custom Vite middleware plugin would be smaller at runtime but would recreate routing, validation, request parsing, and page-server conventions already supplied by SvelteKit.
- Browser Web Serial would couple operation to browser support and permission prompts, duplicate the Node serial client, and make reliable process cleanup and continuous servicing harder.
- A separate Node daemon plus Svelte app would add an unnecessary process and lifecycle boundary.

### Keep one process-wide coordinator outside request lifetimes

Server routes will resolve a process-wide `FixtureCoordinator` that exclusively owns at most one `FixtureClient`. Its identity will survive browser reload, UI hot replacement, and repeated route-module resolution, so those operations cannot open a second serial connection. Coordinator or other server-side source changes require restarting `just fixture dev`; retaining a stale instance across a changed server class is not supported. All commands and serial events will be serialized through one bounded state machine; server routes will never read the serial stream directly.

The coordinator will register process shutdown handling once. Interruption will best-effort reset the fixture, turn off its diagnostic LED, close the owned port, and then allow Vite to exit within a bounded deadline. No action will close ports or processes it did not open.

### Expose a small authenticated state/action API

The SvelteKit server will expose only port discovery, serial attach/detach, complete state reads, one atomic simulated-device configuration action, and the high-level power/ride actions. Mutation requests will require exact Origin `http://127.0.0.1:11004` and a random per-process launch token supplied to the initial server-rendered page. The development server will bind strictly to `127.0.0.1:11004` and reject unexpected hosts, origins, tokens, methods, bodies, and oversized input.

The coordinator will publish complete snapshots with a monotonically increasing revision. The browser can poll those small snapshots at a fixed bounded interval; a dedicated WebSocket or SSE transport is unnecessary for manual controls. Diagnostics and request counters will be bounded and summarized rather than retaining or logging full serial frames, tokens, Bluetooth identifiers, or telemetry streams.

An optional CLI serial path will be passed to the coordinator as its initial attachment request. Without one, the app will start detached and let the developer explicitly select from enumerated ports. It will not probe or reset every serial device automatically.

### Compose controller variants with selectable transport topology

The control surface will provide one configuration form with independent selectors for controller firmware (`6.02`, `6.05`, or `6.06`), exact Refloat package identity (`1.0.0`, `1.1.0`, or `1.2.3`), and transport topology. Saving submits all configuration fields atomically. Runtime composition combines four independent concepts rather than selecting from an enumerated registry:

- one stable THOR300 controller identity;
- one of three VESC firmware response variants;
- one of three Refloat protocol codecs;
- one of two transport topology strategies.

This composition produces nine controller/package combinations and eighteen effective transport configurations without declaring nine or eighteen board objects. A topology strategy chooses how the composed controller is exposed:

- **Direct VESC:** The BLE peer is the selected motor controller. Its initial firmware response reports the selected VESC identity with hardware type VESC, Refloat requests are not CAN-forwarded, and no intermediary identity is presented.
- **VESC Express:** The BLE peer first reports the existing VESC Express 6.05 transport identity with hardware type Express. CAN discovery returns exactly one configured motor-controller ID, and controller firmware plus every Refloat request and response use the existing VESC CAN-forward envelope for that ID.

Express mode accepts one decimal CAN ID from `0` through `253`, defaulting to `1`. Values `254`, `255`, fractions, signs without digits, alternate numeric encodings, whitespace-padded text, and out-of-range values are rejected rather than coerced. The CAN-ID input remains visible but disabled when direct topology is selected so the retained Express value is explicit.

Topology, retained CAN ID, firmware variant, and Refloat codec are process-local and part of every authoritative snapshot. Draft form edits do not change authoritative state. The complete configuration may be saved only while the fixture is powered off and no BLE peer is connected. A successful save atomically replaces all four fields and resets request assembly, package layout, telemetry generation, topology-scoped request counters, and cached response state before the fixture can advertise again. This prevents intermediate form values or one BLE connection from observing an impossible mixed identity or topology.

The initial default is VESC Express with CAN ID `1`, THOR300 controller VESC 6.06, and Refloat 1.2.3, exactly preserving the canonical physical fixture identity. Firmware variants, Refloat codecs, and topology strategies are data and functions rather than separate simulator implementations. Table-driven tests generate the Cartesian product across both transport paths; immutable golden fixtures exist only for distinct upstream wire formats.

Alternatives considered:

- Three chronologically paired presets would miss cross-version combinations that are useful for compatibility investigation.
- Supporting a list of CAN IDs would enable multi-controller tests but introduces controller-selection and per-node profile semantics beyond the requested single motor controller.
- Allowing saved firmware, Refloat, topology, or CAN-ID changes while connected would be convenient but would not represent a real device and could leave native decoder state inconsistent.

### Preserve upstream version-specific wire formats

Research against the tagged [Refloat source](https://github.com/lukash/refloat), [VESC firmware source](https://github.com/vedderb/bldc), and [archived VESC changelogs](https://github.com/vedderb/vesc_fw_archive) establishes the protocol variants to encode:

- VESC 6.02 and 6.05 answer `COMM_FW_VERSION` with the same field ordering. VESC 6.06 adds a trailing hardware CRC. The native decoders tolerate the optional tail, but the variants will retain the source-version shape.
- Refloat 1.0.0 answers INFO with the legacy version-1 payload (`major * 10 + minor`) even when the client requests a newer INFO format. It exposes legacy realtime command 1 and compact all-data command 10; it does not expose dynamic commands 31 and 32.
- Refloat 1.1.0 supports INFO version 2 with full semantic version fields and introduces dynamic realtime IDs command 32 plus float16 realtime data command 31.
- Refloat 1.2.3 retains INFO v2 and the same core dynamic field catalog, while its realtime mask and optional trailing data add active-alert and firmware-fault metadata.

Every controller firmware variant uses the stable deliberate THOR300 hardware name, VESC hardware type, and canonical fixture identity fields. “Source-faithful” means that selected version bytes, field ordering, optional tails, Refloat requests, and Refloat response formats follow their upstream version; it does not claim that all combinations are captures from physical THOR300 firmware releases. Immutable fixtures will record the source tag/commit, request, response payload, frame, deliberate identity fields, and expected decoded identity for each distinct format. Synthetic dynamic values will be encoded into format-specific response templates rather than treating the package version as a display-only byte.

Upstream Refloat documentation says 1.2 still operates on VESC 6.02 but some automatic battery configuration and per-cell alert behavior require 6.05 or newer. Those configuration and alert-control behaviors are outside this read-only simulator; the composed matrix therefore remains valid without inventing write responses.

### Model desired power separately from observed BLE connection

The authoritative state will distinguish serial attachment, saved topology, retained CAN ID, selected firmware variant, selected Refloat codec, desired power (`off` or `on`), observed BLE transport (`unavailable`, `idle`, `advertising`, or `connected`), and ride mode (`idle` or `riding`).

- Attaching serial performs the bounded `RESET`/`READY` handshake and begins powered off.
- Power on sends `ADVERTISE` and records the desired powered-on state.
- A `DISCONNECTED` event while power remains on causes re-advertising.
- Power off sends `RESET`, which stops advertising and terminates any connection, then confirms readiness.
- Detach performs the same best-effort safe reset before closing serial.
- Unexpected serial loss or invalid bounded serial-protocol data closes the owned client, publishes detached/powered-off/unavailable state, preserves the last saved configuration, and requires explicit reattachment.
- Ride mode affects telemetry generation only and requires an attached powered-on fixture, but it does not require an active BLE connection.
- Starting while already riding and stopping while already idle are inert; repeated start does not reset the seed or sample index.
- The atomic configuration save requires powered-off disconnected state and resets all simulated-device protocol state.

Actions will be idempotent. Invalid transitions will return the current snapshot with an actionable error instead of issuing contradictory serial commands.

### Share request classification but inject an interactive telemetry source

The current `VescSimulator` request path will be refactored to accept the composed THOR300 identity, firmware variant, Refloat codec, transport topology, optional CAN identity, and telemetry source. Canonical conformance will retain its existing scripted source, Express topology at CAN ID `1`, current composed configuration, exact phase sequence, and fail-fast handling of unexpected requests. The development coordinator will compose firmware identity, Refloat INFO, allowed read-only commands, layouts, and telemetry encoders while reusing the frame encoder and fragmentation bounds.

In direct mode, request classification accepts the controller's identity and package requests without a forwarding envelope. In Express mode, it accepts transport identity and CAN discovery at the outer level, returns exactly the configured ID from discovery, and dispatches controller/package requests only when command `34` targets that ID. A forwarded request for any other ID receives no response and records a bounded high-level diagnostic; it cannot fall through to the configured controller.

Interactive mode supplies no response for wrong-CAN-target, unknown, unsafe, or mutating requests and records only a bounded high-level diagnostic; these requests do not terminate its long-lived serial loop. Malformed VESC framing, serial bounds violations, invalid fixture control events, and serial failures are transport faults that detach the fixture. This policy avoids a second VESC classifier, preserves canonical fail-fast conformance, and ensures the development tool cannot accidentally answer motor-control or persistent-configuration commands.

### Generate correlated, seeded Refloat telemetry

The interactive telemetry source will produce one normalized sample model and encode it through the selected Refloat wire format. Refloat 1.0.0 uses its legacy fixed realtime/all-data layout, while 1.1.0 and 1.2.3 use their dynamic float16 layouts with version-specific optional tails. A per-ride seed and sample index will drive bounded waveforms and small deterministic jitter so consecutive riding samples appear smooth and related:

- speed and ERPM move together;
- duty cycle and motor/battery current follow load;
- pack voltage exhibits bounded load sag;
- controller and motor temperatures drift slowly;
- Refloat reports a riding board state with both footpads active.

Idle responses will retain plausible voltage and temperature but report idle state, inactive footpads, and zero motion/current. Starting a ride resets the ride generator; stopping switches the next requested telemetry sample directly to idle. Generated values and half-float encoding will be range-checked before framing.

The current native backends deliberately begin telemetry setup only for Refloat 1.2.x and currently require a CAN controller identity for layout and telemetry polling. The simulator will not bypass either policy: Refloat 1.0.0 and 1.1.0 accurately answer requests they receive, and direct topology answers firmware and Refloat INFO, but the fixture controller does not initiate package requests on the native client's behalf. Older packages therefore expose each backend's current unsupported or connected-without-telemetry outcome, while direct mode exposes discovery without frontend riding telemetry. A follow-up backend change will define explicit failure and expand direct/package compatibility.

### Keep verification software-only

Coordinator state transitions, reconnection, serial exclusivity, action validation, telemetry encoding, request rejection, and cleanup will be tested with a fake `FixtureControl`. Svelte server and component checks will run as part of conformance checks/tests without producing a build artifact. `just verify` remains the complete software gate; using a flashed ESP32 and a real native target remains a manual development or explicit physical BLE checkpoint.

## Risks / Trade-offs

- **Vite SSR invalidation could recreate module state and contend for serial** → Store one guarded coordinator in process-global state, make initialization idempotent, and test repeated server-module resolution.
- **Server source HMR could retain an instance created from obsolete coordinator code** → Guarantee only browser/UI HMR during an attached session and require process restart after coordinator or server changes.
- **An interrupted process could leave the ESP32 advertising briefly** → Register bounded shutdown cleanup and begin every later attachment with `RESET`; USB removal remains inherently outside host control.
- **Serial events and UI actions can race** → Funnel both through one serialized coordinator queue and expose only complete post-transition snapshots.
- **Separate configuration mutations could expose impossible mixed profiles** → Submit and validate the complete configuration in one save action, then replace authoritative state and reset protocol state atomically.
- **Synthetic values could become internally implausible or diverge across wire versions** → Generate one correlated bounded ride model, encode it through version-specific codecs, and test decoded ranges and relationships for every package format.
- **Source-derived formats could drift or be simplified into inaccurate labels** → Retain immutable fixtures with upstream tag/commit provenance and matrix tests for exact INFO, firmware, layout, and telemetry framing.
- **Saved configuration changes could leak cached decoder state across simulated devices** → Permit atomic save only while powered off, recreate topology and codec request state together, and test both paths across the full matrix.
- **Loose numeric parsing could map invalid CAN input to another controller** → Accept only canonical decimal integers in the explicit `0...253` range and return validation errors without mutating state.
- **The interactive source could alter canonical conformance semantics** → Preserve the current scripted source as the default and keep its exact-response/cadence tests unchanged.
- **A local hostile page could attempt fixture actions** → Bind strictly to loopback and require both exact Origin validation and an unguessable launch token for mutations.
- **Adding SvelteKit duplicates dependencies already present in `/frontend`** → Keep them as development dependencies in the existing conformance package, align versions, and avoid a third package or lockfile.

## Migration Plan

No data or production migration is required. Add source-derived immutable fixtures and independently typed controller, firmware, Refloat, and topology variants; introduce topology-aware interactive composition behind the existing deterministic default; then add the development source and root command. Rollback consists of removing the new command, SvelteKit source/dependencies, variants, and interactive source while leaving the unchanged firmware and canonical conformance path intact.

## Open Questions

None. The initial version intentionally uses fixed `127.0.0.1:11004`, state polling, one selected serial port, THOR300 identity, three composed firmware variants, exact Refloat 1.0.0/1.1.0/1.2.3 codecs, direct or VESC Express 6.05 transport, one optional CAN controller at a canonical decimal ID from 0 through 253, atomic powered-off configuration, powered-on-only riding, UI-only HMR, current direct/older-package backend behavior, immediate idle on stop, and no production build path.
