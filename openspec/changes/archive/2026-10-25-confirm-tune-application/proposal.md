# Proposal

## Why

Tapping a tune currently writes immediately when idle and only warns when riding. Users need to review the settings that will change and explicitly confirm every application.

## What Changes

- Always open a confirmation for compatible tunes, including already matching tunes.
- Show changed settings from current board values to saved tune values, reusing the compact configuration-write comparison presentation and bounded scrolling.
- Keep the riding warning and live safety gating, and preserve the preview and error on failed application.
- Refresh current values before preview and confirmation; changed values require review again.
- Remove the immediate idle-application path and duplicated comparison markup.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-configuration`: Require an explicit reviewed confirmation before frontend tune application.

## Impact

Frontend Tunes UI, shared compact configuration comparison component, browser tests, and frontend documentation. No API, backend admission, firmware, dependency, or persistence changes. Non-goals: editing tune values, tune migrations, and changes to riding eligibility.
