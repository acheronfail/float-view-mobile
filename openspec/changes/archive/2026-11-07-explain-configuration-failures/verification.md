# Verification

Implemented against source base `fae17623447fb2f5ba540ff3308a3684a69a37ed`.

- Final backend/server regression: `cargo test --locked --offline -p float-view-backend-service -p float-view-loopback-server` passed all 160 tests, including 144 service tests, the BLE transport boundary and authenticated server diagnostic export/error coverage. The initial sandbox run could not bind test sockets; the final run outside the sandbox passed.
- Affected runtime/coordinator/native coverage passed: application runtime (20), coordinator unit/integration suites (51), Android JNI (1), Apple FFI/ABI (8), and shared host (1). The initial combined command stopped only at the sandboxed server transport tests above; those were rerun successfully. No native ABI or public API changes were introduced.
- `cargo clippy --locked --offline --all-targets` passed with `-D warnings` for backend-service, connection-application-coordinator, application-runtime, loopback-server, application-runtime-host, application-runtime-apple-ffi and application-runtime-android-jni.
- `FLOAT_VIEW_CONFORMANCE_SCENARIOS=diagnostics npm run direct` passed against the production shared service, revision 3, outside the socket-restricted sandbox.
- Rust formatting, changed Markdown formatting, `git diff --check`, strict change validation and canonical support-diagnostics validation passed without issues.

Regression evidence covers actual service rejections for package/schema/signature/revision mismatch and unconfirmed IMU authority; busy request correlation; service timeout handling; connection cancellation; late-coalesced sanitized ZIP contents; maximum counter sizes; incident clipping; audited prerelease metadata; and independent report-local operation namespaces. Coordinator tests preserve ACK/readback stage correlation and verify timeout-authority expiry can be diagnosed at the attempt clock before applying a command.

Write authorization, public error codes and product persistence remain unchanged. The original ZIP cannot establish the user's exact failed precondition; the added evidence applies to future captures. Configuration values, arbitrary identity text, fingerprints, raw packets and raw errors remain excluded. No commits or online changes were made.
