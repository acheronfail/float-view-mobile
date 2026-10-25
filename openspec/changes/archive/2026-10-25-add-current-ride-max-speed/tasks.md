# Tasks

## 1. Shared ride peak

- [x] 1.1 Track, restore and project current-ride maximum speed through the shared runtime used by both hosts; verify peaks, missing speed, reconnect, resume and new-ride tests.

## 2. Widget reading

- [x] 2.1 Add the contract/catalog identifier, both persistence validators, numeric units/ranges, mock behavior and documentation; verify focused frontend and product-store tests.

## 3. Integration verification

- [x] 3.1 Add canonical peak and saved-widget assertions; run affected mock/direct conformance, frontend checks/build and relevant browser interaction coverage, Rust consumer tests, formatting/Clippy and strict OpenSpec validation.

## Verification notes

- Passed focused Rust telemetry/runtime/coordinator/store/server/recording tests, saved ride restart coverage, and Apple FFI/Android JNI binding tests.
- Passed 28 focused frontend tests, Svelte/type checks, production build, ESLint, formatting, warnings-denied Clippy and strict OpenSpec validation.
- Passed canonical mock/direct contract and ride-continuity checks; Chromium covered selection, saving, recreation and imperial display in both dashboards.
- WebKit browser coverage could not run: the host lacks its ICU 76, libxml2, libmanette and enchant runtime libraries. Native packaging/device UI was not run; the hosts consume the tested shared projection and bindings unchanged.
