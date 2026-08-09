package fail.acheron.floatview.application

data class SamplingSettings(
    val idlePersistIntervalMs: Long = 1_000,
    val ridingPersistIntervalMs: Long = 100,
    val postRideTailMs: Long = 5_000,
    val reconnectDurationMs: Long = 300_000,
    val idleProbeIntervalMs: Long = 500,
)

class AdaptiveSamplingPolicy(private val settings: SamplingSettings = SamplingSettings()) {
    private var lastPersistedMs: Long? = null
    private var previousRideState = RideState.UNKNOWN
    private var tailUntilMs: Long? = null

    fun shouldPersist(rideState: RideState, elapsedMs: Long): Boolean {
        if (rideState == RideState.RIDING) {
            tailUntilMs = null
        } else if (previousRideState == RideState.RIDING) {
            tailUntilMs = elapsedMs + settings.postRideTailMs
        }
        val inTail = tailUntilMs?.let { elapsedMs <= it } == true
        val interval =
            if (rideState == RideState.RIDING || inTail) settings.ridingPersistIntervalMs
            else settings.idlePersistIntervalMs
        val transition = rideState != previousRideState
        previousRideState = rideState
        val jitterToleranceMs = minOf(25, maxOf(0, interval / 5))
        val dueThresholdMs = maxOf(0, interval - jitterToleranceMs)
        val due = lastPersistedMs?.let { elapsedMs - it >= dueThresholdMs } ?: true
        return (transition || due).also { if (it) lastPersistedMs = elapsedMs }
    }
}
