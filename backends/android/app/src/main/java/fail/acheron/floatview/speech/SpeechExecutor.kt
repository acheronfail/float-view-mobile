package fail.acheron.floatview.speech

/** Closed OS boundary; callbacks and all executor calls run on the main looper. */
internal interface SpeechEngine {
    fun initialize(done: (Boolean) -> Unit)
    fun selectVoice(): String?
    fun speak(request: SpeechRequest, done: (Long, String) -> Unit): Boolean
    fun stop()
    fun shutdown()
}

internal class SpeechExecutor(
    private val createEngine: () -> SpeechEngine,
    private val acquireFocus: (onLoss: () -> Unit) -> (() -> Unit)?,
    private val now: () -> Long,
    private val later: (Long, () -> Unit) -> Unit,
    private val report: (String, Long, String?) -> Unit,
) {
    private var engine: SpeechEngine? = null
    private var ready = false
    private var initializing = false
    private var resetOnRefresh = false
    private var refreshPending = false
    private var epoch = 0L
    private var closed = false
    private val lease = SpeechLease()

    fun voiceSettingsOpened() { resetOnRefresh = true }
    fun refresh() {
        if (closed) return
        if (initializing) { if (resetOnRefresh) refreshPending = true; return }
        if (resetOnRefresh) {
            if (lease.active != null) { refreshPending = true; return }
            resetOnRefresh = false
            ready = false
            discardEngine()
        }
        if (engine != null && ready) { selectVoice(); return }
        initializing = true
        val token = ++epoch
        report("initializing", 0, null)
        try {
            discardEngine()
            val current = createEngine()
            engine = current
            current.initialize { success ->
                if (token == epoch && initializing && !closed) {
                    initializing = false
                    if (refreshPending && resetOnRefresh) { refreshPending = false; refresh(); return@initialize }
                    ready = success
                    if (ready) selectVoice() else {
                        discardEngine()
                        report("unavailable", 0, "engine_unavailable")
                    }
                }
            }
        } catch (_: Exception) {
            initializing = false; ready = false; epoch++
            discardEngine()
            report("unavailable", 0, "engine_unavailable")
        }
        later(10_000) {
            if (token == epoch && initializing) {
                initializing = false; ready = false; epoch++
                discardEngine()
                report("unavailable", 0, "engine_unavailable")
            }
        }
    }
    private fun selectVoice() {
        val current = engine ?: run { ready = false; report("unavailable", 0, "engine_unavailable"); return }
        val reason = runCatching { current.selectVoice() }
            .getOrElse { "engine_unavailable" }
        ready = reason == null
        report(if (ready) "ready" else "unavailable", 0, reason)
    }
    fun speak(request: SpeechRequest) {
        val id = request.id
        if (now() >= request.expires || closed || lease.active != null || !ready) {
            report("outcome", id, "unavailable"); return
        }
        val current = engine ?: run { report("outcome", id, "unavailable"); return }
        val release = runCatching { acquireFocus { if (lease.active == id) finish("interrupted") } }.getOrNull()
        if (release == null) { report("outcome", id, "unavailable"); return }
        lease.begin(id) {
            runCatching { current.stop() }
            runCatching { release() }
        }
        val started = runCatching {
            current.speak(request) { completedId, outcome -> if (lease.active == completedId) finish(outcome) }
        }.getOrDefault(false)
        if (!started) finish("unavailable")
        later((request.expires - now()).coerceAtLeast(0)) { if (lease.active == id) finish("unavailable") }
    }
    fun cancel(id: Long) { if (lease.active == id) finish("interrupted") }
    fun interrupt() { finish("interrupted") }
    private fun finish(outcome: String) {
        val id = lease.active ?: return
        // Clear ownership before stop(), which may synchronously invoke callbacks.
        if (lease.finish(id)) report("outcome", id, outcome)
        if (refreshPending) { refreshPending = false; refresh() }
    }
    private fun discardEngine() {
        val old = engine
        engine = null
        runCatching { old?.shutdown() }
    }
    fun close() {
        if (closed) return
        closed = true; epoch++; finish("interrupted")
        discardEngine()
    }
}
