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
| Bluetooth transport   | `incomplete` — physical/background checkpoint remains                                                              | `incomplete` — physical checkpoint remains                                                                      | `incomplete` — physical/background checkpoint remains                                                           |
| Background execution  | `incomplete` — restoration and background ride checkpoint remain                                                   | `incomplete` — inactive-window continuity evidence remains                                                      | `incomplete` — foreground-service instrumentation and physical evidence remain                                  |
| Location              | `incomplete` — physical authorization/background evidence remains                                                  | `incomplete` — physical provider-transition evidence remains                                                    | `incomplete` — instrumentation and physical provider-transition evidence remain                                 |
| Native documents      | `incomplete` — Rust-owned operations are reachable; automated edge cases and physical picker/share evidence remain | `incomplete` — Rust-owned operations are reachable; automated edge cases and physical open/save evidence remain | `incomplete` — Rust-owned operations are reachable; automated edge cases and physical SAF/share evidence remain |
| System appearance     | `incomplete` — WebView source needs physical verification                                                          | `incomplete` — WebView source needs physical verification                                                       | `incomplete` — WebView source and native chrome need physical verification                                      |
| Lifecycle and WebView | `incomplete` — scene/WebContent replacement evidence remains                                                       | `incomplete` — window/WebContent replacement evidence remains                                                   | `incomplete` — Activity/renderer replacement evidence remains                                                   |
| Runtime bridge        | `incomplete` — physical process-continuity evidence remains                                                        | `incomplete` — physical process-continuity evidence remains                                                     | `incomplete` — instrumentation and physical process-continuity evidence remain                                  |

Software transport is a Rust runtime mode, not a separate native host. Browser
development has no native capability files; it uses the authenticated loopback
endpoints and the same frontend controllers.
