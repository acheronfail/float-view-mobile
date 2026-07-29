package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.backend.RefloatTelemetryRecord
import fail.acheron.floatview.backend.RideState

class RefloatDynamicTelemetryDecoder {
    var fields: List<String> = emptyList()
        private set
    var runtimeFields: List<String> = emptyList()
        private set

    fun reset() {
        fields = emptyList()
        runtimeFields = emptyList()
    }

    fun decodeFields(payload: ByteArray): Pair<List<String>, List<String>> {
        var index = 3
        fun list(): List<String> {
            require(index < payload.size) { "Truncated Refloat field list" }
            val count = payload[index++].toInt() and 0xff
            return List(count) {
                require(index < payload.size) { "Truncated Refloat field ID" }
                val length = payload[index++].toInt() and 0xff
                require(index + length <= payload.size) { "Truncated Refloat field ID" }
                payload.copyOfRange(index, index + length).decodeToString().also {
                    index += length
                }
            }
        }
        return (list() to list()).also {
            fields = it.first
            runtimeFields = it.second
        }
    }

    fun decodeData(payload: ByteArray): RefloatTelemetryRecord {
        require(fields.isNotEmpty()) { "Refloat telemetry IDs are unavailable" }
        val mask = payload[3].toInt() and 0xff
        val includedRuntimeFields = if (mask and 0x1 != 0) runtimeFields else emptyList()
        require(payload.size >= 13 + (fields.size + includedRuntimeFields.size) * 2) {
            "Truncated Refloat realtime response"
        }
        var index = 13
        val values = linkedMapOf<String, Double>()
        for (field in fields + includedRuntimeFields) {
            values[field] = decodeFloat16(payload, index)
            index += 2
        }
        val state = payload[9].toInt() and 0x03
        val footpad = (payload[10].toInt() ushr 6) and 0x03
        return RefloatTelemetryRecord(
            rideState = if (state == 3) RideState.RIDING else RideState.IDLE,
            boardState = state,
            footpadState = footpad,
            packVoltageV = values["motor.batt_voltage"] ?: 0.0,
            dutyCycle = values["motor.duty_cycle"] ?: 0.0,
            controllerTempC = values["motor.mosfet_temp"],
            motorTempC = values["motor.motor_temp"],
            motorCurrentA = values["motor.current"] ?: 0.0,
            batteryCurrentA = values["motor.batt_current"] ?: 0.0,
            erpm = (values["motor.erpm"] ?: 0.0).toInt(),
            speedKph = (values["motor.speed"] ?: 0.0) * 3.6,
        )
    }

    private fun decodeFloat16(payload: ByteArray, index: Int): Double {
        val bits =
            ((payload[index].toInt() and 0xff) shl 8) or
                (payload[index + 1].toInt() and 0xff)
        val sign = if (bits and 0x8000 == 0) 1.0 else -1.0
        val exponent = bits ushr 10 and 0x1f
        val fraction = bits and 0x03ff
        return if (exponent == 0) {
            sign * fraction * Math.scalb(1.0, -24)
        } else {
            sign * (1.0 + fraction / 1024.0) * Math.scalb(1.0, exponent - 15)
        }
    }
}
