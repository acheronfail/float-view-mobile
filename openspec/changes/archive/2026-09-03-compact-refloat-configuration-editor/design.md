## Context

The generated Refloat editor eagerly loads the complete bounded schema as transport pages and currently renders every page in one vertical grid. Each transport page identifies its schema group and subgroup, but a subgroup can occupy multiple pages because both native backends chunk its fields at 32 entries. Field descriptions are normalized, sanitized plain text and are currently rendered below every input.

This change is confined to `frontend/src/lib/features/connection/`. It must preserve the complete draft and write model, the existing schema order and visibility, current/default loading, validation, unsaved-change protection, and the primary-pointer plus guarded click behavior required for frequently updating WebView UI.

## Goals / Non-Goals

**Goals:**

- Present one schema subgroup at a time through a compact native select control.
- Treat backend chunks for the same group and subgroup as one logical section.
- Keep search useful and predictable within the selected section.
- Replace persistent description text with accessible, on-demand field help.
- Retain complete-configuration validation, dirty tracking, confirmation, and writes across hidden sections.

**Non-Goals:**

- Changing the wire contract, backend pagination, normalized schema, native services, or write safety rules.
- Exposing XML separator rows that are not present in the frontend contract.
- Filtering or reclassifying schema-provided groups, including internal groups.
- Redesigning Tunes or the surrounding bottom sheet.

## Decisions

### Derive logical sections from transport pages

The frontend controller will derive an ordered logical-section projection by grouping pages on the exact `(group, subgroup)` pair and concatenating their fields in received order. The first occurrence establishes section order. The projection will not alter the underlying `pages`, `fields`, draft, or write payload.

This avoids coupling visible navigation to the backend's 32-field transport bound. Rendering each transport page as an option was rejected because large schema subgroups such as LEDs would appear as indistinguishable duplicate options. Changing backend pagination was rejected because this is presentation behavior and requires no contract change.

### Use subgroup-oriented native selection

The editor will expose a labelled native `select` and render exactly one selected logical section. Options use the subgroup name when unique; if the same subgroup label occurs more than once, the visible label is qualified with its group. Stable selection identity is based on the exact group/subgroup pair rather than the display label.

The first available section is selected after initial schema loading. Value reloads retain selection because they do not change the schema. If schema replacement makes the selected identity unavailable, selection falls back to the first available section. Tabs were rejected because the number and names of subgroups are firmware-defined and can overflow narrow or touch layouts.

### Scope search to the selected section

Search filters the fields of the selected logical section only, including matches against its group, subgroup, field name, label, and description. It never changes the selected section. An empty result is shown explicitly within that section.

Global search was rejected because hiding and automatically changing the section selector while typing would make navigation unstable. The normalized description remains searchable even though it is no longer shown inline.

### Add a dedicated informational help dialog

Every field will place a `type="button"` control displaying `(?)` beside its visible label. Its accessible name identifies the field. Activating it opens one reusable informational modal whose title is the presented field label and whose body is the normalized description, falling back to the field name when the description is empty.

The modal uses ordinary `dialog` semantics because requested help is non-urgent. It has a single `Done` action, supports Escape and backdrop dismissal, traps focus while open, and restores focus to the triggering button. The help button uses the established primary-pointer activation and guarded keyboard/assistive click paths without double activation. Extending the confirmation dialog was rejected because help has no confirm/cancel decision and should not use alert-dialog semantics.

Inline schema descriptions and their `aria-describedby` references will be removed. Inline validation errors remain adjacent to their inputs and retain live error semantics because they communicate actionable state rather than optional help.

### Keep editor-wide state independent of visibility

Only rendering and search are section-scoped. Validation continues to cover every schema field, dirty counts and reset cover the complete draft, and write confirmation continues to list the complete bounded diff. This prevents a hidden invalid or changed field from being silently excluded from a complete configuration write.

No new dependency is needed; the modal follows the repository's existing focus and activation patterns in a small dedicated component or equivalent local implementation.

## Risks / Trade-offs

- **[Hidden changes or errors are outside the selected section]** → Keep the global changed count and complete validation/write gating; tests will verify edits survive navigation and hidden invalid fields still block writes.
- **[Duplicate subgroup names make options ambiguous]** → Qualify only duplicate visible names with their group while using the exact pair as identity.
- **[Schema chunks could be exposed as duplicate pages]** → Aggregate every chunk with the same pair before producing navigation options.
- **[Help buttons add many focusable controls]** → Keep each control small visually but preserve a touch-appropriate activation area and an explicit accessible name.
- **[Removing inline descriptions reduces passive discoverability]** → Place the consistent `(?)` affordance beside every label and keep descriptions searchable.
- **[Selected-section search may differ from the old global search]** → Label the search scope clearly and never mutate navigation as a side effect of typing.

## Migration Plan

Replace the old presentation projections and markup in one frontend change, update focused component/controller tests, and run `just verify`. There is no data or wire migration. Rollback consists of reverting the frontend change; native backends and stored data remain compatible.

## Open Questions

None.
