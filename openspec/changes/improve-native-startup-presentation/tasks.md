## 1. Establish the acceptance baseline

- [x] 1.1 Review the existing candidate diff against proposal/design/specs and record retained work, gaps and affected ownership boundaries in an acceptance report; verify every delta requirement has a mapped test or visual check before marking implementation tasks complete.
- [ ] 1.2 Capture representative pre-change light/dark cold-launch transitions and first-usable-UI timings on the two connected physical devices from the baseline revision without discarding candidate edits; record OS/WebView versions, revision, at least three runs and retained capture locations for comparison.

## 2. Complete startup presentation

- [x] 2.1 Review/complete iOS adaptive launch resources, native/WebView canvas and explicit startup-failure handling; verify simulator/device packaging and focused observations for normal absence, genuine failure, successful reactivation and background/foreground without transient error text.
- [ ] 2.2 Review/complete Android light/night window backgrounds, API 31+ platform splash styling and WebView/container behavior; verify packaged resources and launches on the connected Android phone without a custom splash activity or fixed delay; additional Android versions/emulator visual checks are out of scope.
- [x] 2.3 Review/complete macOS adaptive window/WebView presentation and move appearance synchronization into the theme-owned host area; verify through existing builds, automated appearance tests and ownership review; manual macOS startup, close/reopen and renderer recordings are out of scope.
- [x] 2.4 Review/complete the shared early HTML canvas and theme hint path; verify computed canvas before application modules load and after authoritative reconciliation in light/dark without platform-specific frontend branches.

## 3. Add focused behavioral regressions

- [x] 3.1 Extend the existing frontend/browser harness to execute actual bootstrap HTML with delayed modules, valid system/light/dark hints, missing/invalid/throwing storage and stale hints; verify early computed backgrounds and authoritative reconciliation, retaining existing theme tests.
- [x] 3.2 Complete applicable lifecycle/presentation checks on the connected devices using existing harnesses: iOS background/foreground, Android retained resume, grace-period disposal and Activity recreation, plus trim-memory where supported by device tooling; retain automated continuity/resource checks. Additional manual macOS or simulator/emulator lifecycle checks are out of scope.
- [x] 3.3 Retain executable delay/failure regressions and exercise error/recovery paths safely reproducible on the connected devices, including launch without a BLE connection; verify errors remain reachable with no startup gate or transient reconstruction error. Record fault injection requiring unavailable tooling as out of scope; retain earlier iOS bind-failure/recovery evidence as supplemental.
- [x] 3.4 Integrate any new executable acceptance checks into existing test commands and the root verification workflow where needed; verify they actually execute, with assertions in conformance/tests and target adapters limited to OS operations, without simple-command wrapper scripts.

## 4. Record visual acceptance

- [ ] 4.1 Record and inspect cold-launch smoke coverage on the currently connected physical iOS and Android devices from the design, including one clean-install launch to rule out stale launch assets; retain captures and mark each required device check pass/fail/unverified in the acceptance report.
- [ ] 4.2 Record both connected devices’ system light/dark × app system/light/dark combinations; repeat representative system and opposite-theme launches at least three times, inspect intermediate frames and compare baseline timings for unintended flashes or added dwell.
- [ ] 4.3 Record light/dark lifecycle transitions, safe-area/system-bar appearance and applicable failure/recovery paths on each connected device; verify no extra splash, temporary loading/error text or obscured failure, identifying intentional theme changes and OS-owned regions separately.
- [ ] 4.4 Finalize the acceptance report with requirement-to-evidence mapping, environment/revision identities, retained capture links and timing observations; identify all other devices/versions, simulator/emulator visual matrices and manual macOS checks as out of scope, while missing required connected-device evidence and failures remain incomplete and no physical BLE checkpoint is added for this presentation-only scope.

## 5. Verification and completion gate

- [x] 5.1 Diagnose the previously observed Android conformance failures and existing frontend lint findings against the final candidate; verify focused checks pass or land necessary independent repairs without weakening assertions, then run the relevant frontend/native tests, formatting, `git diff --check` and `OPENSPEC_TELEMETRY=0 openspec validate improve-native-startup-presentation --strict`.
- [ ] 5.2 Run `just verify` on the final implementation as the existing automated software gate and record a passing result alongside complete visual evidence for the two connected devices; only then mark acceptance complete and proceed to spec synchronization/archive in the final workflow commit.
