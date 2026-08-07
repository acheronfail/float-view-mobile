# Native core spike baselines and budgets

Measured on 2026-08-07. These are acceptance gates for comparable release
builds on the same machine/device class, not product-download promises. Report
both absolute values and deltas because frontend assets and fixture resources
can change independently of the native core.

## Current host baselines

| Host                 | Current artifact                                                                   | Current cold-start/readiness baseline                                                                                                                                |
| -------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS arm64          | release `FloatViewMac` executable: 5,563,184 bytes                                 | `--test-host` process start to loopback readiness: 575.2, 69.5, 71.9, 72.9, 75.0 ms; warm-filesystem median 72.9 ms and first sample 575.2 ms                        |
| iOS simulator        | release universal simulator app: 16,544 KiB allocated; executable 15,772,760 bytes | approximately 1.6 s from cold launch request to SpringBoard dropping the launch assertion and the WebKit-backed scene becoming visible on an iPhone 17 Pro simulator |
| Android arm64 device | debug universal APK: 14,734,798 bytes                                              | Android 10 Galaxy S9+ cold `am start -W` stable samples: 1,147, 1,147, 1,164, 1,129 ms; median 1,147 ms (one 26 ms system-cache outlier excluded)                    |

The Android application behavior is still Kotlin-owned during this spike. The
APK already contains the bridge-only Rust libraries, so its measured total is
conservative. Those four stored APK entries total 1,859,024 bytes; the APK
without them would be approximately 12,875,774 bytes before ZIP structure and
alignment differences.

## Rust packaging results

The direct static-library strategy selected for Apple produced a 16,632,160
byte release archive on macOS. Archive size is not installed size: linking the
complete C harness and dead-stripping unused objects produced a 1,568,576 byte
Mach-O, or 1,301,560 bytes after local-symbol stripping. Apple packaging must
continue to link one target-specific static archive and let the final host link
strip unused code; it must not embed the `.a` as a resource.

The JNI spike produced one release shared library per Android ABI:

| ABI         | Cargo release `.so` | APK packaged/stripped entry |
| ----------- | ------------------: | --------------------------: |
| arm64-v8a   |             688,344 |                     481,776 |
| armeabi-v7a |             486,728 |                     336,436 |
| x86         |             648,556 |                     523,228 |
| x86_64      |             656,184 |                     517,584 |

No compatibility layer, alternate SQLite engine, generated bridge, callback
object graph, or weakened loopback/ABI invariant was required by the spike.

## Reviewed acceptance gates

The spike adopts these deliberately roomy gates until representative release
apps contain the completed core. A change outside them requires an explicit
design review; the gate must not be relaxed silently.

- macOS: Rust may add at most 10 MiB to the stripped release application and
  at most 500 ms to the current cold first-run readiness sample.
- iOS: Rust may add at most 10 MiB to the thinned release application and at
  most 500 ms to median cold frontend readiness on the same device/simulator.
- Android: Rust may add at most 5 MiB to a release APK/AAB download for one
  device ABI and at most 500 ms to median `am start -W` cold launch on the same
  physical device.
- Each packaged architecture contains exactly one core library, release
  symbols are stripped according to platform policy, and no library is also
  copied as an inert resource.
- ABI identity, JNI registration, bundled SQLite identity/options, loopback
  security/bounds, and panic containment remain hard gates regardless of size
  or startup performance.

Final cutover tasks must replace simulator/debug proxies with signed, thinned
release IPA/APK/AAB measurements and representative physical-device startup
samples while retaining these delta limits.
