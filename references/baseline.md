# R0 baseline report

Captured: 2026-07-28.

Recovery revision: `fae1e4efa436314593c80059c93df88e9bab3640`.

Host:

- macOS 15.7.7, arm64;
- Xcode 26.3 / Swift 6.2.4;
- Node 22.12.0 / npm 10.9.0;
- JDK 21.0.11;
- just 1.55.1.

Observed pre-reset state:

- API schema generation/check scripts exist and generated Swift contract smoke
  compilation passes.
- The development mock implements only `system.hello` and
  `system.getSnapshot`.
- Android/iOS product code remains coupled from BLE/protocol callbacks directly
  to the legacy WebSocket server.
- Native logging and native application-service tests do not exist.
- `tsx` CLI execution fails inside the managed sandbox because its IPC socket is
  denied; repository scripts will use Node's `--import tsx` loader instead.
- Gradle's default user cache is outside the writable workspace; verification
  will use a repository-local or temporary Gradle user home.
- No iOS test target or macOS host exists.
- Physical VESC hardware and iOS/Android devices are not assumed connected
  during R0-R5; R6-R8 own hardware evidence.

Known failures are not converted to passing placeholders. Each cumulative
verification target must fail if a required software assertion is skipped.
