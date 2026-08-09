## 1. Establish sibling host roots

- [x] 1.1 Move the complete macOS host source and resource tree from `Sources/FloatViewMac` to the sibling `macos` root.
- [x] 1.2 Configure the existing `FloatViewMac` SwiftPM executable target to build from the explicit macOS host path.

## 2. Update repository ownership references

- [x] 2.1 Update build, development, conformance, resource-copy, and icon-generation references to the new macOS root.
- [x] 2.2 Update Apple backend documentation and layout audits to require sibling `ios` and `macos` host roots and reject the superseded path.

## 3. Verification

- [x] 3.1 Run focused SwiftPM macOS build/tests and the iOS simulator build.
- [x] 3.2 Run `just verify` and resolve any build, resource, audit, or canonical conformance regression.
