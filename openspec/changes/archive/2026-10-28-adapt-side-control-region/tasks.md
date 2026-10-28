# Tasks

## 1. Capture actual layout geometry

- [x] 1.1 Add an explicitly enabled Debug simulator geometry report and focused capture workflow; verify the diagnostic source compiles and workflow configuration is valid.
- [x] 1.2 Run the capture on the authorized branch, download native/CSS bounds and screenshots, and record the measured mapping and run timings in the design/evidence.

## 2. Adapt existing controls

- [x] 2.1 Implement the measured host geometry mapping with resize/fallback handling; verify native geometry and frontend parsing with focused tests.
- [x] 2.2 Move existing tabs, Live header actions and routed back links into the usable side region, releasing bottom/header reservations; verify ordinary and side layouts, pointer/keyboard activation and overlays in browser tests.

## 3. Confirm integration

- [x] 3.1 Run relevant frontend type, unit, formatting and build checks, affected Apple compilation checks and strict OpenSpec validation; document commands and results.
- [x] 3.2 Capture the adapted app in targeted CI and inspect screenshots/control rectangles for system overlap and usable content space; document actual limits and timings with a reusable on-demand procedure.
- [x] 3.3 Pin iOS distribution jobs to Xcode 27.1 and verify script/workflow checks plus an explicitly requested unsigned device compilation; keep ordinary captures limited to the simulator.
