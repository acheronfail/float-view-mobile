## MODIFIED Requirements

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
