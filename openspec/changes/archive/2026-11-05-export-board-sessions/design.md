# Design

## Context

Existing exports run in a bounded shared-service file worker and deliver a staged file through native presentation or authenticated HTTP. Individual archive and CSV encoders already enforce GPS policy and portable names.

## Goals / Non-Goals

Provide one board backup without loading all history into the frontend. No bulk import or format/storage migration, new dependencies, host interfaces or general job framework.

## Decisions

Extend the existing export selector with an exclusive savedDeviceId. Resolve its durable history in ProductStore and prepare bounded source leases under the owner lock; encode on the existing worker. A bundle uses one output stage and one reused scratch stage, staying within the two-transfer staging budget. Keep existing single-export encoding and add a ZIP wrapper with numbered entry prefixes to prevent timestamp collisions. Store inner archives/ZIPs without recompression and copy in bounded cancellable chunks. Limit bundles to 10,000 sessions and the existing 128 MiB export byte bound; fail wholly above either limit. A separate bounded bulk-lease allowance preserves the six ordinary reader leases. Sources remain leased through encoding, preventing destructive maintenance races. Existing encoders capture GPS settings at admission.

Reuse session.export and existing transfer presentation, so both hosts consume the same staged ZIP without policy changes. The board menu calls one shared frontend method with a busy guard and displays an explanation and result.

## Risks / Trade-offs

- Large histories can exceed limits: fail without presenting partial results; individual exports remain available.
- Holding bounded source leases delays deletion/pruning: release on completion/cancellation using existing ownership.
- Nested CSV ZIPs require extraction twice: preserves the established individual export artifacts and avoids another CSV format.
