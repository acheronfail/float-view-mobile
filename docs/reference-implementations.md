# Protocol and implementation references

Status: required reference policy for the backend reset.

This document defines which existing implementations inform Float View, how
conflicts are resolved, how upstream changes are tracked, and when the original
Float View implementation is removed.

## Reference precedence

Use this order when protocol behavior or expected results disagree:

1. **Observed behavior from a declared VESC/Refloat hardware and firmware
   profile**, captured by a read-only test with raw bytes and metadata.
2. **[`vedderb/vesc_tool`](https://github.com/vedderb/vesc_tool)**, the canonical
   VESC host-tool implementation.
3. **Published VESC or firmware documentation** associated with the tested
   version.
4. **[`acheronfail/floaty`](https://github.com/acheronfail/floaty)** as an
   authorized third-party mobile implementation and source of reusable code.
5. **The original code in this repository** as historical evidence only.

Hardware is listed first because firmware-version behavior can legitimately
differ from the current upstream tool. Such a difference must be documented
against an exact firmware profile; it is not permission to silently diverge
from VESC Tool.

The new Swift and Kotlin implementations must not inherit behavior merely
because the old Float View or Floaty implementation does it. Resolve every
material difference against VESC Tool and, where possible, a captured device
response.

## Initial compatibility reference profile

The first fixture and hardware comparison targets VESC firmware 6.06 with
Refloat 1.2 on Thor300, Thor301, Floatwheel XRV, and Floatwheel PintV. Float
package behavior is out of scope and must not leak into normalized Refloat
models.

After this baseline passes, qualify VESC 6.02-6.05 and representative earlier
Refloat releases through new version-labelled fixtures and hardware evidence.
Do not infer compatibility from a common command identifier or from Floaty
metadata alone. VESC Express is modeled as a transport/forwarder distinct from
the primary motor controller; the first smart-BMS reference is VESC VBMS only.

## Pinned upstream references

Record exact commits so fixture expectations are reproducible. Initial inspected
references as of 2026-07-27:

| Repository               | Role                                              | Branch   | Inspected commit                           |
| ------------------------ | ------------------------------------------------- | -------- | ------------------------------------------ |
| `vedderb/vesc_tool`      | Canonical protocol and host behavior              | `master` | `8d32a10f2f3bb7c5514c79e6c1d7b4f5fcf5e53e` |
| `acheronfail/floaty`     | Authorized private mobile comparison/reuse source | `master` | `c4c87fe26e91ef2345518c3e0a1adb941a211885` |
| `acheronfail/float-view` | Prior ride-review and diagnosis UX reference      | `master` | `fae1663c437f9862d008b610cb938950a1b410d6` |

R0 adds a machine-readable `references/upstreams.json` containing repository,
commit, inspection date, license/permission classification, and relevant paths.
Updating a pin requires reviewing relevant upstream diffs and updating fixtures,
traceability, and compatibility notes in the same change.

The build and test suite must not fetch any reference repository implicitly. Pin
refresh and authorized code import are explicit maintenance actions so ordinary
builds remain reproducible, offline-capable after dependency setup, and
independent of access to the private Floaty repository.

## VESC Tool reference map

At the inspected VESC Tool revision, start with:

- `packet.cpp` and `packet.h` for frame lengths, start/stop bytes, CRC16, stream
  reassembly, invalid-frame recovery, and maximum packet handling;
- `commands.cpp` and `commands.h` for command construction and response
  interpretation;
- `datatypes.h` for command identifiers, firmware/hardware types, and normalized
  structures;
- VESC Tool buffer helpers used by `commands.cpp` for byte order, scaling,
  strings, and version-dependent fields;
- `bleuart.cpp` and `bleuart.h` for the BLE UART service/characteristics and
  notification setup expectations;
- `vescinterface.cpp` and `vescinterface.h` for connection-level coordination;
- `mobile/logwriter.cpp`, `mobile/logreader.cpp`, and related UI code for log
  semantics worth comparing, without adopting their storage format by default.

The initial inspection confirms the common BLE UART identifiers:

```text
service: 6e400001-b5a3-f393-e0a9-e50e24dcca9e
write/RX: 6e400002-b5a3-f393-e0a9-e50e24dcca9e
notify/TX: 6e400003-b5a3-f393-e0a9-e50e24dcca9e
```

These are protocol facts, not portable device identity. The platform adapter
must still discover and validate actual characteristic properties.

## Floaty reference and reuse map

Floaty is useful for mobile, background, Onewheel, and Refloat behavior:

- `apps/app/utils/@thankthemaker/vesc-protocol/` contains a TypeScript VESC
  framing, command, parser, BMS, configuration, and Float/Refloat
  implementation;
- `apps/app/utils/BluetoothConnectionManager.ts` and related Bluetooth utilities
  show mobile GATT and reconnection behavior;
- `apps/app/context/BackgroundService/` and
  `apps/app/context/VescBackgroundService.ts` show a mobile background sampling
  approach;
- session components and utilities provide comparative logging/product behavior;
- generated firmware metadata identifies versions and extension areas that need
  fixtures.

The Floaty owner has explicitly authorized reuse in Float View. R0 must turn
that authorization into a durable repository record that states:

- the granting copyright owner;
- the source repository and covered commits/files;
- permission to copy, modify, redistribute, and relicense for this project;
- attribution or notice requirements, if any;
- whether third-party code within Floaty is excluded or governed by its own
  license.

Until that record exists, source inspection and fixture comparison may proceed,
but copied Floaty code must not land. Afterward, reuse should retain provenance
at module/file level and should prefer extracting pure protocol concepts over
bringing in React Native or Expo architecture.

Floaty never overrides VESC Tool on canonical VESC behavior. Use it to identify
version branches and mobile failure cases, then confirm them against VESC Tool,
firmware documentation, and hardware.

No ordinary build or generated artifact may require access to the private Floaty
repository.

## Previous Float View viewer

[`acheronfail/float-view`](https://github.com/acheronfail/float-view) is the
previous responsive Svelte ride viewer. It is a product/UX and compatibility
reference, not the new native storage architecture.

Important behavior to preserve or improve:

- smooth desktop/mobile review of long rides;
- synchronized selection and zoom across charts and maps;
- speed, duty, voltage, wattage, current, temperature, orientation, and
  distance trends;
- footpad and ride-state points of interest;
- fault, quick-stop, charge, and gap identification;
- Float Control, Floaty, and VESC Tool import compatibility where retained.

The previous viewer loads an entire parsed ride into JavaScript and aggregates
chart points approximately to pixel width. The new frontend may reuse its
visual/interaction ideas, but native range queries and rollups must bound data
before it reaches the WebView.

Its package metadata declares `AGPL-3.0-only`. If code is copied into a project
distributed under different terms, cover it in the same durable owner
permission/relicensing record as Floaty and preserve any third-party dependency
licenses. Referencing behavior and independently reimplementing the UX does not
make the old viewer a runtime dependency.

## GPLv3 boundary

VESC Tool is licensed under GNU GPLv3. Reading it as the canonical behavioral
reference, citing paths/commits, and validating independently written fixtures is
the default plan. Directly copying or translating its expressive source into
Float View is a separate product licensing decision.

Unless that decision is accepted:

- independently implement Swift, Kotlin, and TypeScript behavior from protocol
  facts, documented field layouts, captured byte streams, and black-box
  expectations;
- do not paste, vendor, translate line-by-line, or generate new modules from VESC
  Tool source;
- record repository, commit, and relevant path for source-informed expectations
  without copying source text;
- validate source-informed expectations against hardware or another permitted
  source whenever practical.

If Float View directly incorporates GPLv3 code, record the affected modules,
attribution, source-distribution obligations, and project-wide license
compatibility before it lands. Review the intended iOS distribution path
specifically, because application-store terms and GPLv3 obligations can interact
in ways that require informed legal/product review.

This policy is an engineering safeguard, not legal advice.

## Traceability

R0 creates `references/protocol-traceability.md`. Each first-slice behavior has a
row like:

| Behavior             | Canonical reference   | Hardware evidence  | Fixture/test         | Notes                     |
| -------------------- | --------------------- | ------------------ | -------------------- | ------------------------- |
| Short-frame encoding | VESC Tool commit/path | profile/capture ID | fixture ID           | independently implemented |
| Firmware response    | VESC Tool commit/path | profile/capture ID | fixture ID           | version conditions        |
| BLE UART discovery   | VESC Tool commit/path | platform run ID    | conformance scenario | property checks           |

Required first-slice rows include:

- BLE service and characteristic discovery;
- notification enablement;
- short and long frame encoding;
- CRC calculation and rejection;
- arbitrary stream chunking and resynchronization;
- firmware request/response;
- every selected telemetry field, scale, unit, and version condition;
- Refloat custom data used by the first logging profile;
- stop, cancellation, and disconnect behavior where the protocol is involved.

Missing or conflicting evidence is represented as an explicit unresolved row,
not filled from memory.

## Reference-driven fixture workflow

For each protocol behavior:

1. identify the relevant VESC Tool revision and paths;
2. note any firmware-version branches;
3. inspect Floaty and the original Float View for additional cases or
   discrepancies;
4. capture or construct sanitized input bytes with documented provenance;
5. record the expected normalized result independently of any language model;
6. confirm against a declared hardware profile where practical;
7. feed identical fixture/chunk manifests to Swift and Kotlin;
8. add the fixture and traceability row in the same change.

For the R6 hardware baseline, run the declared board with the pinned VESC Tool
and record its read-only firmware/telemetry interpretation. Float View's
normalized output must agree within documented units, scaling, timing, and
firmware-version differences.

## Original Float View retirement

The original implementation is frozen reference material, not a foundation for
the new architecture.

### During R0

- make no feature changes to the legacy native server, BLE, protocol, generator,
  mock, or UI paths;
- inventory every legacy file and identify unique behavior worth preserving;
- compare that behavior with the pinned VESC Tool and Floaty references;
- convert useful behavior into provenance-bearing fixtures or test scenarios;
- record an immutable Git commit/tag that can recover the old implementation.

### At R1

- remove the original Android/iOS server, BLE, protocol, and dispatch code;
- remove the legacy Zod contract, generator, mock `/ws` behavior, and consuming
  UI;
- keep project metadata, signing, identifiers, assets, and useful build wrappers;
- do not keep a compiled, active, or copied `legacy/` implementation in the
  normal source tree;
- begin the new modules only from `api/`, conformance scenarios, fixtures,
  platform ports, and the reference policy above.

If R0 needs a temporary move to unblock new directory names, use a clearly
labelled non-shipping `legacy/` location and delete it before R1 completes. Git
history and the immutable baseline ref are the permanent archive.

After R1, consult the old implementation through the immutable Git ref or a
separate local worktree. Never reintroduce it as a build dependency or
behavioral authority.

## Upstream review cadence

- Check VESC Tool before adding a new command, firmware family, or protocol
  field.
- Review the pinned VESC Tool diff before each hardware compatibility release.
- Review Floaty when its mobile behavior or supported firmware can improve the
  fixture and failure matrix; lack of private-repository access must not block
  ordinary builds.
- Review the prior Float View viewer when planning ride review, chart
  interaction, imports, points of interest, or diagnostic workflows.
- Record upstream drift as a compatibility task rather than silently changing
  parsers.
- Hardware evidence and compatibility remain pinned until updated behavior
  passes the full fixture, conformance, and hardware gates.
