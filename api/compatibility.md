# API v1 compatibility

The contract uses an incompatible major version in the WebSocket path and
negotiates a minor version through `system.hello`.

- A backend implementing v1 must support `system.hello` and
  `system.getSnapshot`.
- Minor releases may add optional fields, operations, capabilities, and
  notifications. They must not change the meaning of an existing field or state
  transition. Values may be added only to enums explicitly documented as
  extensible; the current v1 enums are closed and changing them requires a new
  major version.
- Clients must ignore unknown optional fields and notifications.
- Backends must reject an unsupported major version and a non-overlapping minor
  range with a structured `unsupported` JSON-RPC error.
- A capability may be advertised as available only when that backend passes its
  corresponding conformance scenarios.

`/api/v1/ws` is the sole UI protocol endpoint.
