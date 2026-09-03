## 1. Logical subgroup navigation

- [ ] 1.1 Replace the page-level filtered projection with ordered logical sections keyed by exact group/subgroup identity, merging all matching transport chunks while retaining complete schema fields for validation and writes.
- [ ] 1.2 Add controller tests for chunk aggregation, schema order, duplicate-label qualification, selected-section search matching, empty results, and preservation of complete-editor dirty and validation state.

## 2. On-demand field help

- [ ] 2.1 Add a dependency-free informational modal with ordinary dialog semantics, one Done action, Escape and backdrop dismissal, focus containment, and restoration to its trigger.
- [ ] 2.2 Replace inline field descriptions and description references with a `(?)` button beside every presented field label, using the established primary-pointer and guarded keyboard/assistive activation paths and retaining inline validation errors.
- [ ] 2.3 Add focused tests for help fallback content, accessible button naming, single activation, dialog semantics, dismissal, focus behavior, and the persistent-startup-default label.

## 3. Compact editor presentation

- [ ] 3.1 Add the labelled native subgroup selector, first-section and stale-selection fallback behavior, duplicate option labels, and exactly-one-section rendering to the generated editor.
- [ ] 3.2 Scope the search input and empty-result presentation to the selected section without changing selection, and retain selection across current/default value reloads.
- [ ] 3.3 Add editor tests covering section navigation, multi-page subgroup rendering, search stability, edits across navigation, hidden invalid-field write gating, and selection lifecycle.

## 4. Verification

- [ ] 4.1 Run the focused Refloat configuration frontend tests and fix any regressions.
- [ ] 4.2 Run `just verify` and resolve all software-gate failures.
