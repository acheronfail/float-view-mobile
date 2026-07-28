# Float View planning index

Read these documents in this order when changing architecture or the local API:

1. [`implementation-scope-lock.md`](implementation-scope-lock.md) — accepted
   first-release product/platform choices, compatibility baseline, explicit
   exclusions, and remaining release gates.
2. [`reset-roadmap.md`](reset-roadmap.md) — active execution plan, milestone
   status, deletion boundary, dependencies, and decision log.
3. [`vertical-slice-v1.md`](vertical-slice-v1.md) — normative behavior and
   acceptance criteria for connect, read, log, stop, and disconnect.
4. [`verification-plan.md`](verification-plan.md) — canonical build, run,
   conformance, simulator, hardware verification commands, and development
   dependencies.
5. [`reference-implementations.md`](reference-implementations.md) — canonical
   VESC Tool precedence, authorized Floaty reuse, licensing boundaries, and
   original implementation retirement.
6. [`vesc-test-device-plan.md`](vesc-test-device-plan.md) — ESP32-S3 programmable
   VESC BLE peripheral, host scenario engine, first-slice milestones, automation,
   evidence, and limits relative to physical hardware acceptance.
7. [`durable-logging.md`](durable-logging.md) — native log ownership, platform
   storage, background execution, WebView teardown, recovery, and query
   foundations for later charting.
8. [`product-experience.md`](product-experience.md) — Device/Sessions
   navigation, automatic adaptive logging, saved devices, ride review,
   diagnostic zoom, and user-facing data model.
9. [`board-profiles.md`](board-profiles.md) — per-board battery, motor, rails,
   visual identity, CAN/BMS discovery and overrides, calibration, and session
   profile snapshots.
10. [`api-v1-proposal.md`](api-v1-proposal.md) — API design rationale and broader
   proposed operation set.
11. [`../api/README.md`](../api/README.md) and
    [`../api/compatibility.md`](../api/compatibility.md) — implemented contract
    source of truth and compatibility policy.
12. [`implementation-notes.md`](implementation-notes.md) — ambiguities found
    during implementation and the bounded resolution used to continue.

When documents conflict:

1. committed schemas under `api/` own currently implemented JSON payload shape;
   accepted scope changes must update those schemas in R3 before implementation;
2. `implementation-scope-lock.md` owns accepted product/platform scope;
3. `vertical-slice-v1.md` owns first-slice lifecycle semantics;
4. `verification-plan.md` owns the executable proof required for completion;
5. `reference-implementations.md` owns reference precedence and provenance;
6. `vesc-test-device-plan.md` owns the programmable BLE test-device design and
   its emulator-specific milestone gates; it cannot weaken physical-hardware
   acceptance;
7. `durable-logging.md` owns native persistence and background lifecycle
   semantics;
8. `product-experience.md` owns navigation and user-facing session semantics;
9. `board-profiles.md` owns app-local board configuration and discovery/override
   behavior;
10. `reset-roadmap.md` owns implementation order and completion status;
11. `api-v1-proposal.md` supplies rationale and future direction.

Update planning status and its evidence in the same change that completes a
milestone. Add unresolved cross-cutting choices to the roadmap decision log
instead of leaving them only in an issue, pull request, or conversation.
