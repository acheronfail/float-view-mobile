package fail.acheron.floatview.host

import fail.acheron.floatview.core.RustBackendRuntime

// Capability status: implemented — instrumentation and physical process continuity pass.
// Concrete owner: RustBackendRuntime and its single ProcessCoreBridge worker.
internal typealias FloatViewRuntimeBridge = RustBackendRuntime
