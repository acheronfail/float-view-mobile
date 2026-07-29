# Conformance

This is the only executable acceptance suite for the frontend/backend contract.
It imports the hand-written contract from `frontend/`, drives every backend as a
black box, and owns both deterministic and physical VESC simulation.

```sh
npm ci
npm test
npm run mock
npm run physical -- macos <fixture-port>
npm run physical -- android <fixture-port> <adb-serial>
npm run physical -- ios <fixture-port> <device-id>
```

Android, iOS, and macOS adapters launch or locate their application and expose a
loopback WebSocket URL; scenarios and assertions remain shared. Physical runs
add a serial-controlled `tools/vesc-test-device` byte bridge. Passing runs retain
no evidence; failures print bounded first-failure context. The physical adapters
contain only launch, loopback tunneling, lifecycle, and cleanup controls; all
contract and VESC assertions remain in shared TypeScript.

## Interactive fixture development

`just fixture dev [port]` starts the development-only SvelteKit controller on
`127.0.0.1:11004`. Run it beside `just dev <platform>` to operate the physical
ESP32 byte bridge manually. Reusable protocol and simulation code stays in
`fixture/`; server/UI ownership and operating details are documented in
`fixture-dev/README.md`.

The reusable simulator supports direct controllers, VESC Express and generic
VESC-protocol gateways, explicit or omitted CAN targets, multiple motor
controllers, unsupported/non-motor nodes, distinct controller UUIDs, malformed
or excessive ping lists, silent targets, and route-specific responses. The
compatibility matrix covers every direct/Express combination of VESC
6.02/6.05/6.06 and Refloat 1.0.0/1.1.0/1.2.3; legacy 1.0.0 reports wire version
`1.0` and uses fixed packet 10.
