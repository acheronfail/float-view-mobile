package fail.acheron.floatview.bluetooth

/** One connection's OS MTU result, consumed before starting service discovery. */
internal class GattMtuNegotiation {
    private enum class Phase { NEW, REQUESTED, COMPLETE, CLOSED }
    private var phase = Phase.NEW
    var payloadBytes = 20
        private set

    fun begin(): Boolean {
        if (phase != Phase.NEW) return false
        phase = Phase.REQUESTED
        return true
    }

    fun complete(mtu: Int, success: Boolean): Boolean {
        if (phase != Phase.REQUESTED) return false
        phase = Phase.COMPLETE
        if (success && mtu in 23..517) payloadBytes = (mtu - 3).coerceAtMost(512)
        return true
    }

    fun close() { phase = Phase.CLOSED }

    companion object {
        // Negotiate receive capacity independently of the shared 244-byte write
        // cap. Older Express firmware chunks notifications at min(MTU, 255)
        // without subtracting ATT overhead; MTU 247 loses three bytes per chunk.
        const val REQUESTED_MTU = 517
    }
}
