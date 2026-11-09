## ADDED Requirements

### Requirement: Stable minor configuration admission

Supported stable patches SHALL accept audited full-schema fingerprints and wire signatures from their stable minor family, replacing exact patch matching. Unknown schemas and cross-minor schemas SHALL fail closed for configuration. Configuration identity and caches SHALL retain the exact observed patch. Legacy INFO 1.0 SHALL retain fingerprint-selected source identity. Existing exact prerelease schema and Tune-family rules SHALL remain unchanged.

#### Scenario: Later patch reuses known schema

- **WHEN** stable 1.2.255 or 1.3.2 supplies an audited schema from its own stable minor
- **THEN** configuration reads and guarded writes are available with the observed patch version and unchanged full fingerprint/signature
- **AND** reconnect and cached-schema validation apply the same policy

#### Scenario: Schema validation remains local and strict

- **WHEN** a supported patch supplies an unknown fingerprint or signature, a different minor's schema or malformed XML
- **THEN** configuration remains unavailable without broadening admission from the version guarantee alone

#### Scenario: Tune identity remains exact

- **WHEN** a stable patch has the same schema as another patch but a different exact Tune version
- **THEN** it does not silently share saved Tunes across patches
- **AND** exact same-version Tunes and the existing beta2/beta3/stable-1.3.1 Tune family remain available
