# Spec Delta

## ADDED Requirements

### Requirement: Exact beta3 configuration support

The backend and simulator SHALL additionally support exact Refloat 1.3.1-beta3 with its audited full-schema fingerprint and wire signature using the existing bounded schema/value codec, configuration write/readback, cache and Tune policies. Beta3 and beta2 SHALL retain separate exact-version Tune namespaces even when their schemas match. Supported stored data SHALL remain readable.

#### Scenario: Beta3 schema and verified writes

- **WHEN** beta3 supplies its audited 172-field schema and complete current/default values
- **THEN** configuration becomes available with beta3 identity; eligible edited writes and same-version portable Tune application require fresh authority and complete verified readback

#### Scenario: Matching schema does not authorize another release's Tune

- **WHEN** beta3 is connected and a saved beta2 Tune has the same schema fingerprint and wire signature
- **THEN** that Tune remains retained but is not offered or applied as compatible with beta3

#### Scenario: Reconnect invalidates current authority

- **WHEN** beta3 disconnects and reconnects
- **THEN** old revisions cannot authorize writes and fresh current-generation identity, timeout evidence and current values are required
