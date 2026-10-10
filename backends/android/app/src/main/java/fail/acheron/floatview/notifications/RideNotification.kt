package fail.acheron.floatview.notifications

import fail.acheron.floatview.application.LiveActivityState
import java.util.Locale

/** Display-only values. Timestamp changes alone do not cause notification posts. */
data class RideNotification(
    val generation: Long?,
    val source: String?,
    val load: String,
    val distance: String,
    val battery: String,
    val chip: String?,
    val stale: Boolean,
    val recording: Boolean,
    val footpadLeft: String,
    val footpadRight: String,
) {
    companion object {
        fun from(state: LiveActivityState?, recording: Boolean, now: Long): RideNotification {
            val stale = state?.observedAt == null || now - state.observedAt >= 15_000
            fun percent(value: Double?) = value?.let { String.format(Locale.getDefault(), "%.0f%%", it) } ?: "—"
            val label = when (state?.source) {
                "duty" -> "Duty"
                "motor" -> "Motor"
                "battery" -> "Battery current"
                else -> "Drive load"
            }
            val load = if (stale) "—" else percent(state?.percentage)
            val distance = if (stale) "—" else state?.distanceMeters?.let {
                String.format(Locale.getDefault(), "%.2f %s", it / if (state.imperial) 1609.344 else 1000.0,
                    if (state.imperial) "mi" else "km")
            } ?: "—"
            val voltage = state?.batteryVoltage?.let { String.format(Locale.getDefault(), "%.1f V", it) } ?: "—"
            return RideNotification(state?.generation, if (stale || state?.status != null) null else state?.source,
                if (stale) "$label —" else state?.status ?: "$label $load", distance,
                if (stale) "—" else "${percent(state?.batteryPercent)} · $voltage",
                if (stale) null else state?.status ?: load.takeIf { state?.percentage != null }, stale, recording,
                if (stale) "neutral" else state?.footpadLeft ?: "neutral",
                if (stale) "neutral" else state?.footpadRight ?: "neutral")
        }
    }
}

/** A swipe dismisses updates for this connection, without stopping the recorder. */
class RideNotificationDismissal {
    private var dismissed = false
    private var generation: Long? = null

    fun dismiss(current: Long?) { dismissed = true; generation = current }

    fun suppressed(current: Long?): Boolean {
        if (dismissed && current != null) {
            if (generation == null) generation = current
            else if (current != generation) dismissed = false
        }
        return dismissed
    }
}
