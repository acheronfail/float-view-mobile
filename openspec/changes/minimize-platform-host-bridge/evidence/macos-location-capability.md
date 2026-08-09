# macOS location capability evidence

Recorded on 2026-08-09.

The first physical host request remained `notDetermined` because the SwiftPM
executable did not embed `FloatViewMac-Info.plist`; Core Location therefore had
no registered usage description. The correction:

- embeds that plist in the executable's `__TEXT,__info_plist` section;
- adds `NSLocationWhenInUseUsageDescription`; and
- uses the narrower when-in-use request on macOS because Float View relies on
  normal process lifetime rather than a mobile background launch mechanism.

The rebuilt binary contains the embedded section and Core Location recognizes
the `fail.acheron.float-view.macos` bundle identity. A physical request changed
Rust state from `notDetermined/unavailable` to `authorized/available` through
the focused authorization sink. The shared adapter tests cover finite bounded
observations and delegate delivery outside the Core Location callback thread.

No coordinate arrived during the stationary indoor 20-second observation, so
the checkpoint records authorization/provider reachability without claiming a
persisted location row.
