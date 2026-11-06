# Tasks

## 1. Native lifecycle

- [x] 1.1 Add bounded reconnect lifecycle decisions and focused Swift tests for expiry, recovery, foreground transitions and repeated snapshots.
- [x] 1.2 Integrate UIKit execution allowance and immediate ActivityKit cleanup; verify iOS simulator compilation and review expiration/replacement ordering.

## 2. Verification

- [x] 2.1 Run focused Apple tests, formatting, diff and OpenSpec validation; record results here.
- [x] 2.2 Verify on a physical iPhone: locked disconnect lasting beyond grace, recovery within grace, return to foreground, later background reconnection and backend reconnect independence. Preserve detailed evidence locally.

## Workflow follow-up

- Sync specs and archive after the required physical checkpoint passes.

## Evidence

- 2026-11-06: `swift test --package-path backends/apple --scratch-path /tmp/float-view-swift-build --filter LiveActivity` passed: 21 tests, zero failures (outside sandbox for compiler-cache access).
- iOS Debug arm64 simulator build passed using `xcodebuild` with `ARCHS=arm64 ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO` (outside sandbox for Xcode services). The initial universal simulator build lacked the pinned x86_64 Rust target; arm64 is installed and verified.
- SwiftFormat lint for the four changed Swift files and `git diff --check` passed. Active change validation returned no issues.
- 2026-11-06: User installed the initial five-second build and reported all five physical scenarios passed: locked disconnect cleanup, within-grace recovery, later background recovery with foreground presentation restoration, foreground during grace, and backgrounding an already-reconnecting app. Detailed user-reported evidence is retained locally under `.local/physical-testing/`. OS execution denial/early expiration were not separately forced; abrupt termination remains outside the guarantee.

- Subsequent user-requested tuning increased grace to ten seconds. All 21 focused Live Activity tests passed again; formatting and diff checks passed. Ten-second timing and recovery between five and ten seconds have not been physically retested.
