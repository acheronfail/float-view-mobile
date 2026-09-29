# BLE extraction inventory

Baseline: `fae14c1f`; planning already committed in `fae1ec75`.

| Responsibility                           | Current implementation / deletion target                                                                       | Selected owner                                                                |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Saved/UART advertisement eligibility     | Swift/Kotlin `VescBluetoothPolicy.shouldPublish`, native saved-ID copies                                       | backend-service, exact existing opaque identity                               |
| Required profile and response preference | both `VescBluetoothPolicy.writeMode/setupFailure`                                                              | backend-service                                                               |
| Segmentation and admission               | both `chunks/acceptsQueuedWrite`, Apple `pendingWrites/pendingWriteBytes/drainWriteQueue`, Android equivalents | backend-service; include current chunk                                        |
| Setup timeout                            | Apple `setupDeadline/scheduleSetupDeadline`, Android `connectionTimeout/scheduleConnectionTimeout`             | existing `Policy::ConnectionAttempt`, ten seconds                             |
| Write progress timeout                   | absent                                                                                                         | backend-service, ten seconds                                                  |
| Callback identity                        | native peripheral/GATT/characteristic identity; SharedService substitutes current generation                   | capture generation at connect issuance, operation at effect execution         |
| GATT/notification setup                  | CoreBluetooth and Android adapter                                                                              | native OS execution; shared profile decision before enable effect             |
| Descriptor interoperability              | Kotlin `notificationSetup`                                                                                     | inline native execution; absent CCCD allowed only after local enable succeeds |
| Object retention                         | currently unbounded discovered maps                                                                            | native registry capped at 50, active object retained independently            |
| Reconnect / public snapshots             | backend-service policy                                                                                         | unchanged                                                                     |

The two native policy files and their portable helper tests are exact deletion targets after host selection. Replace their tests with shared reducer cases and native boundary tests. Keep permission policy, Android identity hashing and GATT UUIDs native. The public specification's platform identity means the existing exact local identifier; Android exposes the installation-salted address identity, never raw addresses.

## Behavior evidence

- Vendored `vendor/vesc_tool/bleuart.cpp:134–148` splits into 20-byte writes without response. Lines 151–185 implement exact saved-identity/UART discovery, and lines around 317 enable the notification descriptor.
- Both existing native policy suites cover exact saved identity, 20/244 segmentation, response preference, missing profile, cancellation and queue arithmetic. They do not test actual native queue accounting or same-device asynchronous callback replacement.
- Android's adapter documents Samsung/VESC Express corruption after MTU 247. Retain default ATT MTU 23, payload 20, and 20 ms no-response pacing. Existing Kotlin tests also cover absent CCCD after successful local notification routing.
- Apple uses negotiated maximumWriteValueLength for the chosen mode, capped at 244; no-response uses canSendWriteWithoutResponse and the OS readiness callback.
- Historical checkpoints under `conformance/physical-checkpoints/` document real hosts; they are prior behavior evidence, not acceptance of this change. New physical evidence remains required.
- Both queues currently subtract an in-flight chunk from totals. Both hosts own duplicate setup timers. Apple stale service/characteristic errors can fail the current connection; Android no-response continuation checks only device identity. SharedService labels received events with the latest generation. All require replacement.

## Closed exchange and resource matrix

Keep the existing lifetime-qualified, sequenced 1 MiB JSON exchange and exactly the existing C/JNI families. Add normalized profile facts (required service, write/notify properties, per-mode payload ceilings), notification completion and write progress facts. Setup and writes carry generation plus operation IDs; profile discovery uses the connect operation identity. Effects select notification setup and one write chunk. Distinguish submission, acknowledgement and availability; duplicates must not supply another operation's capacity. Native callbacks capture their issuance identity.

| Resource             | Bound / ordering                                                                            | Planned evidence                                                    |
| -------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Discovery            | 50 shared candidates, 50 native cached objects                                              | saved bypass only for advertisement, registry eviction              |
| Frame admission      | 128 total chunks, 65,536 bytes including current                                            | exact/+1 chunk and byte arithmetic; reject before copying           |
| Payload              | positive negotiated ceiling, at most 244; Android 20                                        | zero/invalid, small/large segmentation, byte order                  |
| Write                | one operation; response waits submission + ack, no-response waits submission + availability | either result order, duplicates, stale generation/op, failure       |
| Setup/write deadline | existing ten-second setup timer; ten-second current write timer                             | missing result, late result, cancellation                           |
| Callback admission   | existing 64 ordinary callbacks; cleanup reserve                                             | saturation fails affected generation once; cleanup remains admitted |
| OS effects           | existing 2,048 effects, drain 8, cleanup reserve                                            | cancel removes setup/write effects and timers                       |

At 244 bytes × 128 chunks, the chunk cap makes 65,536 simultaneous bytes unreachable; test byte accounting arithmetic independently, and test the reachable combined limit without inventing oversized chunks.

## Source and dependency baseline

`audit-ownership.sh`: Rust 37,979, Apple 2,288, Android 2,460 production source lines (42,727 total; existing audit convention includes inline Rust tests). Workspace: 28 members, two binding crates/families, zero temporary bridges, eight Android runtime target artifacts. C exports: eight; JNI exports: seven, as counted by no_mangle declarations. No new dependency, crate, binding family, frontend contract or persistent format is needed. Final comparison must use the same audit plus production-only diff accounting for newly added test modules.
