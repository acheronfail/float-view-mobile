package fail.acheron.floatview.permissions

/** Native dialog identity survives client timeouts until the OS resolves it. */
internal class PermissionPresentation {
    private var next = 0L
    private var current: Pair<Long, (Boolean) -> Unit>? = null
    private var system: Long? = null

    @Synchronized fun begin(available: Boolean, done: (Boolean) -> Unit): Long? {
        if (!available || current != null || system != null) { done(false); return null }
        val id = ++next
        current = id to done
        return id
    }
    @Synchronized fun busy() = current != null || system != null
    @Synchronized fun active(id: Long) = current?.first == id
    @Synchronized fun startSystem(id: Long): Boolean {
        if (!active(id)) return false
        system = id
        return true
    }
    @Synchronized fun awaitingSystem() = system != null
    @Synchronized fun resolveSystem(success: Boolean) {
        val id = system ?: return
        system = null
        finish(id, success)
    }
    @Synchronized fun finish(id: Long, success: Boolean) {
        val pending = current?.takeIf { it.first == id } ?: return
        current = null
        pending.second(success)
    }
}
