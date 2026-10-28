# Shared Rust backend

`backend-service::Process` owns one application runtime, product store and
maintenance executor. Both native hosts select this constructor through the
existing C/JNI binding families. Listeners borrow the same service; stopping a
listener or replacing a WebView does not stop recording or maintenance.

`product-store` owns catalog startup, settings, board identity and profiles,
Tunes, schema caches, session lifecycle/recovery, archive validation and CSV.
`application-runtime` owns protocol coordination and the sole session writer.
The product store owns the pinned battery cell catalog and validates saved profile
choices. `backend-service::battery_estimator` calculates approximate state of charge;
the process-owned connection tracks accepted, persistence-gated samples. Signed
trapezoidal current integration uses optional full-pack capacity in Ah (parallel,
not series, multiplies cell Ah). The original charge axis is remapped using the
zero-load reserve at the configured empty voltage. Without capacity or a valid
current pair, voltage smoothing uses a 30-second EMA capped at 0.1 displayed
percentage points per second. Integrated charge is not subject to that rate cap.
Rest correction requires 30 seconds within 0.1 A per parallel cell and 0.01 V per
cell, uses a 120-second EMA with the same display cap, and clears provisional only
within two displayed percentage points. Charging does not qualify as rest.
Gaps over five seconds, reconnects and invalid telemetry break current pairs and
settling without invented consumption. Effective configuration/controller changes
reseed; identity/name-only edits and WebView recreation preserve charge. Process
restart has no durable estimator checkpoint. Method/provisional metadata describe
this behavior without claiming measured accuracy. Raw voltage/alerts and stored
telemetry remain responsive and unchanged. The service-owned named pack catalog,
selection versus import semantics and nominal provenance are documented in
[product-store/BATTERY_PRESETS.md](product-store/BATTERY_PRESETS.md).
Read-only history, archive and maintenance workers use bounded independent
readers and service-issued leases. An exclusive root lock prevents a second
process owner from recovering or deleting resources still in use.

`loopback-server` owns authenticated HTTP/WebSocket routes, public dispatch and
projection, bounded historical reads and compression. It calls the shared
service directly. There is no native domain-operation exchange, native catalog,
second runtime, generated API or per-protocol binding.

Native hosts supply the private root, HTML and closed OS facts; they execute
Bluetooth, timer, location, permission, lifecycle, theme and file-presentation
effects. The OS exchange validates lifetime, sequence, generations and byte
bounds. Queues, timers, workers, files and correlated requests have explicit
limits, with reserved cleanup capacity. Native presentation retains an export
lease until dismissal or completion of a scoped destination copy.

`application-runtime-host` is the safe process registry and bounded result
buffer. Exactly two unsafe bindings sit above it:

- `application-runtime-apple-ffi`: caller-owned C buffers for iOS/macOS;
- `application-runtime-android-jni`: call-local JVM arrays, without retained JNI
  references or callbacks.

The build scripts audit exact exported symbols and use the locked dependency
set. Android packages one library for each supported ABI. Generated output stays
under `target/`. The workspace contains at most 28 members; protocol codecs and
state machines remain downward dependencies, not separate host runtimes.

`audit-ownership.sh` checks production source below 49,922 lines, two binding
families, the workspace ceiling and absence of native portable implementations.
Use focused Cargo tests and lint checks for changed crates and affected consumers.
Run direct Rust conformance for affected public service behavior, adding native
integration or packaging checks when host boundaries change. The direct host uses
synthetic OS facts and the same canonical product scenarios as native hosts.
`just verify` is optional comprehensive coverage. Follow the root
[verification policy](../../AGENTS.md#verification) for physical checkpoints;
shared logic or persistence edits alone do not require devices or an ESP32.

The v1.0.0 candidate uses owned-storage baseline 100. Recognised earlier shared
markers reset only validated owned development files, once. Baseline data survives
ordinary launches; unknown/newer/malformed/missing-on-nonempty markers, corruption
and I/O errors never trigger erasure. Later incompatible versions require explicit
supported-data upgrades. Portable archives remain independently numbered. See
[the storage inventory](../../docs/storage-baseline.md) and AGENTS.md for policy;
do not retain readers/aliases for earlier disposable development data.

## Process-memory UI checkpoint

`backend-service` retains one typed version-1 checkpoint independently of
listener lifetime. Authenticated `uiState.get` returns it or null;
`uiState.set({checkpoint})` atomically replaces it, or clears it when null.
The shared server exposes both methods through existing C/JNI consumer paths.
Records are at most 4,096 encoded UTF-8 bytes, target strings at most 512 bytes,
and route/surface unions reject unknown fields and incompatible combinations.

The checkpoint describes a route, optional Live mode/device target and one
ordinary surface. The frontend owns current-target eligibility and rendering.
Checkpoint changes produce no state events, revision increments, product
writes, BLE effects or exports. A new process starts empty; no schema change,
migration, browser store or background task is involved.

## Ride announcements and load ownership

`application-runtime::load` is the only utilisation/winner calculation. It feeds
recorded motor/battery fractions, sample-scoped live load, native presentation
and announcement policy. Frontend consumers format these results; session review
keeps raw duty and captured current-load fields without present-day limits.

`backend-service::announcements` owns announcement settings, a shared monotonic
cooldown for ordinary rules, independent battery cooldowns and one ten-second
speech operation.
Manual tests can repeat as soon as playback finishes and still advance the
automatic cooldown to keep automatic speech from following a test immediately. Typed native effects
carry optional closed left/right/both footpad release, the highest load source,
optional speed, speech gain and admission expiry, in that spoken order.
Footpad alerts default off with a one-second delay, configurable from 1–5 whole
seconds. Two monotonic timestamps qualify released sides independently only
with fresh known contact and finite speed magnitude >5 and ≤200 km/h, in saved
or guest mode. `left` means left pressed/right released. The speech timer stays
armed while a side accumulates its delay. Contact recovery clears only that side;
invalid evidence, speed gate loss, a gap ≥2 seconds (checked before replacing the
previous observation), controller/connection changes and footpad settings edits
clear both. Resets retain cooldown. WebView replacement preserves service-owned
timing; process restart discards it. Recovery lets admitted speech finish;
invalid context and rule edits cancel it. No release samples or warnings queue.
Missing/stale telemetry cancels automatic speech; unavailable audio drops an
attempt without a backlog. Test waiters belong to their requesting client.

Authoritative flywheel mode suppresses all automatic footpad, speed, load and
battery speech and cancels an automatic phrase already playing. It clears transient
qualification while preserving preferences and process-local cooldowns. Missing or
unknown mode cannot clear this exclusion; fresh known non-flywheel telemetry starts
qualification again. Manual previews, visual electrical warnings, raw telemetry
and recording retain their existing behavior. Flywheel handling of ride statistics
and GPS capture is a separate policy review.

Battery speech uses confirmed current Refloat voltage limits, 10%/5% voltage-window
headroom, running-state eligibility and regen for advance high alerts, with separate
one-minute low/high cooldowns. Visual battery qualification shares the service owner
but uses a 25% low margin and immediate high entry above the configured cell profile's
zero-current full-pack voltage. Visuals need fresh ready-board voltage, including while
stationary or drawing power; they do not depend on speech settings or eligibility.
Low recovery uses an additional 2% of the Refloat window; high recovery uses 0.5%
of full-pack voltage, both for two continuous seconds. Percentages remain capped at 100. Missing Refloat limits and invalid estimation configuration independently disable
low and high visuals respectively. No frontend or native adapter computes thresholds.

The closed `voiceSettingsAvailable` OS fact controls the shared Settings button.
`announcements.openVoiceSettings` requires the foreground and one bounded native
completion within five seconds; it accepts no caller-provided URL. Android opens
the system TTS chooser. Apple reports unavailable because no supported public
deep link opens its speech voice chooser. Opening settings never takes audio focus.
