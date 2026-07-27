# Float View API v1

This directory is the source of truth for the versioned local API between the
shared web UI and every native backend.

The active delivery sequence is documented in
[`../docs/reset-roadmap.md`](../docs/reset-roadmap.md). The lifecycle acceptance
contract for the first connect/read/log/disconnect slice is
[`../docs/vertical-slice-v1.md`](../docs/vertical-slice-v1.md).

```text
asyncapi.yaml                 WebSocket channel and operations
schemas/*.schema.json         JSON Schema 2020-12 payload definitions
examples/*.json               Validated protocol examples
generated/typescript/models.ts
                              Committed generated TypeScript types
generated/swift/FloatViewAPI.swift
                              Committed generated Codable models
generated/kotlin/FloatViewAPI.kt
                              Committed generated kotlinx.serialization models
compatibility.md              Major/minor compatibility policy
```

From `web/`:

```sh
npm run api:generate
npm run api:check
```

`api:check` validates all schemas, checked-in examples, the required AsyncAPI
structure, and generated TypeScript/Swift/Kotlin drift. Edit schemas, never
generated models.

Native model checks:

```sh
swiftc generated/swift/FloatViewAPI.swift tests/swift/ContractSmoke.swift \
  -o /tmp/float-view-api-contract-smoke
/tmp/float-view-api-contract-smoke examples/system.hello.response.json

cd ../android
./gradlew :api-models:test
```

The current implementation is intentionally additive:

- `/api/v1/ws` is JSON-RPC 2.0 and currently supports `system.hello` followed by
  `system.getSnapshot` in the development mock.
- `/ws` remains the legacy protocol used by iOS, Android, and the existing UI.

Do not send legacy messages to the v1 endpoint or add v1 fields to the legacy Zod
contract. Migrate one complete, conformant operation at a time.

The black-box system suite is under `conformance/` and can be run against the
mock with `just api-conformance`.
