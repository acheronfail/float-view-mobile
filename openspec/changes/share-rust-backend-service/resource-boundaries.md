# Shared-service admission matrix

This completes the proposed limits missing from the initial audit. Existing
protocol/transport/history bounds in behavior-audit.md remain unchanged. These
are implementation targets, not claims that all service/host tests already pass.
Every target must be enforced before allocation, scheduling or mutation; exact
and +1 tests belong to the task that implements the owner.

| Owner / resource                               | Ceiling and admission point                                                                                                          | Cleanup / failure test                                                                                                         |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| Product settings                               | 4,096 serialized UTF-8 bytes before parsing a patch or stored value                                                                  | Invalid patch leaves SQL and revision unchanged                                                                                |
| Profiles                                       | 16,384 UTF-8 bytes, patch and merged value; the stricter existing Android/import ceiling                                             | Invalid merge rolls back route-hint changes                                                                                    |
| Names / local identifiers / opaque history IDs | 128 UTF-8 bytes; validate before lookup or insertion                                                                                 | NUL/path-like history rejected; raw firmware IDs never archived                                                                |
| Saved boards                                   | 50 persisted entries; reject new board before insertion; existing association/update still allowed                                   | Existing public snapshots already carry at most 50; explicit admission replaces hidden unreachable overflow                    |
| Discovery                                      | 50 current entries; refresh known entry or drop oldest expired/weakest candidate before admitting new observation                    | Active connection retained separately; 120-second expiry and 250-ms coalescing                                                 |
| Cache                                          | Eight entries; 32,768 payload bytes each; 65,536 envelope bytes each, 524,288 steady-state bytes                                     | Evict before storing ninth; reject bad identity/digest, remove corrupt owned file                                              |
| Catalog/recovery scan                          | 100 rows per keyset batch; one session open at a time                                                                                | Resume from committed last key; never materialize entire history                                                               |
| Owned-directory scan                           | 100,000 inspected entries per pass, streaming; no list of all files                                                                  | Stop with capacity error; reset validates the entire bounded pass before deletion                                              |
| Archive/export file                            | 134,217,728 bytes; count streamed input/output before next chunk                                                                     | Oversize cancels and removes partial staging                                                                                   |
| Transfer chunk / live transfers                | 65,536 bytes / two admitted leases                                                                                                   | Release admission only after worker actually terminates                                                                        |
| Staging                                        | 536,870,912 bytes aggregate; at most two active transfers and 20 retained exports; each abandoned export expires after 86,400,000 ms | Evict unleased exports before admission; active presentation is protected from client disconnect                               |
| Catalog lock                                   | 250-ms SQLite busy timeout                                                                                                           | Return busy without revision advancement                                                                                       |
| Writer / reader/pruner lock                    | Existing 1,000-ms / 250-ms SQLite busy timeouts                                                                                      | Interrupt reads/workers during cancellation and test lock contention                                                           |
| Portable command owner                         | 64 accepted requests; eight per client; no unbounded executor queue behind admission                                                 | Saturation fails before side effects                                                                                           |
| Worker pool                                    | Two archive/maintenance workers; two independent history readers                                                                     | Worker lease remains held across cancellation until exit                                                                       |
| OS effects/completions                         | Eight effects per drain, 1-MiB envelope, 64 live correlated operations                                                               | Correlate lifetime/generation; duplicates cannot mutate state                                                                  |
| Cleanup reserve                                | One cancellation flag per admitted operation (64), one cleanup bit per transfer (two), one shutdown flag                             | Cleanup updates fixed slots; no new allocation or ordinary queue permit needed                                                 |
| Control/configuration waiters                  | One pending mutation through existing coordinator, 3,250-ms terminal deadline; at most 64 transport response waiters                 | Disconnect clears waiter only; never replay an accepted mutation                                                               |
| Timer tokens                                   | Existing 2,048 token ceiling; service discovery/reconnect timers use fixed slots                                                     | Replace/cancel in place and reject stale generations                                                                           |
| Listener / service stop                        | Listener two seconds; service five seconds for durable close and worker cancellation                                                 | At deadline return failure while retaining worker-owned resources; never free live worker state or claim successful durability |
| Revision                                       | Maximum 9,007,199,254,740,991; checked increment before mutation                                                                     | Overflow is capacity failure, no wraparound                                                                                    |

New observable ceilings are saved-board admission, discovery entry count,
settings bytes, aggregate staging/export bytes, bounded directory scans and
service shutdown. The profile ceiling adopts the stricter existing platform
limit. These limits do not introduce new public methods or change SQL/archive
encoding. Catalog and recording datasets use bounded streaming and available
storage thresholds rather than whole-dataset in-memory collections. Storage
availability below 262,144,000 bytes stops new recording writes; existing warning
and critical thresholds remain unchanged.

Tune values retain the codec's 512-field/4,096-byte-string ceilings, 49,152-byte
serialized values limit and exact schema/wire validation. The shared store must
not admit invalid or incomplete values simply because SQLite accepts their JSON.
Long codec inputs remain subject to existing 32-KiB compressed / 384-KiB expanded
schema and 12,000 XML-element / depth-32 bounds.

Validation remains staged: product tests cover root/settings/identity/profile/
Tune/cache admission; subsequent session, archive, service, host and saturation
checks must prove their respective rows before task completion.
