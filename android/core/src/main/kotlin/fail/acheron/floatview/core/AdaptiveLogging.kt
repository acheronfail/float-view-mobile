package fail.acheron.floatview.core

data class SamplingSettings(
    val idlePersistIntervalMs: Long = 1_000,
    val ridingPersistIntervalMs: Long = 100,
    val postRideTailMs: Long = 15_000,
    val reconnectDurationMs: Long = 300_000,
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

enum class AlertKind {
    DUTY_HIGH,
    VOLTAGE_LOW,
    VOLTAGE_HIGH,
    MOTOR_TEMPERATURE_HIGH,
    CONTROLLER_TEMPERATURE_HIGH,
    FOOTPAD,
    CONNECTION_LOSS,
}

data class AlertRule(
    val id: String,
    val kind: AlertKind,
    val threshold: Double?,
    val durationMs: Long = 0,
    val hysteresis: Double = 0.0,
    val cooldownMs: Long = 10_000,
)

class AlertEvaluator {
    private val activeSince = mutableMapOf<String, Long>()
    private val lastFired = mutableMapOf<String, Long>()

    fun evaluate(rule: AlertRule, sample: TelemetrySample, connected: Boolean, atMs: Long): Boolean {
        val (value, high) = when (rule.kind) {
            AlertKind.DUTY_HIGH -> sample.dutyCycle to true
            AlertKind.VOLTAGE_LOW -> sample.packVoltageV to false
            AlertKind.VOLTAGE_HIGH -> sample.packVoltageV to true
            AlertKind.MOTOR_TEMPERATURE_HIGH -> sample.motorTempC to true
            AlertKind.CONTROLLER_TEMPERATURE_HIGH -> sample.controllerTempC to true
            AlertKind.FOOTPAD -> sample.footpadState?.toDouble() to false
            AlertKind.CONNECTION_LOSS -> (if (connected) 0.0 else 1.0) to true
        }
        val threshold = rule.threshold ?: return false
        value ?: return false
        val breached = if (high) value >= threshold else value <= threshold
        if (!breached) {
            activeSince.remove(rule.id)
            return false
        }
        val since = activeSince.getOrPut(rule.id) { atMs }
        if (atMs - since < rule.durationMs) return false
        if (lastFired[rule.id]?.let { atMs - it < rule.cooldownMs } == true) return false
        lastFired[rule.id] = atMs
        return true
    }
}
