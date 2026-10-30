# Tasks

## 1. Shared selections and presentation

- [x] 1.1 Add net Wh/Ah identifiers to the frontend and Rust catalogues; verify persistence using store tests and canonical conformance.
- [x] 1.2 Derive and expose both readings in Live and Custom pickers with missing-value handling, units, precision and sign colours; verify with frontend tests.

## 2. Verification

- [x] 2.1 Run relevant frontend checks, product-store and consumer checks, mock/direct conformance and OpenSpec validation. Record results and coverage limits.

## Verification results

- Focused frontend tests: 33 passed across live values, widget catalogue and both dashboard helpers/validators.
- Frontend Svelte/type checks, production build, changed-file ESLint and formatting passed.
- Product-store and loopback-server crate tests passed, including reopen persistence for global slots, board slots and Custom widgets. Rust formatting and warnings-denied product-store Clippy passed.
- Canonical mock and direct conformance passed with both identifiers in saved Live slots and Custom widgets. Conformance TypeScript check passed.
- Chromium and WebKit: existing Custom dashboard suite and extended live-value search suite passed. The latter checks touch/keyboard activation, net selection reload and Custom numeric rendering (693.0 Wh and 15.70 Ah).
- Strict OpenSpec validation and diff whitespace checks passed.
- Local-server tests required execution outside the sandbox. Initial sandbox bind failures were resolved on rerun. No native adapters or bindings changed; shared runtime and transport coverage exercises the affected boundary used by both hosts, without native packaging or simulator runs.
