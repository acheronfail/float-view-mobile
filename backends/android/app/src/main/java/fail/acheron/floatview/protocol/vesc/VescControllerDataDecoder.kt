package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.application.ControllerCurrentLimits
import fail.acheron.floatview.protocol.RealtimeValues
import fail.acheron.floatview.protocol.RustVescControllerDataBridge
import fail.acheron.floatview.protocol.VescFault
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyVescControllerDataAbiVersion

/** Android adapter for Rust-owned command-4 and command-14 decoding. */
object VescControllerDataDecoder {
    private const val OPERATION_REALTIME = 1
    private const val OPERATION_CURRENT_LIMITS = 2
    private const val STATUS_OK = 0
    private const val STATUS_UNSUPPORTED_COMMAND = 2
    private const val RECORD_VERSION = 1
    private const val RESULT_REALTIME = 1
    private const val RESULT_CURRENT_LIMITS = 2
    private const val SERIES_CELLS_PRESENT = 1
    private const val FAULT_NONE = 0
    private const val FAULT_OVER_VOLTAGE = 1
    private const val FAULT_UNKNOWN = 2
    private const val RECORD_HEADER_BYTES = 8
    private const val REALTIME_RECORD_BYTES = 60
    private const val CURRENT_LIMITS_RECORD_BYTES = 40
    private const val MAXIMUM_RECORD_BYTES = REALTIME_RECORD_BYTES
    private val linkedAbiVersion = RustVescControllerDataBridge.abiVersion()

    fun realtime(payload: ByteArray): RealtimeValues =
        realtimeRecord(decode(OPERATION_REALTIME, payload))

    fun currentLimits(payload: ByteArray): ControllerCurrentLimits =
        currentLimitsRecord(decode(OPERATION_CURRENT_LIMITS, payload))

    private fun decode(operation: Int, payload: ByteArray): ByteArray {
        verifyVescControllerDataAbiVersion(linkedAbiVersion)
        val record = ByteArray(MAXIMUM_RECORD_BYTES)
        val result = IntArray(2)
        val status = RustVescControllerDataBridge.decode(operation, payload, record, result)
        return decodeBridgeResult(status, record, result)
    }

    internal fun decodeBridgeResult(
        status: Int,
        record: ByteArray,
        result: IntArray,
    ): ByteArray {
        if (result.size != 2) fail()
        val (recordLength, unsupportedCommand) = result
        return when (status) {
            STATUS_OK -> {
                if (
                    recordLength !in 0..minOf(record.size, MAXIMUM_RECORD_BYTES) ||
                    unsupportedCommand != 0
                ) fail()
                record.copyOf(recordLength)
            }
            STATUS_UNSUPPORTED_COMMAND -> {
                if (recordLength != 0 || unsupportedCommand !in 0..255) fail()
                throw VescProtocolException.UnsupportedCommand(unsupportedCommand)
            }
            else -> fail()
        }
    }

    internal fun realtimeRecord(record: ByteArray): RealtimeValues {
        if (
            record.size != REALTIME_RECORD_BYTES ||
            record[0].unsigned() != RECORD_VERSION ||
            record[1].unsigned() != RESULT_REALTIME ||
            record[2].unsigned() != 0 ||
            record[5].unsigned() != 0 ||
            record[6].unsigned() != 0 ||
            record[7].unsigned() != 0
        ) fail()

        val rawFault = record[4].unsigned()
        val faultKind = when (record[3].unsigned()) {
            FAULT_NONE -> if (rawFault == 0) VescFault.Kind.NONE else fail()
            FAULT_OVER_VOLTAGE -> if (rawFault == 1) VescFault.Kind.OVER_VOLTAGE else fail()
            FAULT_UNKNOWN -> if (rawFault > 1) VescFault.Kind.UNKNOWN else fail()
            else -> fail()
        }
        var cursor = RECORD_HEADER_BYTES
        val controllerTemp = record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        val motorTemp = record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        val motorCurrent = record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        val batteryCurrent = record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        val duty = record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        val packVoltage = record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        val erpm = record.readInt(cursor).also { cursor += Int.SIZE_BYTES }
        if (cursor != record.size) fail()
        return RealtimeValues(
            controllerTemp,
            motorTemp,
            motorCurrent,
            batteryCurrent,
            duty,
            erpm,
            packVoltage,
            rawFault,
            VescFault(rawFault, faultKind),
        )
    }

    internal fun currentLimitsRecord(record: ByteArray): ControllerCurrentLimits {
        if (
            record.size != CURRENT_LIMITS_RECORD_BYTES ||
            record[0].unsigned() != RECORD_VERSION ||
            record[1].unsigned() != RESULT_CURRENT_LIMITS ||
            record[2].unsigned() and SERIES_CELLS_PRESENT.inv() != 0 ||
            record[3].unsigned() != 0 ||
            record[4].unsigned() != 0 ||
            record[6].unsigned() != 0 ||
            record[7].unsigned() != 0
        ) fail()
        val hasCells = record[2].unsigned() and SERIES_CELLS_PRESENT != 0
        val cells = record[5].unsigned()
        if (hasCells != (cells > 0)) fail()

        var cursor = RECORD_HEADER_BYTES
        val limits = List(4) {
            record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        }
        if (cursor != record.size || limits.any { kotlin.math.abs(it) <= 0.0 }) fail()
        return ControllerCurrentLimits(
            limits[0],
            limits[1],
            limits[2],
            limits[3],
            cells.takeIf { hasCells },
        )
    }

    private fun ByteArray.readFiniteDouble(offset: Int): Double {
        if (offset < 0 || offset > size - Long.SIZE_BYTES) fail()
        var bits = 0L
        repeat(Long.SIZE_BYTES) { index ->
            bits = (bits shl 8) or this[offset + index].unsigned().toLong()
        }
        return Double.fromBits(bits).takeIf { it.isFinite() } ?: fail()
    }

    private fun ByteArray.readInt(offset: Int): Int {
        if (offset < 0 || offset > size - Int.SIZE_BYTES) fail()
        var bits = 0
        repeat(Int.SIZE_BYTES) { index ->
            bits = (bits shl 8) or this[offset + index].unsigned()
        }
        return bits
    }

    private fun Byte.unsigned(): Int = toInt() and 0xff

    private fun fail(): Nothing = throw VescProtocolException.TruncatedPayload
}
