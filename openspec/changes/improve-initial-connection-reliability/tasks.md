## 1. Connection recovery

- [ ] 1.1 Add a generation-safe one-time initial direct-firmware retry to the Apple application service
- [ ] 1.2 Mirror the bounded initial direct-firmware retry in the Android application service

## 2. Diagnostics

- [ ] 2.1 Add bounded privacy-safe BLE setup and identification diagnostics on Apple
- [ ] 2.2 Add equivalent Android diagnostics and gate routine adapter/application console output to debuggable builds

## 3. Regression coverage

- [ ] 3.1 Add Apple tests for successful recovery, retry exhaustion, and stale-generation suppression
- [ ] 3.2 Add equivalent Android tests for successful recovery, retry exhaustion, and stale-generation suppression
- [ ] 3.3 Extend the canonical software connection scenario to cover a dropped first firmware response on both backends

## 4. Verification

- [ ] 4.1 Run focused Apple, Android, and conformance checks for the changed connection paths
- [ ] 4.2 Run `just verify` and resolve all software-gate failures
