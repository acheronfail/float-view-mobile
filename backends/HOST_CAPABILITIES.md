# Native host capabilities

This file is the source-of-truth inventory for native OS ownership. The shared
frontend and Rust backend own everything not listed here.

## Status

- `implemented`: the production entry point exists, names its concrete owner,
  and has the focused automated and physical evidence listed by the host.
- `unsupported`: the entry point is a small compiling stub that explains the OS
  limitation and returns the stable unavailable result. The frontend does not
  advertise the capability as available.
- `incomplete`: implementation or required evidence is missing. This status is
  allowed while developing a change but fails the final host-surface gate.

An absent row or entry file is never interpreted as unsupported.

## Required entry points

Every iOS, macOS, and Android host exposes these entry names using `.swift` or
`.kt` as appropriate:

| Capability                      | Entry name                               | Native ownership                                                                                                |
| ------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Bluetooth transport             | `FloatViewBluetooth`                     | OS discovery, permissions/radio, GATT setup, write backpressure, restoration, callback serialization, raw bytes |
| Background execution            | `FloatViewBackgroundExecution`           | OS background modes or service, effective eligibility, notification requirements                                |
| Location                        | `FloatViewLocation`                      | OS authorization, provider availability, background availability, coordinates and accuracy                      |
| Native documents                | `FloatViewDocuments`                     | Picker/save/share presentation, security-scoped URL or content-URI access, bounded copying                      |
| System appearance               | `FloatViewAppearance`                    | The host's single effective-appearance source and native-only chrome                                            |
| Lifecycle and WebView ownership | `FloatViewLifecycle`, `FloatViewWebView` | Process/window/scene/Activity lifecycle and disposable WebView presentation                                     |
| Runtime bridge                  | `FloatViewRuntimeBridge`                 | One process-owned Rust runtime, one off-UI-thread pull worker, focused commands/events and shutdown             |

`Location` is used instead of `Gps` because the capability also owns
authorization, provider and background availability, and accuracy.

Each host README and entry file records:

1. status;
2. concrete implementation owner or unsupported reason;
3. stable unavailable behavior when unsupported;
4. focused automated verification; and
5. required physical checkpoint.

Forwarding entry points are allowed only when they identify one concrete shared
owner directly. Generic `Domain`, `Services`, `Utils`, and `PlatformAdapter`
files are not capability owners.

## Current inventory

| Capability            | iOS                                                                                                                | macOS                                                                                                           | Android                                                                                                         |
| --------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Bluetooth transport   | `implemented` — physical fixture/background notifications pass | `implemented` — physical 16-scenario matrix passes | `implemented` — physical fixture/background notifications pass |
| Background execution  | `implemented` — physical background ride/runtime continuity passes | `implemented` — normal process/window continuity passes | `implemented` — foreground-service and physical ride pass |
| Location              | `implemented` — focused observations plus physical authorization/background availability pass | `implemented` — embedded metadata plus physical authorization/provider transition pass | `implemented` — focused tests and physical background GPS pass |
| Native documents      | `incomplete` — focused tests pass; physical picker/share roundtrip remains | `implemented` — bounded tests and physical open/save roundtrip pass | `implemented` — bounded tests and physical SAF/share roundtrip pass |
| System appearance     | `implemented` — physical iPhone WebView verification passed                                                         | `implemented` — physical macOS WebView verification passed                                                      | `implemented` — physical Android WebView and native-chrome verification passed                                  |
| Lifecycle and WebView | `implemented` — physical scene/WebView recovery passes | `implemented` — physical window/WebContent replacement passes | `implemented` — physical Activity/WebView replacement passes |
| Runtime bridge        | `implemented` — simulator and physical runtime continuity pass | `implemented` — focused and physical runtime continuity pass | `implemented` — instrumentation and physical runtime continuity pass |

Software transport is a Rust runtime mode, not a separate native host. Browser
development has no native capability files; it uses the authenticated loopback
endpoints and the same frontend controllers.
