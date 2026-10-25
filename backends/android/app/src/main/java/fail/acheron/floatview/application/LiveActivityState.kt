package fail.acheron.floatview.application

import org.json.JSONObject

/** Native presentation from Rust; no reconstruction of telemetry or estimation. */
data class LiveActivityState(
    val generation: Long,
    val observedAt: Long?,
    val source: String?,
    val percentage: Double?,
    /** Shared current-ride total, including saved distance restored on resume. */
    val distanceMeters: Double?,
    val batteryPercent: Double?,
    val batteryVoltage: Double?,
    val highestTemperatureC: Double?,
    val temperatureSource: String?,
    val imperial: Boolean,
    val status: String?,
    val footpadLeft: String,
    val footpadRight: String,
) {
    companion object {
        fun decode(value: JSONObject): LiveActivityState = LiveActivityState(
            generation = value.getLong("generation"),
            observedAt = if (value.isNull("observed_at")) null else value.getLong("observed_at"),
            source = if (value.isNull("source")) null else value.getString("source"),
            percentage = value.numberOrNull("percentage"),
            distanceMeters = value.numberOrNull("distance_meters"),
            batteryPercent = value.numberOrNull("battery_percent"),
            batteryVoltage = value.numberOrNull("battery_voltage"),
            highestTemperatureC = value.numberOrNull("highest_temperature_c"),
            temperatureSource = if (value.isNull("temperature_source")) null else value.getString("temperature_source"),
            imperial = value.getBoolean("imperial"),
            status = if (value.isNull("status")) null else value.getString("status"),
            footpadLeft = value.getString("footpad_left"),
            footpadRight = value.getString("footpad_right"),
        )

        private fun JSONObject.numberOrNull(key: String): Double? =
            if (isNull(key)) null else getDouble(key).takeIf { it.isFinite() }
    }
}
