# Shared Frontend Artifact Specification

## Purpose

Define the single SvelteKit frontend source and self-contained HTML artifact embedded unchanged by every native host.

## Requirements

### Requirement: One frontend source

The repository SHALL contain the one SvelteKit and TypeScript application under lowercase `/frontend`. Android, iOS, macOS, and browser development SHALL use that frontend, and frontend code MUST NOT branch on the native platform.

#### Scenario: Same application behavior on every host

- **WHEN** the frontend is loaded by Android, iOS, macOS, or the development mock
- **THEN** it uses the same components, state store, backend client, and backend contract

### Requirement: One inline production artifact

`npm run build` from `/frontend` SHALL produce `frontend/build/index.html` as the one canonical HTML runtime artifact with application JavaScript and CSS inline, and the artifact MUST NOT require external build assets.

#### Scenario: Production build is self-contained

- **WHEN** the production frontend build completes
- **THEN** exactly one runtime HTML resource is required to load the application

#### Scenario: Native builds embed identical content

- **WHEN** Android, iOS, and macOS applications are built from the same source revision
- **THEN** each application embeds byte-identical frontend HTML

### Requirement: Conventional frontend development loop

The frontend SHALL retain SvelteKit, TypeScript, Tailwind, and static adapter tooling behind the commands `npm run dev`, `npm run build`, and `npm test`. `npm run dev` and `just dev mock` SHALL provide deterministic mock-backed hot module replacement, while `just dev macos|ios|android [target]` SHALL use the same development frontend against the selected native backend.

#### Scenario: Mock hot-reload development

- **WHEN** a developer runs `npm run dev` or `just dev mock`
- **THEN** the frontend and deterministic backend mock start with SvelteKit hot module replacement

#### Scenario: Native hot-reload development

- **WHEN** a developer runs `just dev macos`, `just dev ios <target>`, or `just dev android <target>`
- **THEN** the same frontend components, state store, backend client, and contract run with SvelteKit hot module replacement against the selected native backend

#### Scenario: Production frontend build

- **WHEN** a developer runs `npm run build`
- **THEN** development proxy behavior is absent from the canonical inline production artifact

### Requirement: Minimal frontend serving

Each native backend SHALL serve the embedded HTML only at `/` and SHALL NOT implement a general static-file server.

#### Scenario: Root document

- **WHEN** an authenticated application WebView requests `/`
- **THEN** the backend returns the embedded HTML with a `text/html` content type

#### Scenario: Unknown content path

- **WHEN** a client requests an unrecognized non-API path
- **THEN** the backend returns `404` without resolving a filesystem path

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
