# Proposal

## Why

Release patches currently follow the CI counter, so minor/major bumps cannot reset the patch independently. Continuous development downloads also appear as permanent stable releases.

## What Changes

- Derive stable versions from explicit immutable `vX.Y.Z` tags and provide `just release patch|minor|major` to calculate and push annotated tags.
- Build master pushes, a daily schedule and manual master runs as commit-identified nightly prereleases, updating one rolling Nightly download.
- Separate numeric native build identifiers from display/marketing versions across the frontend, iOS, macOS and Android.
- Remove the fixed `v0.1.x` validators and patch-derived native build numbers.
- Preserve existing version tags/releases. No app-store submission, signing-identity change, public API change or storage change.

## Capabilities

### New Capabilities

- `app-distribution`: version resolution, nightly publication, tagged releases and native build metadata.

### Modified Capabilities

None.

## Impact

GitHub Actions, release tooling, local packaging, frontend build metadata, native version configuration and contributor documentation. Uses Node standard libraries and existing Git/GitHub CLI tools; adds no dependencies.
