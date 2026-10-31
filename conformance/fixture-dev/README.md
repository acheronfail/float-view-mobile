# Interactive fixture development

This development-only SvelteKit workbench controls the ESP32 byte bridge while
the production native backend owns Bluetooth. It runs through Vite on
`http://127.0.0.1:11004`; it has no deployment or persisted simulator state.

## Ownership

| Location               | Responsibility                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------- |
| `../fixture/`          | Reusable framing, serial transport, topology, firmware models and canonical scenarios |
| `src/lib/coordinator/` | Authoritative state, exclusive serial lifecycle and request servicing                 |
| `src/lib/control/`     | Browser client, reactive control state and form/UI                                    |
| `src/lib/contract.ts`  | Bounded browser/server state/action boundary                                          |
| `src/routes/`          | Thin SvelteKit adapters                                                               |

The reusable fixture must not import Svelte/workbench code. Node/serial-capable
files use `.server.ts`. Keep focused tests beside their owner; cross-owner and
native-decoder tests stay in `../test/`. The workbench reports requests; the
separate conformance runner owns assertions, pass/fail and physical checkpoints.

## Operation

Flash the fixture, then run in separate terminals:

```sh
just fixture dev <fixture-port>
just dev macos
```

Omit the serial path to select an enumerated port in the UI. Startup fails if the
loopback port is occupied. Browser reload/HMR preserve serial ownership; restart
`just fixture dev` after coordinator or other server-side changes.

Choose a direct/Express topology and firmware composition, select a semantic scene,
then pin individual values or freeze/resume motion as needed. Templates are editable
starting points; Float View chooses its own controller route. Connection controls
serialize disconnects around route/identity changes and reconnects.

For a fault exercise, select `firmwareFault`, then `idle` to end the active fault
while retaining history/latch. Once idle, clear the latch through Float View's
Fault history; recorded occurrences remain. Resolve ambiguous BMS selection with
the Board Profile's `preferredBmsCanId`, independently of controller `preferredCanId`.

Firmware models and supported variants live in `../fixture/` and source-derived
backend fixtures. Complete configuration writes must validate against the selected
schema and wire signature; unsafe states, malformed requests and motor/remote
actuation remain rejected. The ESP32 interprets none of these product commands.

Request diagnostics expose bounded routes, command classes and rejection reasons,
never raw schemas or configuration values. Shareable scenario JSON excludes
serial paths, launch tokens, power state and private device identities. Import
validation is atomic; an invalid document must not replace retained/active state.
