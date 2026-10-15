package fail.acheron.floatview.speech

/** Operation-specific ownership: stale callbacks must never release a newer focus lease. */
internal class SpeechLease {
    var active: Long? = null
        private set
    private var release: (() -> Unit)? = null
    fun begin(id: Long, cleanup: () -> Unit): Boolean {
        if (active != null) return false
        active = id
        release = cleanup
        return true
    }
    fun finish(id: Long): Boolean {
        if (active != id) return false
        active = null
        val cleanup = release
        release = null
        cleanup?.invoke()
        return true
    }
}
