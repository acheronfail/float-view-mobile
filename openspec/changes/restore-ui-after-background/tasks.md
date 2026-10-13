# Tasks

## 1. Typed checkpoint contract and process ownership

- [ ] 1.1 Define version-1 route/Live/surface unions, bounds and `uiState.get`/`uiState.set` in the frontend contract; implement equivalent mock behavior and test valid round trips, null clearing, unknown fields/versions and incompatible combinations with focused frontend unit tests and type checking.
- [ ] 1.2 Implement one nullable checkpoint in the existing Rust service and authenticated shared-server dispatch/projection; add focused Rust tests for atomic rejection, encoded UTF-8 bounds, no state events/revision or product effects, listener stop/start retention and fresh-process clearing, then run affected crate tests.
- [ ] 1.3 Add canonical conformance scenarios for checkpoint authentication, get/set/clear, malformed and oversized records, listener recreation and process lifetime; verify the focused scenarios through the direct Rust host and affected Apple/Android integration boundaries using existing C/JNI paths.
- [ ] 1.4 Document the contract's memory-only lifetime and exact supported shape/bounds in the existing backend/frontend ownership documentation; verify links and `git diff --check`, with no new endpoint, native bridge or dependency.

## 2. Frontend capture and startup ordering

- [ ] 2.1 Add explicit checkpoint get/set handling to the existing frontend backend client/state owner and a small app-owned capture coordinator; test deduplication, one in-flight request/one latest pending record, accepted submission order, failure retention and disposal with focused unit tests.
- [ ] 2.2 Gate application startup on fresh state and checkpoint resolution, suppress initial default capture/Home scan/recovered-connection redirects, and resolve eligible routes and return context; test no-checkpoint fallback, active connection with Sessions/Settings/session destinations, missing targets and request failure/retry.
- [ ] 2.3 Scope restoration to one frontend startup, cancel pending restoration on user navigation and prevent late loads from reopening old UI; verify transport reconnect preserves current UI and explicit navigation exits failed restoration using focused mock/browser coverage.

## 3. Supported presentation owners

- [ ] 3.1 Wire Live normal/detailed mode and its allowlisted actions/charts/BMS/Refloat/Tunes/Board Profile surfaces into explicit capture/restore fields, including target checks; test each supported surface open/close round trip, changed-device rejection and current-state eligibility, and remove the superseded persistence TODO.
- [ ] 3.2 Wire board-history actions and configuration surfaces into the same single checkpoint; verify saved-board removal fallback and fresh authoritative editor initialization with focused component/mock coverage.
- [ ] 3.3 Verify no confirmation, draft, selection/viewport, live-chart samples or operation flags are restored and no mutation is replayed; cover reopening editors, authoritative flywheel precedence and background during a pending action in frontend mock/browser tests.
- [ ] 3.4 Add bounded debug-only capture/restore outcomes and document restoration limits in the existing frontend README; inspect diagnostic output for IDs/addresses/tokens/payloads, verify absence of per-sample capture/logging, and run documentation checks.

## 4. Affected-consumer acceptance

- [ ] 4.1 Run relevant frontend unit/browser checks, type/lint checks and build, affected Rust crate/consumer checks and focused canonical conformance; record actual commands/results and resolve failures without requiring unrelated full-suite packaging.
- [ ] 4.2 Perform a scoped physical iOS background/foreground checkpoint from session detail and an ordinary Live panel, verifying restored destination/mode/surface, actual WebView/server teardown, no checkpoint background activity, and native connection/recording continuity where permitted; record evidence or explicitly report unavailable required hardware without claiming software establishes OS lifecycle behavior.

## Workflow follow-up

- Commit the completed planning artifacts before implementation; keep each verified implementation slice buildable and include its task updates.
- After affected-consumer acceptance, sync these delta specs and archive the change together in one final commit.
