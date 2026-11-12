# Design

## Context

The shared listener already reports its bound port, but the closed host protocol and native wrappers enforce a fixed registry. Frontend bootstrap derives URLs from the listener. Browser storage only holds optional early appearance hints.

## Goals / Non-Goals

Keep one Rust service and existing HTTP/WebSocket transport. Remove native port selection policy and origin-dependent startup appearance. Preserve product contracts/storage and development fixture/mock ports.

## Decisions

- Native host start requests use port zero; Rust binds once and returns the actual port. No bind probing or separate discovery service.
- macOS uses its existing readiness stdout. Mobile debug hosts publish only the actual port to an app-private readiness file after listener startup. Hosts remove stale readiness during process startup; runners read through existing OS developer tools, validate bootstrap against the discovered port, and create scoped forwarding. No tokens are written to readiness files. iOS simulator connects directly; Android uses ADB tcp:0 allocation; iproxy uses an available host port with bind failure remaining visible.
- Serve current validated appearance as HTML root data attributes, using an explicit marker in the built template. The early script and theme controller prefer these values to optional localStorage. Re-read the published authoritative snapshot for every HTML request, avoiding stale values on renderer recreation. Only closed enum values enter HTML.
- No extra dependencies, native messaging layer or persistent storage migration. Old origin hints can be ignored safely.

## Risks / Trade-offs

- Stale mobile readiness → remove before launch and publish only after binding; validate numeric ports and bound retries.
- Forwarded host/device ports differ → preserve the device origin in authenticated requests while rewriting only the connection endpoint.
- Initial native canvas remains OS-host owned → run relevant startup software/native checks; distinguish missing phone visual evidence from software results.
- A free-port check for iproxy cannot reserve its socket → monitor proxy startup and fail actionably on contention; production listeners and Android forwarding allocate atomically.

## Migration Plan

Ship host, shared server and frontend changes together. No durable data upgrade. Update tooling to discover endpoints; remove fixed-port constants and obsolete launch configuration. Rollback requires the previous matching binaries/frontend/tooling, without data changes.
