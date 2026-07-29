# Interactive fixture development

This development-only SvelteKit application controls the unchanged ESP32 byte
bridge while a production native Float View backend owns Bluetooth. It runs only
through Vite at `http://127.0.0.1:11004`; there is no build, preview, adapter,
package, deployment, or persisted simulator state.

## Ownership

- `../fixture/` owns reusable VESC framing, serial transport, the deliberate
  THOR300 identity, firmware variants, Refloat codecs, topology strategies,
  telemetry generation, simulation, and canonical physical scenarios. It must
  not import Svelte or fixture-development code.
- `src/lib/coordinator/` owns authoritative state, exclusive serial lifecycle,
  high-level actions, request servicing, and process-wide resolution. Files
  that can touch Node or serial use the `.server.ts` suffix.
- `src/lib/control/` owns the browser API client, reactive control state, the
  configuration form, and the control surface.
- `src/lib/contract.ts` is the bounded state/action boundary shared by browser
  and server.
- `src/routes/` contains thin SvelteKit adapters only.

Keep owner directories flat, keep focused tests beside their owner, and avoid
generic `components`, `services`, `types`, `utils`, and barrel directories.
Cross-owner and native-decoder tests remain in `../test/`.

## Operation

Flash the fixture firmware, then run these in separate terminals:

```sh
just fixture dev /dev/cu.usbmodemXXXX
just dev macos
```

Omit the serial path to select an enumerated port in the UI. The server binds
strictly to `127.0.0.1:11004`; if that port is occupied, startup fails.

The workbench composes up to 16 THOR300 controller candidates and 64 discovered
CAN IDs with VESC firmware 6.02, 6.05, or 6.06; Refloat 1.0.0, 1.1.0, or 1.2.3;
and direct or VESC Express topology. The common templates remain editable
starting points for direct, single-Express, and reported multi-controller
setups. Float View still chooses and polls only one active controller route; the
workbench never drives that selection or reports a test result.

Choose a coherent semantic scene, then optionally pin individual bounded values
while the remaining motion continues, freeze/resume generated motion, or reset
to the scene baseline. Connection controls can serialize a disconnect around a
CAN move, identity replacement, ping-visibility change, or unchanged reconnect.
The request area reports the selected route, command class, response kind,
fragmentation, counters, and bounded diagnostics. Refloat 1.0.0 serves exact
command 1 and command 10 detail 1–4 layouts; 1.1.0 and 1.2.3 serve negotiated
command 31/32 layouts and compact command 10 detail 4. Command 35 and commands
41–43 remain unsupported.

Scenario JSON is a shareable, strictly validated document capped at 65,536 UTF-8
bytes. It includes public aliases, topology, package identities, discovery
behavior, fragmentation, scene, and overrides. It excludes serial paths, launch
tokens, power state, real Bluetooth identifiers, and raw controller UUID bytes.
Aliases are capped at 64 UTF-8 bytes and deterministically map to private
simulated identities only inside the server. An invalid import is rejected
without changing the retained or active scenario.

This UI is for manual investigation beside a developer-operated product. The
separate conformance runner remains the owner of scripted scenarios, assertions,
pass/fail, and physical checkpoints. Shared protocol and semantic fixture types
in `../fixture/` are used by both paths.

The native backends support route-correct telemetry for direct and Express
topologies across VESC 6.02, 6.05, and 6.06 and the Refloat 1.0.0, 1.1.0, and
1.2.3 protocol families. Refloat 1.0.0 uses fixed legacy packets; 1.1.0 and
1.2.3 negotiate dynamic layouts. All families use simple periodic command 10
detail-4 compact polling for accumulated and charging values.

Browser reload and UI HMR preserve serial ownership. Restart
`just fixture dev` after changing coordinator or other server-side source.
