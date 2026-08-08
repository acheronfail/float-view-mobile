# Native ABI contract

`include/float_view_core.h` is the only public native boundary. It is generated
with cbindgen 0.29.2 from `host-c-abi` and checked in. Normal Xcode
and Gradle builds never run the generator.

## Identity and status

ABI version 3 and the exact 16-byte build identity are validated before a host
starts platform work. Functions return `FloatViewStatus`, a signed 32-bit
integer. Zero is success; every nonzero value is a stable named failure. Hosts
must not infer behavior from unrecognized values beyond treating them as fatal
for the current operation.

## Handles and lifecycle

`FloatViewRuntime *` is an opaque process-lifetime token, never host-owned
memory. The valid sequence is create, start, stop, destroy. A created-but-never
started runtime may also be destroyed. Destroying a started runtime fails.
Destroyed, null, fabricated, and prior-process tokens are invalid. Tokens are
monotonic and are not dereferenced, preventing stale host pointers from reading
freed Rust storage.

Start returns a nonzero runtime generation. Every event, byte submission,
completion, and command wait carries that generation. A mismatched generation
is rejected before payload processing or state mutation.

## Buffers and ownership

A nonzero length requires a non-null pointer. Lengths are `size_t`, bounded by
the named ABI maximum, validated before creating a Rust slice, and copied before
return. A zero length permits a null data pointer. Rust never retains a host
pointer and the host never owns a Rust allocation.

Outputs use caller-owned writable storage. Command retrieval writes one
`FloatViewCommandHeader` plus its payload. On insufficient capacity it writes
only the required payload size, does not dequeue the command, and returns
`FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL`. Fixed-width header fields are ordered as
ABI version, kind, flags, reserved, runtime generation, operation identifier,
and payload length.

## Panic and symbols

Development and release profiles use unwind-capable panics. Every exported C
operation catches unwinding; an associated runtime is poisoned and the call
returns `FLOAT_VIEW_STATUS_PANIC`. No panic may unwind into C, Swift, or JNI.
Poisoned runtimes reject further work and are not silently recovered. Process
abort remains fatal if a future target or dependency is explicitly built with
abort semantics.

Only explicitly named `float_view_*` entry points form the C ABI. Apple final
links apply `crates/host-c-abi/apple-exported-symbols.txt`; Android exposes only
its explicitly registered JNI load facade and hides the C surface from the APK
dynamic symbol table. Verification compares the generated declarations,
export list, and linked symbols. No Rust string, slice, collection, trait
object, allocator function, domain model, frontend envelope, snapshot,
telemetry history, or SQL representation crosses this boundary.
