package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.backend.RefloatTelemetryRecord
import fail.acheron.floatview.backend.RideState

object RefloatLegacyTelemetryDecoder {
    fun decode(payload: ByteArray): RefloatTelemetryRecord {
        val detailLevel = payload[3].toInt() and 0xff
        require(detailLevel in 1..3) { "Unsupported Refloat detail level" }
        require(payload.size >= if (detailLevel >= 3) 55 else if (detailLevel >= 2) 42 else 35) {
            "Truncated Refloat telemetry response"
        }
        fun unsigned(index: Int) = payload[index].toInt() and 0xff
        fun signed16(index: Int): Int =
            ((unsigned(index) shl 8) or unsigned(index + 1)).toShort().toInt()
        val state = unsigned(10) and 0x0f
        val switchBits = unsigned(11) and 0x0f
        val adc1 = unsigned(12) / 50.0
        val adc2 = unsigned(13) / 50.0
        val footpad = when (switchBits) {
            2 -> 3
            1 -> if (adc1 > adc2) 1 else 2
            else -> 0
        }
        return RefloatTelemetryRecord(
            rideState = if (state in 1..5) RideState.RIDING else RideState.IDLE,
            boardState = state,
            footpadState = footpad,
            packVoltageV = signed16(23) / 10.0,
            dutyCycle = (unsigned(33) - 128) / 100.0,
            controllerTempC = if (detailLevel >= 2) unsigned(39) / 2.0 else null,
            motorTempC = if (detailLevel >= 2) unsigned(40) / 2.0 else null,
            motorCurrentA = signed16(29) / 10.0,
            batteryCurrentA = signed16(31) / 10.0,
            erpm = signed16(25),
            speedKph = signed16(27) / 10.0 * 3.6,
        )
    }
}
