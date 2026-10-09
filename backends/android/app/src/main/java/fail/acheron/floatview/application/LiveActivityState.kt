package fail.acheron.floatview.application

import org.json.JSONObject

/** Native presentation from Rust; no reconstruction of telemetry or estimation. */
data class LiveActivityState(
    val generation: Long,
    val observedAt: Long?,
    val source: String?,
    val percentage: Double?,
    val distanceMeters: Double?,
    val batteryPercent: Double?,
    val imperial: Boolean,
) {
    companion object {
        fun decode(value: JSONObject): LiveActivityState = LiveActivityState(
            generation = value.getLong("generation"),
            observedAt = if (value.isNull("observed_at")) null else value.getLong("observed_at"),
            source = if (value.isNull("source")) null else value.getString("source"),
            percentage = value.numberOrNull("percentage"),
            distanceMeters = value.numberOrNull("distance_meters"),
            batteryPercent = value.numberOrNull("battery_percent"),
            imperial = value.getBoolean("imperial"),
        )

        private fun JSONObject.numberOrNull(key: String): Double? =
            if (isNull(key)) null else getDouble(key).takeIf { it.isFinite() }
    }
}
