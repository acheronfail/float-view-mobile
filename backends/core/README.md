# Bounded portable backend components

`backends/core` contains narrowly scoped Rust components with explicit host
ownership. It is a locked Cargo workspace, not another backend and not a
production runtime.

The workspace currently has thirty-four members:

| Component               | Scope                                                                 | Integration state                                                                             |
| ----------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `android-jni-support/`   | Checked call-local JNI environments, primitive-array copies, alias validation, and panic containment | Shared internally by the ten Android bridges; exports no JNI symbol and owns no protocol behavior |
| `apple-ffi-support/`     | Dependency-free checked regions, panic containment, and bounded C output publication | Shared internally by the ten Apple bridges; exports no C symbol and owns no protocol behavior |
| `refloat-compatibility/` | Safe bounded Refloat INFO identity, exact profile catalogue, and canonical dynamic-layout admission | Selected by Apple and Android through separate host bridges |
| `refloat-compatibility-android-jni/` | Four-symbol stateless primitive-array JNI ABI for INFO and layout admission | Temporary bridge selected only by Android |
| `refloat-compatibility-apple-ffi/` | Four-symbol caller-owned-buffer C ABI for INFO and layout admission | Temporary bridge selected only by the shared iOS/macOS adapter |
| `refloat-configuration-codec/` | Safe bounded Qt-compressed `ConfigParams` parsing, schema normalization, canonical schema/value records, fingerprint/signature calculation, and exact typed value serialization | Selected by Apple and Android through separate host bridges |
| `refloat-configuration-service/` | Safe connection-scoped catalogue, schema paging/cache admission, read/default/write verification, revision, flywheel, and recovery decisions above the codec | Exhaustively tested candidate; deliberately unselected by every production host |
| `refloat-configuration-codec-android-jni/` | Temporary four-operation stateless primitive-array JNI ABI for Refloat configuration records | Selected only by Android until the unified Rust runtime removes the bridge |
| `refloat-configuration-codec-apple-ffi/` | Temporary four-symbol stateless caller-owned-buffer C ABI for Refloat configuration records | Selected only by the shared iOS/macOS adapter until the unified Rust runtime removes the bridge |
| `refloat-control/`       | Safe bounded Refloat live-control payload encoding and lighting response decoding | Selected by Apple and Android through separate host bridges                         |
| `refloat-control-apple-ffi/` | Three-symbol caller-owned-buffer C ABI for Refloat controls              | Selected only by the shared iOS/macOS adapter                                       |
| `refloat-control-android-jni/` | Three-symbol primitive-array JNI ABI for Refloat controls             | Selected only by Android                                                            |
| `refloat-dynamic-telemetry/` | Safe bounded Refloat command-32 layout and command-31 dynamic telemetry decoding plus canonical version-1 layout/sample records | Selected by Apple and Android through separate host bridges |
| `refloat-dynamic-telemetry-apple-ffi/` | Three-symbol stateless caller-owned-buffer C ABI for dynamic telemetry | Temporary bridge selected only by the shared iOS/macOS adapter |
| `refloat-dynamic-telemetry-android-jni/` | Three-symbol stateless primitive-array JNI ABI for dynamic telemetry | Temporary bridge selected only by Android |
| `refloat-legacy-telemetry/` | Safe bounded Refloat command-10 compact telemetry decoding and canonical version-1 record | Selected by Apple and Android through separate host bridges |
| `refloat-legacy-telemetry-apple-ffi/` | Two-symbol caller-owned-buffer C ABI for legacy telemetry | Temporary bridge selected only by the shared iOS/macOS adapter |
| `refloat-legacy-telemetry-android-jni/` | Two-symbol primitive-array JNI ABI for legacy telemetry | Temporary bridge selected only by Android |
| `vesc-bms-values/`       | Safe bounded VESC command-96 BMS values decoding                     | Selected by Apple and Android through separate host bridges                                    |
| `vesc-bms-values-android-jni/` | Two-symbol primitive-array JNI ABI for BMS values              | Selected only by Android                                                                       |
| `vesc-bms-values-apple-ffi/` | Two-symbol caller-owned-buffer C ABI for BMS values              | Selected only by the shared iOS/macOS adapter                                                  |
| `vesc-controller-data/`  | Safe bounded VESC command-4 realtime and command-14 current-limit decoding | Selected by Apple and Android through separate host bridges                      |
| `vesc-controller-data-android-jni/` | Two-symbol primitive-array JNI ABI for controller data | Selected only by Android                                                           |
| `vesc-controller-data-apple-ffi/` | Two-symbol tagged-record C ABI for controller data | Selected only by the shared iOS/macOS adapter                                                |
| `vesc-custom-configuration/` | Safe bounded VESC commands 92–95 envelope encoding and decoding  | Selected by Apple and Android through separate host bridges                                   |
| `vesc-custom-configuration-apple-ffi/` | Three-symbol caller-owned-buffer C ABI for commands 92–95 | Selected only by the shared iOS/macOS adapter                                                  |
| `vesc-custom-configuration-android-jni/` | Three-symbol primitive-array JNI ABI for commands 92–95 | Selected only by Android                                                                       |
| `vesc-firmware-identity/` | Safe bounded `COMM_FW_VERSION` decoding and canonical version-1 identity record serialization | Selected by Apple and Android through separate host bridges                                   |
| `vesc-firmware-identity-apple-ffi/` | Two-symbol caller-owned-buffer C ABI for firmware identity | Selected only by the shared iOS/macOS identity adapter                                        |
| `vesc-firmware-identity-android-jni/` | Two-symbol primitive-array JNI ABI for firmware identity | Selected only by Android                                                                      |
| `vesc-frame/`             | Safe bounded VESC CRC, frame encoding/decoding, and stream scanning | Apple and Android select every framing decision                                 |
| `vesc-frame-apple-ffi/`   | Caller-owned-buffer C ABI                                             | Selected only by the shared iOS/macOS frame-codec adapter                       |
| `vesc-frame-android-jni/` | Primitive-array JNI ABI                                              | Selected by Android for CRC, encoding, exact decoding, and stream-boundary scans |
| `vesc-request-scheduler/` | Safe bounded generation, priority/FIFO, attribution, deadline, cancellation, and ordered host-effect state machine | Exhaustively tested application-behavior candidate; deliberately unselected by every production host |

The thirty-four crates remain independent components and artifacts. The ten
Apple-selected component families' repeated
source-build mechanics are centralized in `build-apple-bridge.sh` and
`build-android-bridge.sh`, driven by a closed ten-entry Apple and
ten-entry Android `bridge-build-catalog.sh` allowlist. Cross-platform callers pass `frame`,
`firmware-identity`, `custom-configuration`, `bms-values`, `controller-data`,
`refloat-control`, `refloat-compatibility`, `refloat-dynamic-telemetry`,
`refloat-legacy-telemetry`, or `refloat-configuration-codec`. Package names,
archive/library names, diagnostics, and exact JNI symbols come from the
catalog. The catalog is deliberately non-executable, while the two builders
retain the pinned offline toolchain, Apple target, Android NDK/API/ABI,
architecture, and symbol checks.

The ten Apple bridges also share the protocol-agnostic memory-boundary
mechanics in `apple-ffi-support/`: checked pointer regions, disjointness,
panic containment, borrowed input construction, and bounded byte/scalar
publication. Exported functions, status mapping, result records, headers,
archives, domain dependencies, and Swift adapters remain explicit in each
bridge. Header generation and export macros are intentionally absent because
they would add tooling while obscuring the small reviewed public ABIs.

The ten Android bridges similarly share only protocol-agnostic JNI safety in
`android-jni-support/`: call-scoped environment and exception checks, primitive
array lengths and copies, local-reference alias checks, exact integer
publication, and panic containment. The support crate is the workspace's only
direct `jni-sys` consumer. Export names, statuses, validation order, result
records, domain dependencies, Kotlin loaders, and ten independent libraries
remain explicit. JNI export macros, binding generation, and higher-level JNI
wrappers stay out of scope because the small bridge ABIs differ and the shared
mechanics do not need another dependency or generated source tree.

Android's application build consumes these ten artifacts through one typed
Gradle integration catalog and two generic task factories. That catalog holds
only Gradle-owned task suffixes, source inputs, generated directories, host
library stems, and JVM properties; `bridge-build-catalog.sh` remains the closed
owner of Cargo packages, native artifact names, diagnostics, and JNI symbols.
Keeping the two small catalogs explicit avoids generated metadata or shell
parsing while audits require the same ordered identifier set and the exact
thirty public Gradle task names. A later selected bridge adds one entry rather
than copying package, host, source-set, test, merge, and clean-order blocks.

The shared Apple backend retains only bounded Swift stream storage. Rust scans
each borrowed region, reports exact consumption boundaries, validates every
complete frame, and owns outbound encoding without transferring allocator or
object lifetime across the bridge. Android uses the same safe crate through a
separate four-function JNI bridge over the shared support. Rust selects Android stream boundaries;
Kotlin retains only caller-owned bounded byte storage, mechanical consumption,
and error mapping without transferring allocator or object lifetime.

The firmware identity component has an empty production dependency graph. It
owns the one bounded platform-neutral identity record, including version,
layout, flags, bounds, replacement-decoded names, optional CRC, serialization,
and closed semantic outcome. Its tests use only the existing safe framing crate
to unwrap committed framed fixtures. Separate Apple and Android bridges depend
only on it and their minimal host ABI support; neither bridge exposes Rust
allocation or layout. The bridges retain only explicit C-pointer or call-local
JNI validation, fixed host status mapping, and caller-owned publication. This
ordinary shared API reduced their combined production source from 418 to 264
lines without code generation or export macros; later record families should
follow this pattern only through their own incremental changes.

The custom-configuration component encodes and decodes the bounded VESC
commands 92–95 envelope while treating compressed schemas and serialized
values as opaque caller-owned bytes. Separate Apple and Android bridges expose
only fixed metadata and caller-owned regions; schema and typed-value semantics
remain native.

The Refloat configuration codec owns the complete safe protocol layer:
bounded Qt/zlib expansion, streaming `ConfigParams` validation, normalized
schema metadata, raw-source SHA-256 fingerprints, CRC32C wire signatures, and
complete typed value decoding/encoding plus canonical schema/value records.
Apple and Android select it through independent temporary four-operation bridges
and strict native record adapters; the superseded Swift and Kotlin protocol
algorithms are gone. Both hosts retain configuration lifecycle, caches,
scheduling, verified writes, Tune persistence, application behavior, and OS
capabilities until their later unified-runtime migration. Both codec bridges and
record adapters are deleted at that boundary.

The Refloat configuration service candidate owns the deterministic lifecycle
above that codec: audited release selection, host-provided cache admission,
bounded schema paging, generations and operations, current/default revisions,
flywheel policy, verified writes, normalization, conflicts, and recovery. Its
API is platform-neutral facts, events, effects, and snapshots. It has no bridge,
host package input, callback, transport, storage implementation, scheduler, or
OS dependency, so Swift and Kotlin remain selected until later host-cutover
changes. Those cutovers must delete the native state machines; once the unified
runtime owns the surrounding application behavior, it also deletes both codec
bridges and canonical-record adapters.

The VESC request scheduler candidate owns connection-wide arbitration above
protocol request construction and below host timer/BLE execution: bounded
priority and FIFO ordering, one in-flight response request, typed attribution,
generations, deadlines, cancellation, one-way completion, and deferred pumping.
It emits ordered safe Rust effects and owns no clock, timer, writer, callback,
host ABI, protocol codec, persistence, or transport. Swift and Kotlin remain
selected; later unified-runtime cutovers compose this candidate directly rather
than adding scheduler-specific Apple FFI and Android JNI crates.

The BMS values component decodes only the bounded command-96 payload into a
platform-neutral fixed-capacity model. Apple and Android select it through
separate caller-owned-record bridges. BMS compatibility, discovery, routing,
scheduling, polling, observation time, application state, and live-only policy
remain native.

The Refloat control component owns each host's exact
package, flywheel, legacy/stable lighting, and fatal-clear wire payloads. It
owns no compatibility lookup, routing, framing, authorization, scheduling,
confirmation, telemetry, or application state. The shared Apple adapter and
Android adapter use independent focused three-symbol bridges.

The Refloat compatibility component constructs and decodes only the bounded
INFO identity and applies the exact audited package, telemetry, control, and
required-layout policy to the canonical dynamic-layout record. Apple and
Android select it through separate temporary four-symbol bridges and strict
record adapters; the superseded Swift and Kotlin INFO offsets, catalogues,
profile derivation, and set-based layout admission are gone. Both hosts retain
discovery effects, routing, scheduling, state, presentation, and OS behavior.
The host bridges are migration scaffolding and are deleted when the unified
Rust application runtime no longer needs native protocol boundaries.

The controller-data component decodes only the consumed prefix of standard
VESC realtime values and the current-limit subset of motor configuration.
Apple and Android select it through separate tagged caller-owned-record bridges.
Both hosts retain request construction,
direct/CAN routing, scheduling, and model mapping until later cutovers.

The Refloat legacy telemetry component decodes complete fixed-layout command-10
detail levels and active-fault responses into an allocation-free canonical
record. Apple and Android select it through temporary two-symbol bridges and
strict native record adapters; both superseded native wire parsers are gone.
Both hosts retain compatibility,
polling, cache merging, application mapping, persistence, and OS behavior. The
host bridges are migration scaffolding and are deleted when the unified Rust
application runtime no longer needs native protocol boundaries.

The Refloat dynamic telemetry component owns bounded command-32 UTF-8 layout
validation and command-31 mask-dependent decoding, including the complete
finite source float16 range, semantic state, charging and alert tails, typed
known values, bounded extensions, and canonical layout/sample records. Apple
is selected through temporary stateless three-symbol bridges and strict record
adapters on both host families; the superseded Swift and Kotlin wire parsers are
gone. Both bridges are migration scaffolding and are deleted when the unified
Rust application runtime no longer needs native protocol boundaries.

Do not add empty future crates, generated bindings, a general runtime, host
callbacks, or OS-owned Bluetooth, location, lifecycle, permission, native-file,
theme, or UI behavior here. Unsafe code belongs only in named audited host
boundary crates: exported operations remain in each bridge, shared Apple and
Android primitives remain in their support components, and every safe
algorithm crate continues to forbid unsafe code.
