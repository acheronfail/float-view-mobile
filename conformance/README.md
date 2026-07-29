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
