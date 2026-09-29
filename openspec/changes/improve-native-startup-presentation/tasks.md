## 1. Establish the acceptance baseline

- [ ] 1.1 Review the existing candidate diff against proposal/design/specs and record retained work, gaps and affected ownership boundaries in an acceptance report; verify every delta requirement has a mapped test or visual check before marking implementation tasks complete.
- [ ] 1.2 Capture representative pre-change light/dark cold-launch transitions and first-usable-UI timings from the baseline revision without discarding candidate edits; record OS/WebView versions, revision, at least three runs and retained capture locations for comparison.

## 2. Complete startup presentation

- [ ] 2.1 Review/complete iOS adaptive launch resources, native/WebView canvas and explicit startup-failure handling; verify simulator/device packaging and focused observations for normal absence, genuine failure, successful reactivation and background/foreground without transient error text.
- [ ] 2.2 Review/complete Android light/night window backgrounds, API 31+ platform splash styling and WebView/container behavior; verify packaged resources and launches on API 29/30, 31 and current target 36 without a custom splash activity or fixed delay.
- [ ] 2.3 Review/complete macOS adaptive window/WebView presentation and move appearance synchronization into the theme-owned host area; verify initial and later theme effects, explicit-to-system, OS changes and close/reopen with the same service owner.
- [ ] 2.4 Review/complete the shared early HTML canvas and theme hint path; verify computed canvas before application modules load and after authoritative reconciliation in light/dark without platform-specific frontend branches.

## 3. Add focused behavioral regressions

- [ ] 3.1 Extend the existing frontend/browser harness to execute actual bootstrap HTML with delayed modules, valid system/light/dark hints, missing/invalid/throwing storage and stale hints; verify early computed backgrounds and authoritative reconciliation, retaining existing theme tests.
- [ ] 3.2 Add missing native lifecycle/presentation assertions using existing host/conformance harnesses: iOS foreground/recovery, Android retained resume, grace-period disposal, trim-memory/activity recreation, and macOS reopen/recovery; verify appearance, native session continuity and no growing observers/views across repeated recreation.
- [ ] 3.3 Exercise delayed/failed navigation, backend unavailability and unavailable Bluetooth, plus iOS startup failure followed by successful activation; verify existing errors stay visible/reachable and no new startup gate, retry loop or transient error during normal reconstruction.
- [ ] 3.4 Integrate any new executable acceptance checks into existing test commands and the root verification workflow where needed; verify they actually execute, with assertions in conformance/tests and target adapters limited to OS operations, without simple-command wrapper scripts.

## 4. Record visual acceptance

- [ ] 4.1 Record and inspect minimum/current OS cold-launch smoke coverage from the design, including one clean-install launch to rule out stale launch assets; retain captures and mark each version pass/fail/unverified in the acceptance report.
- [ ] 4.2 Record current-platform system light/dark × app system/light/dark combinations; repeat representative system and opposite-theme launches at least three times, inspect intermediate frames and compare baseline timings for unintended flashes or added dwell.
- [ ] 4.3 Record light/dark lifecycle transitions, safe-area/system-bar appearance and applicable failure/recovery paths on each current platform; verify no extra splash, temporary loading/error text or obscured failure, identifying intentional theme changes and OS-owned regions separately.
- [ ] 4.4 Finalize the acceptance report with requirement-to-evidence mapping, environment/revision identities, retained capture links and timing observations; verify missing runtimes and failures remain incomplete and no physical BLE checkpoint is added for this presentation-only scope.

## 5. Verification and completion gate

- [ ] 5.1 Diagnose the previously observed Android conformance failures and existing frontend lint findings against the final candidate; verify focused checks pass or land necessary independent repairs without weakening assertions, then run the relevant frontend/native tests, formatting, `git diff --check` and `OPENSPEC_TELEMETRY=0 openspec validate improve-native-startup-presentation --strict`.
- [ ] 5.2 Run `just verify` on the final implementation with the required runtimes available and record a passing result alongside complete visual evidence; only then mark acceptance complete and proceed to spec synchronization/archive in the final workflow commit.
