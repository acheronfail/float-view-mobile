# Float View API v1

This directory is the source of truth for the versioned local API between the
shared web UI and every native backend.

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

`/api/v1/ws` is the sole UI transport. The mock and native hosts implement the
same JSON-RPC envelope and `system.snapshotChanged` notification. Some product
operations are still ahead of the committed schema; do not advertise them as
contract-complete until schema, examples, generated models, mock, conformance,
Swift, and Kotlin agree.

The black-box system suite is under `conformance/` and can be run against the
mock with `just verify` or `cd web && npm run api:conformance:mock`.
