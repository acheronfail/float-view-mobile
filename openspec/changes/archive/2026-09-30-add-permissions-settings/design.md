## Context

See proposal.md for motivation. SettingsView currently has no permission inventory. BluetoothState exposes scan/connect grants but no recovery action. LocationState mixes the recording preference and provider authorization; AndroidLocationProvider collapses coarse/fine access and uses a single requested flag. CoreLocationAdapter requires Always for capture that may start while locked; When In Use remains foreground-only. Existing native UI effects complete on dispatch, not necessarily when the user chooses a grant.

## Goals / Non-Goals

Keep one frontend and one Rust application owner. Native hosts report closed OS facts and perform permission presentation; Rust owns inventory, public projection, validation, recovery selection, and bounded pending operations. Inspecting or requesting access must not enable recording, start scanning, or start capture. Do not expand the app's authorization scope or change ride-capture policy.

## Decisions

### One explicit permission inventory

Add mandatory `permissions` to complete snapshots. Use a closed inventory of `bluetooth`, `location`, and `backgroundLocation` rows, present only where applicable, capped at three. Each row carries a closed status (notDetermined, granted, limited, denied, restricted, unavailable), access detail (such as approximate/precise and while-in-use/always where known), recovery action (requestPermission, openSettings, or null), feature usage, a bounded explanation, and pending state. Bluetooth groups scan/connect in one user-facing row but exposes both access facts so partial grants are visible. Status expresses OS authorization, independently of feature enablement. Separate service availability facts explain radio-off/location-services-off without overwriting grants. Unknown observations remain unavailable/checking, never optimistically granted.

This avoids frontend platform sniffing and avoids adding arbitrary labels, URLs, or dynamic permission identifiers to native boundaries. Rust projects a closed guidance code and bounded OS-provided option label where needed; frontend owns normal copy. Existing Bluetooth/location feature summaries remain, derived consistently from the same facts.

### Platform matrix

| Platform | Inventory and recovery |
| --- | --- |
| iOS | Bluetooth and Location. Request unresolved access; denied access opens the app Settings URL. Show restricted access with an explanation, without promising an in-app fix. Preserve the existing Always request flow and requirement for rides that begin while locked; show When In Use as foreground-only with an explicit upgrade action, without a separate background-permission row. Report reduced accuracy as limited detail without blocking usable capture. |
| macOS | Bluetooth and Location. Report authorization independently of provider availability. Request unresolved authorization using existing native APIs; open the relevant Privacy & Security pane when possible, with explicit manual navigation guidance otherwise. No mobile background-permission row. |
| Android 29–30 | Location (coarse/fine, also explaining fine-location dependency for BLE scanning), plus separate Background location. Bluetooth connect has no runtime grant on these versions; do not fabricate a requestable Bluetooth row. API 29 can request background separately after foreground; API 30 uses Settings guidance. |
| Android 31+ | Nearby devices/Bluetooth (scan and connect), Location (coarse/fine), and Background location. Request missing nearby grants together because the OS groups them. Request foreground coarse/fine together; never request background in the same operation. |

The Android manifest currently requests no notification permission. Normal install-time permissions, foreground-service declarations, and entitlements are not actionable runtime permission rows. Approximate location remains useful for ride recording; fine location is specifically necessary for older Android BLE scanning. Background location is optional for board-only operation and its action remains unavailable with an explanation until foreground access exists.

Use current native grant checks, resolved request history, rationale eligibility, and restriction information to choose Android retry versus Settings; a false rationale alone does not distinguish a first request from a blocked prompt. Persist only minimal per-permission request history, not grants. Refresh after one-time grants expire or the OS resets access. Show rationale before re-request when the OS recommends it. No automatic retry or automatic settings redirect on denial.

Platform references, checked 2026-09-30:
- [Android runtime permission guidance](https://developer.android.com/training/permissions/requesting): request in context, explain rationale, respect denial.
- [Android background location](https://developer.android.com/develop/sensors-and-location/location/permissions/background): foreground first; API 30+ Settings flow with localized option label and a decline path.
- [Apple app Settings URL](https://developer.apple.com/documentation/uikit/uiapplication/opensettingsurlstring): use the system-provided app Settings destination on iOS.
- [macOS Privacy & Security](https://support.apple.com/guide/mac-help/change-privacy-security-settings-on-mac-mchl211c911f/mac): guide users to the relevant permission controls. Existing pane deep links are best-effort, not a documented stability guarantee.

### Targeted recovery through the shared contract

Add `permissions.recover({ permission, action })`, returning the current permissions projection after native presentation is dispatched or fails. The action must match the latest advertised action; stale actions return invalid_state and cause a refresh, never silently switch from a permission prompt to opening Settings. Unknown identifiers are invalid_request; inapplicable identifiers are invalid_state. A presentation result does not imply a grant. Native authorization callbacks and activation refreshes publish the actual result.

Keep existing bluetooth.requestPermission, location.requestPermission, and location.openSettings for their current scan/live-view callers, sharing recovery decisions and presentation admission rather than retaining parallel policy. Remove the superseded location denied-to-settings shortcut and single requested-flag inference. All entry points use one process-wide presentation slot; duplicates return conflict. No unbounded queue: foreground/activity absence returns unavailable, pending operations have a bounded timeout, and late completion uses existing lifetime/operation identity guards.

Fixed enum OS facts/effects travel through existing C/JNI bindings. Native hosts own platform version checks, permission grants, rationale capability, settings destinations, and UI completions; they do not construct public snapshots. A failed Settings open returns unavailable with actionable manual guidance. The frontend cannot submit a URL. Successful navigation means only that navigation was accepted.

### Refresh and UI

Refresh native facts on startup, permission callbacks, and foreground/activation, even with recording disabled and no board connected. Integrate with existing lifecycle observation and state.get synchronization so a recreated client gets fresh complete monotonic snapshots. Coalesce observations; never poll at telemetry frequency. On failed refresh mark access unknown/unavailable rather than treating a cached grant as current. Returning from external Settings must reconcile revoked access with existing Bluetooth/capture policy.

Place Permissions near Session settings. Rows show purpose, text status, access detail, optional-feature context, and one applicable action. Optional disabled location must not make the app look broken; avoid a global all-green gate or color-only meaning. Disable actions while presentation is pending or transport disconnected; surface recoverable errors. Use native button semantics, primary pointer activation/cancellation, and guarded keyboard/assistive clicks. The view never optimistically marks access granted and never persists permission state.

## Risks / Trade-offs

- OS dialogs and settings routing vary → focused host tests plus physical permission/return checks; manual guidance if a pane cannot open.
- Android grouped grants and one-time access can mislead → retain individual underlying facts and refresh on activation.
- Existing coarse provider states hide authorization when services are off → add explicit facts rather than deriving the inventory from feature readiness.
- Presentation callbacks can be absent or late → one bounded slot, timeout, lifetime guards, no repeated prompting.
- User leaves Settings without changes → retain actual state and keep recovery available, without claiming success.

## Migration Plan

Commit planning artifacts before implementation. Extend contract, shared service, mock, both host families, and canonical conformance together in buildable slices. No database migration or stored grant cache. If native request-history storage changes incompatibly, reset only that owned history with an explicit format marker; never reset OS grants or product data. Roll back the coordinated code change as a unit. After focused verification and relevant physical checkpoints, sync specs and archive together.
