# Design

## Context

See proposal.md. The existing audited catalogue has different full fingerprints for 1.3.0 and 1.3.1 but the same wire signature. The store currently uses exact fingerprint/version filters, and dispatch incorrectly would submit the source fingerprint if those filters alone were relaxed.

## Goals / Non-Goals

Share supported stable-minor Tunes without changing saved identities, observed board versions, wire contracts or storage. Keep unknown-schema admission and unknown-firmware support outside this change.

## Decisions

- Reuse the configuration service's audited catalogue for a bounded Tune schema equivalence key. Stable 1.0.x–1.3.x form independent families; exact existing beta2/beta3 aliases join 1.3.x. Unknown versions retain exact matching. An unknown fingerprint never gains equivalence to a known fingerprint. Matching signatures remain mandatory.
- Add the configuration-service dependency to product-store to reuse this evidence rather than duplicate a catalogue or move firmware policy into the generic codec. Frontend/mock mirror the small family policy, checked against the source catalogue by conformance tests.
- Keep public/stored stable patch versions and full fingerprints. Existing beta normalization remains. SQL filters use bounded family version/fingerprint sets before pagination; cursors bind to compatibility keys. No row rewrites, migration or archive change.
- Validate source portable values against the target schema, preserving exact field-set/type/bounds/encoding checks. Dispatch uses the target's exact fingerprint, signature and version for the complete merged write and readback. Overwrite preserves the original Tune identity.
- Reconcile conflicting canonical scenarios and the separate fallback draft. No implementation of that draft is authorized or required here.

## Risks / Trade-offs

The upstream guarantee does not admit unknown schema metadata: target configuration discovery still requires audited fingerprints, and Tune matching keeps unknown fingerprints separate. Frontend policy duplication is bounded by parity tests. Family queries are bounded to 256 stable patches plus two audited betas and the small audited schema catalogue.

## Migration Plan

No data changes. Older app versions retain their narrower compatibility rules over the same records. Existing safety, freshness and confirmation gates remain unchanged.
