# Refloat firmware faults

## What users see

Live shows a persistent warning while a controller fault is active or Refloat's
fatal error remains latched. **Fault history…** in Board actions opens a plain
list, newest first. Saved connections read the ride's recorded history; guests
keep the 20 most recent occurrences. Clearing the latch does not remove history.
The sheet restores after WebView recreation and reports read failures with Retry.

Session review shows a **Faults** list and red markers on charts and the map.
Selecting a timed fault inspects that point in the ride. Nearby chart markers
share a count; the list retains each occurrence. Map markers require a continuous
GPS segment covering the fault time. Unknown codes remain visible as a numeric
controller fault. Untimed records stay in the list without a guessed marker.
Lists initially show 20 rows, with Show more for longer histories.

Times use **≈** because they are estimated from telemetry. Old recordings and
faults that predate the ride can say **Time unavailable**. The existing raw state
transition list remains available separately.

## Refloat behavior

The vendored Refloat 1.2.0 and 1.3.0 implementations of
[`cmd_alerts_list`](../vendor/refloat/src/main.c) return the current controller
fault and up to 20 historical start/end records. The volatile buffer may overwrite
older records and disappears on power cycling.

[`AlertTracker`](../vendor/refloat/src/alert_tracker.c) treats firmware faults as
fatal. With Persistent Fatal Error enabled (the default), the latch remains after
an active fault ends. The setting describes pushback and four-beep haptic feedback;
the current haptic selector explicitly prioritizes the fatal pattern. A latch
alone does not establish that balancing is disabled. Refloat supports explicit
clearing as well as power cycling. Neither repairs the underlying cause.

Float View keeps active faults separate from the latch and never creates a
warning from a retained beep reason alone. The existing clear action requires
fresh supported writable state, idle, no active fault and no competing operation.
It waits for telemetry confirmation. Users are told to stop riding and investigate
before clearing or power cycling.

## Timing and storage

The audited [VESC interface](../vendor/refloat/vesc_pkg_lib/vesc_c_if.h) defines
10,000 system ticks per second. The shared service anchors fault ticks to received
fast telemetry on the ride's monotonic timeline. Tick differences handle wrap;
negative ride times and unusable clocks remain unavailable. Estimates slightly
ahead of retrieval are capped at retrieval within the existing 900 ms request
bound, accounting for the larger telemetry response's transport delay. Larger
inconsistencies remain unavailable. Reconnect preserves the history cursor and
cause; a new session resets them. A detected clock rollback invalidates the anchor.

Each `firmwareFaultHistory` event keeps its retrieval time, raw code, source ticks,
active/end state and optional `occurredElapsedMs`. Both starts and ends stay in the
session file and archive, independently of later samples, pruning and latch clear.
Fault-bearing sessions survive the minimum-distance filter. Existing four-field
records remain readable without a storage/schema/archive version change; their
occurrence time is not inferred retrospectively. Frozen baseline archives remain
supported. See [storage details](../backends/sql/README.md).

The authenticated event page adds an optional parallel `faults` column containing
raw codes and occurrence times. Both native hosts use the shared Rust projection.
`LiveSample.faultHistory` contains up to 20 native-owned occurrences, including
sampled faults on older supported packages. Saved history reads are bounded to
100,000 events and 10,000 fault occurrences, and fail visibly instead of displaying
a truncated prefix. Sample-only session refreshes do not reload the fault list.

## Verification

The [ESP32 checkpoint](../conformance/src/physical.ts) covers direct and CAN routes,
transient recovery, reconnect retention and deduplication, clear eligibility,
recorded occurrence times and durable idle-session history. Detailed evidence
belongs under ignored `.local/physical-testing/`. These are source-derived firmware
responses over native BLE; they do not induce a fault on a real controller.

On 2026-10-18, both macOS ESP32 direct/CAN checkpoints passed with Refloat 1.3.0
and 20-byte notification fragments, including the occurrence-time and recent-history
assertions. Chromium/WebKit covered the live menu, history list, chart/map markers
and existing footpad selection. Real-controller fault behavior and iOS/Android
physical coverage were not tested.
