# Historical physical result

Associated implementation: [shared Rust loopback server](tasks.md).
macOS passed 16/16 and iOS/Android passed 17/17 production BLE fixture scenarios,
including recording, reconnect and applicable background/location behavior.
Native export presentation and cleanup passed. The idle WebSocket issue found
by testing was fixed with bounded ping/pong liveness handling.

Detailed device results, attempts and local log paths are retained under ignored
`.local/physical-testing/`. These results cover this historical change only.
