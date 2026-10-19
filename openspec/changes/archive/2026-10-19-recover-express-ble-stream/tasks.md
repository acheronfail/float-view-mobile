# Tasks

## 1. Shared framed-stream recovery

- [x] 1.1 Implement bounded recovery and preserve valid packets around corruption; verify synthetic fragmentation, nested-frame, corruption, false-header and resource-bound tests in `float-view-vesc-frame`.
- [x] 1.2 Verify runtime dispatch with valid and malformed packets in one input and run affected Rust consumer tests, formatting and Clippy; replay the private reconnect capture without publishing raw packets.

## 2. Android negotiated transfer sizes

- [x] 2.1 Negotiate connection-qualified MTU before service discovery, retain default-MTU failure fallback and remove fixed write ceilings; verify focused setup-state unit tests and Android compilation/lint.
- [x] 2.2 Build a test APK and attempt data-preserving deployment; verify the connected Android/Express setup, negotiated payload, telemetry and reconnect or document concrete tooling/device limitations and retain evidence locally.

## Workflow follow-up

- Sync delta specs and archive with a final commit after relevant verification.
