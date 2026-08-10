package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.application.BmsSnapshot
import fail.acheron.floatview.protocol.RustVescBmsValuesBridge
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyVescBmsValuesAbiVersion
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets
import java.time.Instant

/** Android adapter for the Rust-owned COMM_BMS_GET_VALUES structural decoder. */
object VescBmsValuesDecoder {
    private const val RECORD_VERSION = 1
    private const val RECORD_HEADER_BYTES = 8
    private const val FIXED_VALUE_COUNT = 17
    private const val MAXIMUM_COUNT = 50
    private const val MAXIMUM_STATUS_BYTES = 40
    private const val MAXIMUM_RECORD_BYTES = 1_034
    private const val ANY_EXPECTED_CAN_ID = 255
    private const val EXTENDED_TAIL_PRESENT = 1
    private const val KNOWN_FLAGS = EXTENDED_TAIL_PRESENT
    private const val STATUS_OK = 0
    private const val STATUS_UNSUPPORTED_COMMAND = 2
    private val linkedAbiVersion = RustVescBmsValuesBridge.abiVersion()

    fun decode(
        payload: ByteArray,
        expectedCanId: Int?,
        observedAt: Instant,
    ): BmsSnapshot {
        verifyVescBmsValuesAbiVersion(linkedAbiVersion)
        val record = ByteArray(MAXIMUM_RECORD_BYTES)
        val result = IntArray(2)
        val status = RustVescBmsValuesBridge.decode(
            payload,
            expectedCanId ?: ANY_EXPECTED_CAN_ID,
            record,
            result,
        )
        return decodeBridgeResult(status, record, result, observedAt)
    }

    internal fun decodeBridgeResult(
        status: Int,
        record: ByteArray,
        result: IntArray,
        observedAt: Instant,
    ): BmsSnapshot {
        if (result.size != 2) fail()
        val (recordLength, unsupportedCommand) = result
        return when (status) {
            STATUS_OK -> {
                if (
                    recordLength !in minimumRecordBytes()..minOf(record.size, MAXIMUM_RECORD_BYTES) ||
                    unsupportedCommand != 0
                ) fail()
                snapshot(record.copyOf(recordLength), observedAt)
            }
            STATUS_UNSUPPORTED_COMMAND -> {
                if (recordLength != 0 || unsupportedCommand !in 0..255) fail()
                throw VescProtocolException.UnsupportedCommand(unsupportedCommand)
            }
            else -> fail()
        }
    }

    internal fun snapshot(record: ByteArray, observedAt: Instant): BmsSnapshot {
        if (record.size !in minimumRecordBytes()..MAXIMUM_RECORD_BYTES) fail()
        val version = record[0].unsigned()
        val flags = record[1].unsigned()
        val cellCount = record[2].unsigned()
        val temperatureCount = record[3].unsigned()
        val reportedCanId = record[4].unsigned()
        val dataVersion = record[5].unsigned()
        val statusLength = record[6].unsigned()
        val reserved = record[7].unsigned()
        if (
            version != RECORD_VERSION ||
            flags and KNOWN_FLAGS.inv() != 0 ||
            cellCount > MAXIMUM_COUNT ||
            temperatureCount > MAXIMUM_COUNT ||
            reportedCanId > 253 ||
            statusLength > MAXIMUM_STATUS_BYTES ||
            reserved != 0
        ) fail()

        val extended = flags and EXTENDED_TAIL_PRESENT != 0
        if (!extended && (dataVersion != 0 || statusLength != 0)) fail()
        val expectedLength = RECORD_HEADER_BYTES.toLong() + FIXED_VALUE_COUNT * Long.SIZE_BYTES +
            cellCount * Long.SIZE_BYTES + cellCount +
            temperatureCount * Long.SIZE_BYTES + statusLength
        if (expectedLength != record.size.toLong()) fail()

        var cursor = RECORD_HEADER_BYTES
        val fixed = List(FIXED_VALUE_COUNT) { record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES } }
        val cells = List(cellCount) { record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES } }
        val balancing = List(cellCount) {
            when (record[cursor++].unsigned()) {
                0 -> false
                1 -> true
                else -> fail()
            }
        }
        val temperatures = List(temperatureCount) {
            record.readFiniteDouble(cursor).also { cursor += Long.SIZE_BYTES }
        }
        val status = if (extended) record.strictUtf8(cursor, record.size) else null

        return BmsSnapshot(
            observedAt = observedAt,
            totalVoltageV = fixed[0],
            chargeVoltageV = fixed[1],
            inputCurrentA = fixed[2],
            inputIcCurrentA = fixed[3],
            sessionAmpHoursAh = fixed[4],
            sessionWattHoursWh = fixed[5],
            cellVoltagesV = cells,
            balancing = balancing,
            temperaturesC = temperatures,
            icTemperatureC = fixed[6],
            humiditySensorTemperatureC = fixed[7],
            humidityPercent = fixed[8],
            highestCellTemperatureC = fixed[9],
            stateOfCharge = fixed[10],
            stateOfHealth = fixed[11],
            reportedCanId = reportedCanId,
            lifetimeChargeAmpHoursAh = fixed[12],
            lifetimeChargeWattHoursWh = fixed[13],
            lifetimeDischargeAmpHoursAh = fixed[14],
            lifetimeDischargeWattHoursWh = fixed[15],
            pressurePa = fixed[16],
            dataVersion = if (extended) dataVersion else null,
            status = status,
        )
    }

    private fun minimumRecordBytes(): Int = RECORD_HEADER_BYTES + FIXED_VALUE_COUNT * Long.SIZE_BYTES

    private fun ByteArray.readFiniteDouble(offset: Int): Double {
        if (offset < 0 || offset > size - Long.SIZE_BYTES) fail()
        var bits = 0L
        repeat(Long.SIZE_BYTES) { index ->
            bits = (bits shl 8) or this[offset + index].unsigned().toLong()
        }
        return Double.fromBits(bits).takeIf { it.isFinite() } ?: fail()
    }

    private fun ByteArray.strictUtf8(start: Int, end: Int): String {
        try {
            return StandardCharsets.UTF_8.newDecoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
                .decode(ByteBuffer.wrap(this, start, end - start))
                .toString()
        } catch (_: CharacterCodingException) {
            fail()
        }
    }

    private fun Byte.unsigned(): Int = toInt() and 0xff

    private fun fail(): Nothing = throw VescProtocolException.TruncatedPayload
}
