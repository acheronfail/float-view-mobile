# Frontend library structure

Organize by ownership. Keep a feature's components, controllers, calculations and
tests together. The shared frontend consumes the authenticated
[backend contract](backend/contract.ts) on every host; platform decisions belong
to the shared service and native adapters.

## Owners

| Directory              | Responsibility                                                          |
| ---------------------- | ----------------------------------------------------------------------- |
| `app/`                 | Persistent shell, navigation, theme and adaptive control placement      |
| `backend/`             | Bootstrap, transport, authoritative state and contract                  |
| `features/connection/` | Discovery, connection, live views, dashboard editing and Refloat panels |
| `features/faults/`     | Fault labels, history loading and shared fault list                     |
| `features/sessions/`   | Session lists, review queries, charts, maps and export                  |
| `features/boards/`     | Saved boards, Board Profile and appearance editors                      |
| `features/settings/`   | Application-wide settings                                               |
| `ui/`                  | Domain-independent presentation reused across features                  |

Route files are thin adapters for parameters, navigation and their owning feature.
A feature must not import another feature's private files. Promote code to `ui/`
only after domain-independent reuse is demonstrated.

## Files and imports

- Keep feature directories flat by default. Use a PascalCase component directory
  only when it has meaningful private children.
- Keep component-private props and types in `Thing.svelte`.
- Use `thing.svelte.ts` for rune-based controllers and specific names such as
  `chart.ts` for pure logic.
- Colocate `*.spec.ts` with the code it tests.
- Use relative imports within an owner and `$lib/...` across ownership boundaries.
- Avoid catch-all `components/`, `controllers/`, `stores/`, `types/`, `utils/` and
  `helpers/` directories. Do not add barrel files without a useful public boundary.

## Entry points

| Work                                       | Start here                                                                                                                         |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| Live value selection and formatting        | [live-values.ts](features/connection/live-values.ts)                                                                               |
| Custom dashboard editor and gestures       | [CustomView.svelte](features/connection/CustomView.svelte), [dashboard-gesture.ts](features/connection/dashboard-gesture.ts)       |
| Dashboard widget choices                   | [widget-catalog.ts](features/connection/widget-catalog.ts)                                                                         |
| Warnings shared by all live views          | [LiveWarnings.svelte](features/connection/LiveWarnings.svelte)                                                                     |
| Refloat editor state                       | [refloat-configuration.svelte.ts](features/connection/refloat-configuration.svelte.ts)                                             |
| Tune library                               | [RefloatTunes.svelte](features/connection/RefloatTunes.svelte)                                                                     |
| Session field selection and recording time | [session-fields.ts](features/sessions/session-fields.ts), [recording-time.ts](features/sessions/recording-time.ts)                 |
| Board artwork and parts picker             | [BoardIcon.svelte](features/boards/BoardIcon.svelte), [BoardAppearanceEditor.svelte](features/boards/BoardAppearanceEditor.svelte) |

Use published availability and write-block reasons; do not infer firmware support,
write safety, battery estimates or drive-load policy in presentation code. Missing
values stay unavailable. Use existing backend operations for durable product
state; feature owners manage transient UI state. Restoration must never replay mutations.

## Shared interaction and styling

Follow the [interaction policy](../../../AGENTS.md#frontend-interaction). Reuse
[activation.ts](ui/activation.ts) for guarded pointer/keyboard activation,
[back-dismissal.ts](ui/back-dismissal.ts) for Escape/native Back, and
[motion.ts](ui/motion.ts) with the tokens in [app.css](../app.css). Dismissal must
call the surface owner's existing save/cancel handler, preserving busy/error guards.

Use native button semantics, accessible names, modal focus isolation/restoration
and reduced-motion support. Outgoing surfaces become inert before fading;
animation must not delay commands, replay on telemetry refresh or remount charts.
Use semantic theme tokens rather than a second palette. Host layout facts affect
placement only; they are not a native product-operation channel.

For product requirements, see [OpenSpec](../../../openspec/specs/); for checks and
browser prerequisites, see the [development reference](../../../docs/development.md#verification).
Implementation constants and detailed behavior live with their source and tests.
