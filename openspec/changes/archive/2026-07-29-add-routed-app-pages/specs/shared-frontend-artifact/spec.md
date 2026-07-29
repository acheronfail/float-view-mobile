## ADDED Requirements

### Requirement: Client-side routes preserve the single artifact

The one frontend artifact SHALL resolve Home, live-board, Sessions, session-detail, Boards, board-configuration, and Settings routes entirely in the client. Navigating among them MUST NOT require a non-root HTML response, an external runtime asset, or a native general-purpose static-file server.

#### Scenario: Production route navigation

- **WHEN** the embedded frontend navigates between any supported application routes
- **THEN** the route is resolved from the already loaded inline `index.html`

#### Scenario: Production artifact contents

- **WHEN** the routed frontend production build completes
- **THEN** `frontend/build` still contains only the canonical inline `index.html`

#### Scenario: Unknown native content path

- **WHEN** a client requests a non-root, non-API path from a native backend
- **THEN** the backend continues to return `404`

### Requirement: Discoverable frontend ownership

`frontend/src/lib/README.md` SHALL define an ownership-first Svelte code-layout convention. Application-shell code, backend integration, product features, and genuinely reusable domain-independent UI SHALL have distinct owners; feature-local components, reactive controllers, models, pure calculations, and tests SHALL remain colocated by default.

#### Scenario: Locate a product capability

- **WHEN** a maintainer looks for connection, session, board, or settings behavior
- **THEN** its view components and supporting frontend logic are discoverable under one correspondingly named feature directory

#### Scenario: Locate shared backend integration

- **WHEN** a maintainer looks for bootstrap, transport, authoritative frontend state, or the wire contract
- **THEN** those files are discoverable together under `$lib/backend`

#### Scenario: Component grows private implementation

- **WHEN** a complex component gains meaningful private children or implementation modules
- **THEN** it may receive a PascalCase capsule directory while simple feature directories remain flat

### Requirement: Frontend ownership boundaries

Route files SHALL remain thin route adapters, feature code SHALL use relative imports within its owner and `$lib/...` imports across ownership boundaries, and one feature MUST NOT import another feature's private files. Generic catch-all component, controller, effect, type, utility, helper, or store directories and default barrel files SHALL NOT be introduced.

#### Scenario: Shared presentation primitive

- **WHEN** a domain-independent component has demonstrated reuse across features
- **THEN** it may be owned by `$lib/ui` rather than duplicated

#### Scenario: Feature-private state

- **WHEN** reactive state or a model is used by one product feature
- **THEN** it remains beside that feature rather than moving to a technical catch-all directory

#### Scenario: Colocated test

- **WHEN** frontend logic or a component has a focused test
- **THEN** its `*.spec.ts` file is stored beside the implementation it verifies
