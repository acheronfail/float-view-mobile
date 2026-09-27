# Proposal

## Why

Apple and Android duplicate the local server and frontend API in 3,699 lines across their two `LoopbackServer` files. Sharing this boundary before Linux support avoids a third implementation and gives authentication, serialization, and client lifecycle one owner.

## What Changes

- Add a shared Rust loopback server above the application runtime. Move HTTP/WebSocket routes, authentication, request dispatch, contract serialization, state publication, compression, and request cancellation together.
- Add bounded host operations through the existing Apple and Android bindings. Keep OS adapters, permissions, lifecycle, native file presentation, and catalog storage in the hosts.
- Switch iOS/macOS and Android to the shared server within this change. Delete superseded native server logic, payload serializers, tests of deleted internals, and server-only dependencies.
- Extend canonical conformance and packaging checks to prove unchanged frontend behavior, bounded work, background recording, and both production selections.
- Preserve the frontend contract, SQL schema, storage marker, stable ports, embedded HTML, and development proxy behavior. Internal host envelopes and symbol audits change together.

Non-goals: Linux or Windows support, a desktop shell, catalog extraction, Bluetooth/location migration, database migrations, generated APIs, new frontend methods, or a second application runtime.

## Capabilities

### New Capabilities

- `shared-rust-loopback-server`: One shared frontend API server, bounded host operation exchange, independent lifecycle, and deletion requirements for both hosts.

### Modified Capabilities

- `platform-hosts`: Shared Rust owns transport and contract dispatch while native hosts retain their OS responsibilities.
- `unified-rust-runtime`: Keep the application runtime free of transport while allowing a shared Rust server above it.
- `apple-unified-runtime-cutover`: Replace Swift transport ownership with a thin shared-server adapter.
- `android-unified-runtime-cutover`: Replace Kotlin transport ownership with a thin shared-server adapter.
- `backend-conformance`: Require shared-server parity, boundary, lifecycle, packaging, and deletion evidence.

## Impact

Affected areas include `backends/core`, both native `transport` adapters, application integration, native build definitions, `conformance`, ownership documentation, and root verification recipes.

The change removes Swift FlyingFox and Android Ktor server dependencies when their remaining references are deleted. It replaces their HTTP/WebSocket machinery with one audited Rust server stack. Dependency selection and cross-target compilation precede production selection.

Public API and persistence behavior remain unchanged. The TypeScript contract stays the source of truth. Host communication remains hand-written, versioned, bounded, and callback-free. Final source and dependency inventories must demonstrate a net reduction rather than hide duplicate servers behind adapters.
