# Float View backend conformance

The conformance suite treats a backend as a black box. It connects through the
same versioned WebSocket used by the shared UI and verifies behavioral semantics,
not platform implementation details.

Current system scenarios verify:

- requests before `system.hello` fail consistently;
- incompatible major versions return the stable `unsupported` error;
- v1.0 negotiation includes capabilities and an authoritative snapshot;
- a later snapshot is queryable and structurally consistent;
- unknown methods return the standard JSON-RPC method-not-found code plus the
  stable Float View error kind.

Run the suite against the development mock:

```sh
cd web
npm run api:conformance:mock
```

`conformance/src/system.ts` depends only on a small request/close interface.
Android, iOS, macOS, Linux, and Windows test hosts should adapt their WebSocket
client to that interface and execute the same scenarios.
