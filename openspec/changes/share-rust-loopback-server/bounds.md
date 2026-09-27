# Admission and lifecycle bounds

Implementation selections for tasks 1.4 and 2–4. The test names below are
assigned coverage, not claims that those tests already exist or pass. Public
field/page/cursor/schema limits remain exactly those inventoried in baseline.md.
All counts include in-flight work; draining a queue does not release admission.

| Resource | Maximum | Enforcement and assigned boundary test |
| --- | ---: | --- |
| Accepted sockets (HTTP and upgraded WebSocket combined) | 16 | Acquire before connection task creation; `connection_admission` accepts 16, rejects 17, recovers after close |
| Accepted host operations per server generation | 64 | Queued and executing combined; `operation_admission` accepts 64, rejects 65 before host execution |
| Outstanding operations per client | 8 | `client_admission` verifies ninth rejection does not affect another client |
| Correlated replies per client | 8 | Separate FIFO; `slow_client_replies` checks correlation/no conflation; close on sustained inability to send |
| Pending complete state per client | 1 | `slow_client_state` replaces latest only, never replies |
| Host drain batch | 8 operations | `drain_boundary` verifies size and FIFO; byte ceiling may produce a smaller batch |
| Host command/completion envelope | 1,048,576 bytes | `envelope_boundary` checks exact/+1, malformed/unknown/version rejection before admission |
| WebSocket assembled message | 65,536 bytes | `fragmented_message_boundary` checks exact/+1 across fragments, not just individual frames |
| Session-data/export request | 16,384 bytes | `streamed_request_boundary` checks exact/+1 with and without Content-Length |
| Decoded historical response | 4,194,304 bytes | `decoded_response_boundary` checks encoded output before gzip; oversized result releases read lease |
| Historical reads | 2 | `history_read_admission` holds two and rejects third; cancellation releases capacity |
| Active archive transfers | 2 | `transfer_admission` checks third rejection before temporary storage creation |
| Archive chunk | 65,536 raw bytes | `transfer_chunk_boundary` checks exact/+1 and generation/handle/offset; caller-owned binary copy, no base64 |
| Pending chunks per transfer | 1 | `transfer_backpressure` prevents further socket/host reads until consumption |
| Import total | 134,217,728 bytes | `streamed_import_boundary` exact/+1, absent Content-Length, cumulative checked arithmetic and cleanup |
| Shared async executor threads | 1 | Current-thread Tokio runtime on dedicated server thread; `executor_ownership` excludes UI/runtime executors |
| Blocking response workers | 2 | Admission before scheduling; `worker_admission` verifies no unbounded blocking-task queue |
| Dedicated native host drain workers | 2 | `host_reentry` races complete/publish/runtime read/stop; serialized mutations retain application ordering |
| Native drain wait | 250 ms | `wait_and_stop` exact maximum, interruption, stop wakeup; never called on UI executor |
| Socket read/write inactivity | 30 s | `stalled_client_timeout` uses controlled time and checks release of all leases |
| Listener stop | 2 s | `stop_deadline` races reads, gzip, transfer and host completion; all transport tasks join within deadline |

There are no new archive content/identity limits. Export length is the immutable,
validated host file length and is streamed in bounded chunks; it must not be
mistakenly limited to the import ceiling. Temporary archive storage and existing
archive validation remain native-owned. Reserve cleanup state within the two
transfer slots; stop must not depend on admitting a cleanup operation into a full
ordinary operation queue. Late native completion is rejected by generation and
must still release its native temporary resources.

Control operations must have an available reply slot before acceptance. Slow
clients cannot acquire additional mutation slots indefinitely. Once accepted,
mutations are neither retried nor undone on client loss; queued/unstarted reads
are cancellable and running reads use the existing interruption path.

Historical data larger than a completion envelope must be delivered as bounded
typed batches through its read lease; the shared serializer alone constructs the
public response. Archive transfer buffers are binary caller-owned exchanges,
separate from control JSON. No native frontend JSON pass-through is permitted.

The listener generation changes on each successful start. Publisher state,
publication sequence, public revisions and 32-byte OS-random launch secret belong
to the process-owned server handle and survive listener stop. Sequence and ID
increments are checked; exhaustion fails closed rather than wrapping. Destroy
is idempotent; invalid or stale handles cannot alias a newer server. These are
covered by `generation_restart`, `publication_order`, `counter_exhaustion` and
`destroy_twice`.

Canonical conformance owns public boundary, archive, state and lifecycle
assertions. Rust component tests own internal exchange/admission/generation and
deadline checks; linked Swift/JVM tests own buffer/symbol/native executor checks.
