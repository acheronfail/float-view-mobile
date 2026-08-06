## Context

Apple and Android each implement the same connection state machine by hand. Once BLE UART notifications are enabled, both immediately enqueue one direct `COMM_FW_VERSION` request and arm a ten-second identification deadline. The request scheduler removes a timed-out request, but the application ignores that outcome; the outer deadline then fails the connection. A manual retry creates a fresh scheduler and protocol generation and sends the same safe read again, which explains why the second connection often succeeds on a healthy real board.

The symptom occurs on both CoreBluetooth and Android GATT, so transport-specific reconnect behavior is not an adequate shared fix. VESC identification reads are idempotent, request/response matching is serialized, and both implementations already reject old scheduler generations.

## Goals / Non-Goals

**Goals:**

- Recover transparently when the first direct firmware response is dropped during the notification-to-UART handoff.
- Keep the attempt bounded by the existing overall identification deadline and current connection generation.
- Preserve exact Apple/Android behavior and regression coverage.
- Make development logs reveal transport setup, identification stage, attempt, request outcome, and final issue code without private identifiers or telemetry.
- Ensure routine diagnostics are absent from production/release console logging.

**Non-Goals:**

- Retrying incompatible identities, configured CAN-route failures, every CAN inventory probe, or mutating commands.
- Changing the public backend contract, persistence, frontend retry interaction, or reconnect policy.
- Adding dependencies or long-lived diagnostic storage.

## Decisions

### Retry the first direct firmware read at the application layer

Each backend will give the first direct firmware request a short bounded response window. If that request times out while the same connection and identification generation remain active, it will immediately enqueue the same idempotent request once more. The existing overall identification timer remains authoritative, so the retry does not extend the connection attempt indefinitely.

This is preferred to reconnecting BLE because the UART profile is already proven and a reconnect repeats substantially more state. It is preferred to a fixed post-notification sleep because a delay always penalizes healthy connections and still cannot prove readiness. It is preferred to scheduler-wide automatic retries because telemetry, CAN enumeration, and mutations have different safety and latency semantics.

### Preserve generation and serialized response matching

The retry uses the current VESC request generation and is scheduled only from the timed-out request completion. A response delayed from the first send is valid for the identical second read and may satisfy it; responses from an older connection generation remain rejected. The retry helper checks active device, connection state, discovery generation, and request generation before sending.

### Emit structured bounded diagnostics through existing sinks

The application state machine will emit stable key-value diagnostic strings for identification stages and outcomes, including route, attempt, and failure code. BLE adapters will report setup callback outcomes and bounded platform error domain/code or numeric status. Diagnostics will exclude device IDs, addresses, launch tokens, raw packets, firmware UUIDs, and telemetry values.

Apple hosts will print these events only under `#if DEBUG`. Android will route adapter and application diagnostic strings only when the installed application is debuggable, replacing unconditional warnings for expected BLE setup failures. Unexpected throwable reporting remains separate.

### Test the behavior at state-machine and conformance levels

Native unit tests will simulate a timed-out first firmware request and assert a second write occurs without a second BLE connect, followed by normal readiness. They will also prove the retry stops after one attempt and is suppressed after generation/state changes. Canonical conformance will exercise equivalent externally observable recovery for both backends where its software peripheral can model a dropped first response.

## Risks / Trade-offs

- [A delayed first response satisfies the second identical request] → This is safe because firmware discovery is an idempotent read with the same route and generation; any later duplicate is unmatched and ignored.
- [The shorter first-attempt window may create unnecessary duplicate reads on a slow board] → Keep the window conservative, retry only once, and retain the existing total identification bound.
- [Parallel implementations drift] → Use equivalent constants, diagnostic vocabulary, and mirrored tests in Apple and Android.
- [More logging leaks production data] → Keep fields allowlisted and privacy-safe, with host-level debug/debbugable gating and no packet contents.

## Migration Plan

No data or API migration is required. The change can be rolled back by removing the retry helper and diagnostics; persisted sessions and board profiles are unaffected.

## Open Questions

None. Physical-device verification remains the explicit BLE checkpoint after software verification.
