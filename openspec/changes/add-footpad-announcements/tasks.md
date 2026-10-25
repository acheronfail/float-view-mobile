# Tasks

## 1. Contract and persisted settings

- [ ] 1.1 Add `footpadEnabled` and `footpadDelaySeconds` to the frontend contract/defaults, shared product settings, projection and client/mock settings paths; verify false/1 defaults and accepted whole-second values 1–5 through focused settings and contract tests.
- [ ] 1.2 Default absent new persisted fields without masking invalid present values; add product-store tests for supported old settings with non-default announcements, frozen baseline fixture preservation, new-field save/reopen, invalid updates and revision conflicts; verify affected product-store tests pass without modifying frozen fixtures.
- [ ] 1.3 Update settings/ownership documentation for the new additive fields and unchanged storage layout; verify documented defaults and compatibility behavior match tests and the spec delta.

## 2. Shared announcement policy

- [ ] 2.1 Extend observations and bounded per-side monotonic release timing, retaining speech timer scheduling while delay is accumulating; add policy tests for left/right mapping, independent durations, exact delay boundary, fixed >5 km/h gate, reverse, units, saved/guest context and footpad-only operation; verify focused backend-service tests pass.
- [ ] 2.2 Implement contact, speed, unknown/missing evidence, stale-gap, connection/controller and footpad settings resets without clearing shared cooldown; add tests for a delayed worker crossing the two-second gap, WebView replacement and process restart; verify no elapsed time carries across invalid context.
- [ ] 2.3 Integrate qualifying sides into the existing typed speech effect and shared admission path; add tests for persistent repeats, speed/load/test cooldown sharing, simultaneous rules, busy/muted/unavailable audio, recovery during playback, cancellation and late callbacks; verify one bounded slot and no queued or catch-up speech.
- [ ] 2.4 Update shared-core announcement ownership documentation and remove obsolete two-rule assumptions; verify the documented timing, effect and cancellation behavior against focused policy tests.

## 3. Native speech consumers

- [ ] 3.1 Extend Apple speech effect decoding and phrase construction for closed left/right/both values and footpad-only utterances; update Swift tests for exact wording/order, invalid values, existing speed/load effects and resource lifecycle; verify affected Apple speech/bridge tests pass.
- [ ] 3.2 Extend Android's strict speech request decoder and phrase construction in the same way; update Kotlin tests for exact keys, closed values, footpad-only speech, combined ordering and existing speed/load behavior; verify affected Android speech/bridge tests pass.

## 4. Settings UI and canonical coverage

- [ ] 4.1 Add the independent Footpad announcements control and labelled 1–5-second native slider with visible seconds, existing autosave/error behavior and a clear speed-gate explanation; update frontend documentation and focused tests for defaults, retained disabled delay, save serialization and validation; verify frontend tests and `npm run check` pass.
- [ ] 4.2 Extend canonical mock/browser coverage for toggle primary/non-primary/keyboard activation, slider pointer/touch/keyboard edits, persisted state and save failure; build the frontend then run the affected announcements browser test as documented in CONTRIBUTING.md, verifying behavior on desktop and touch contexts.
- [ ] 4.3 Add canonical direct/native contract scenarios for new settings and automatic footpad qualification, repeats, combined ordering and reconnect resets using existing simulator controls; verify focused direct conformance and affected Apple/Android integration scenarios pass without adding new wire commands or fixture-side policy.

## 5. Integration verification

- [ ] 5.1 Run affected Rust crate/consumer tests, formatting and warnings-denied Clippy checks; run relevant frontend, client/mock, canonical conformance and host checks from CONTRIBUTING.md; verify regression coverage includes existing speed/load speech and supported stored settings, and record results and any unavailable host checks.
- [ ] 5.2 Validate spoken wording/order and shared audio behavior on affected native hosts, using a scoped physical checkpoint only where changed native playback or background behavior cannot be established in software; keep detailed evidence local and report actual platform coverage and any required unavailable hardware.
- [ ] 5.3 Run strict OpenSpec validation and `git diff --check`, review the final implementation against every scenario, and report completed checks with material coverage limits; verify all planning artifacts and delivered behavior agree before marking tasks complete.
