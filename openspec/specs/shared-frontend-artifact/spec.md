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
The frontend SHALL retain SvelteKit, TypeScript, Tailwind, and static adapter tooling behind the commands `npm run dev`, `npm run build`, and `npm test`.

#### Scenario: Hot-reload development
- **WHEN** a developer runs `npm run dev`
- **THEN** the frontend and deterministic backend mock start with SvelteKit hot module replacement

### Requirement: Minimal frontend serving
Each native backend SHALL serve the embedded HTML only at `/` and SHALL NOT implement a general static-file server.

#### Scenario: Root document
- **WHEN** an authenticated application WebView requests `/`
- **THEN** the backend returns the embedded HTML with a `text/html` content type

#### Scenario: Unknown content path
- **WHEN** a client requests an unrecognized non-API path
- **THEN** the backend returns `404` without resolving a filesystem path
