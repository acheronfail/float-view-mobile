# Effective appearance verification

The selected source on every host is
`frontend-prefers-color-scheme`. The persisted `ThemePreference` remains Rust
application state; native code only colors platform chrome/canvas consistently.

## Automated host observations (2026-08-09)

| Host target      | Initial | Live change | Foreground return | WebView recreation | Result |
| ---------------- | ------- | ----------- | ----------------- | ------------------ | ------ |
| iOS simulator    | light   | light→dark  | dark→light        | light              | pass   |
| macOS process    | dark    | dark→light  | light             | light              | pass   |
| Android emulator | light   | light→dark  | dark→light        | light              | pass   |

The iOS lifecycle backgrounded the scene and discarded its frontend before the
light return. Android backgrounded and recreated its Activity/WebView. macOS
foreground return retained light appearance, and terminating only Float View's
disposable WebKit content process produced frontend generation 2 in light mode.

During the macOS recreation check the discarded WebContent window remained
allocated. `MacAppDelegate.discardFrontend()` now closes that exact window
after detaching delegates, preventing replacement from leaking a stale window.

## Physical checkpoints

- macOS: pass on the development Mac.
- iOS device: pass on the installed development iPhone build, confirmed by the
  user for initial appearance, live change, foreground return, and recreation.
- Android device: pass on the connected Samsung SM-G965F (API 29). The check
  covered initial light appearance, a live Dark mode Quick Settings change,
  Activity/WebView recreation, a foreground return after the 30-second WebView
  teardown grace, and a recreated dark WebView backed by the same process
  runtime.

The first physical Android live change exposed a lifecycle ordering race: a
replacement Activity could navigate to its cached loopback URL while the Rust
runtime was resuming, leaving WebView's one-shot request on
`ERR_CONNECTION_REFUSED`. `MainActivity` now creates a frontend only while
started and retries only main-frame loopback connection failures, with eight
bounded attempts. The rebuilt APK passed the same physical sequence.

No software-host result requires a native effective-appearance submission or a
second appearance policy path.
