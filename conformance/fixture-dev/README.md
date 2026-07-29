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

The saved form atomically composes one THOR300 identity with VESC firmware
6.02, 6.05, or 6.06; Refloat 1.0.0, 1.1.0, or 1.2.3; and either direct or VESC
Express topology. Express exposes one CAN controller at a canonical decimal ID
from 0 through 253. Configuration changes require power off. Rides require an
attached powered-on fixture.

Current native backends poll Refloat telemetry only for supported 1.2.x devices
behind CAN. Direct mode still exercises identity and Refloat discovery, while
older packages reproduce the current unsupported or connected-without-telemetry
behavior. A later backend change will expand that compatibility.

Browser reload and UI HMR preserve serial ownership. Restart
`just fixture dev` after changing coordinator or other server-side source.
