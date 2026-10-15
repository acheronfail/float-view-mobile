# Design

## Context

The existing workflow builds three platforms on every master push and maps run numbers directly to version patches. Native package builds currently derive their numeric build identifier from that patch. Existing stable history ends at v0.1.49.

## Goals / Non-Goals

Provide nightly development downloads, deliberate tagged releases, semantic release helpers and independent native metadata with no new dependencies. Preserve historical releases. Store signing/upload, channel-specific application identities and automatic commit-message classification are out of scope.

## Decisions

- Keep one existing workflow/counter for all channels. Add a preparation job that resolves display version, numeric marketing version, build number, configuration and skip state. Serialize distribution runs and check master history with a full tag-aware checkout.
- Strict stable tags are authoritative. Choose the greatest numeric stable ancestor (never the GitHub latest badge or a lexical sort). Nightlies use the next patch with `-nightly.<commits-since-tag>.g<short-sha>`. Initial repositories use 0.0.0 as a calculation baseline.
- `just release patch|minor|major` validates a clean current master, fetches remote tags/master, calculates the increment and pushes only its annotated tag. It does not modify source or infer intent from commit text.
- Use the existing workflow run number as the independent native build identifier; reruns reuse that identity. Retain the workflow file/counter. Store retries needing new binary identities must use a fresh run rather than reupload the same build.
- Both channels reuse platform jobs. Stable builds use Release configuration. Current AltStore, Android sideload and self-signed macOS outputs remain the delivery targets; store distribution is deferred.
- Stable publication uses an unpublished draft until all assets are uploaded, then publishes it. Published stable releases are never clobbered. A failed draft can be resumed. Nightly uploads new versioned assets before moving its tag/metadata and removing superseded assets. A body marker records the successful source SHA for unchanged-build detection. API failures other than confirmed 404 abort rather than masquerading as missing releases.
- The frontend accepts general stable/nightly versions. Apple uses numeric core versions and an explicit build number, and Android has independent versionName/versionCode. Local versioned builds require an explicit positive build number; ordinary development remains unversioned.

## Risks / Trade-offs

A rolling Nightly is mutable and GitHub cannot update its tag, release and assets transactionally; publication stages preserve previous assets on failure and retries repair incomplete work. CI run numbers are unique within this retained workflow, not globally across arbitrary workflows. GitHub concurrency may coalesce pending pushes; Nightly represents successful development snapshots rather than promising every commit a release. Existing sideload signatures are not store distribution credentials.

## Verification

Use isolated temporary Git repositories and a fake GitHub executable for version ordering, ancestry, bump resets, dirty/stale helper rejection, skip/failure behavior and publication recovery. Check frontend typing/lint/build and Chromium/WebKit version rendering, workflow schema/shell syntax and native metadata/build paths. Use real CI for affected platform packaging when pushed. No BLE behavior changes.

## Migration Plan

Replace fixed validators and patch-derived native build numbers together, preserve historical stable tags, publish future master builds only to Nightly and reserve stable releases for tag pushes. Sync the new spec and archive after relevant verification.
