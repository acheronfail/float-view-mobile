package fail.acheron.floatview.protocol

import fail.acheron.floatview.application.ControllerCurrentLimits
import fail.acheron.floatview.protocol.vesc.VescControllerDataDecoder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class RustVescControllerDataBridgeTest {
    @Test
    fun linkedBridgeReturnsExactRealtimeAndCurrentLimitRecords() {
        assertEquals(VESC_CONTROLLER_DATA_ABI_VERSION, RustVescControllerDataBridge.abiVersion())

        val realtimeRecord = ByteArray(60)
        val realtimeResult = IntArray(2)
        assertEquals(
            0,
            RustVescControllerDataBridge.decode(
                1,
                realtimePayload(1),
                realtimeRecord,
                realtimeResult,
            ),
        )
        assertEquals(listOf(60, 0), realtimeResult.toList())
        assertEquals(listOf(1, 1, 0, 1, 1, 0, 0, 0), realtimeRecord.take(8).map { it.unsigned() })
        assertEquals(25.0, realtimeRecord.readDouble(8), 0.0)
        assertEquals(-5.5, realtimeRecord.readDouble(16), 0.0)
        assertEquals(12.34, realtimeRecord.readDouble(24), 0.0)
        assertEquals(-2.5, realtimeRecord.readDouble(32), 0.0)
        assertEquals(0.5, realtimeRecord.readDouble(40), 0.0)
        assertEquals(50.4, realtimeRecord.readDouble(48), 0.0)

        val currentRecord = ByteArray(60)
        val currentResult = IntArray(2)
        assertEquals(
            0,
            RustVescControllerDataBridge.decode(
                2,
                currentLimitsPayload(18),
                currentRecord,
                currentResult,
            ),
        )
        assertEquals(listOf(40, 0), currentResult.toList())
        assertEquals(listOf(1, 2, 1, 0, 0, 18, 0, 0), currentRecord.take(8).map { it.unsigned() })
    }

    @Test
    fun rawBridgeRejectsInvalidCallersAndReportsSemanticFailures() {
        val output = ByteArray(60)
        assertEquals(1, RustVescControllerDataBridge.decode(99, realtimePayload(0), output, IntArray(2)))
        assertEquals(1, RustVescControllerDataBridge.decode(1, null, output, IntArray(2)))
        assertEquals(1, RustVescControllerDataBridge.decode(1, realtimePayload(0), null, IntArray(2)))
        assertEquals(1, RustVescControllerDataBridge.decode(1, realtimePayload(0), output, null))
        assertEquals(1, RustVescControllerDataBridge.decode(1, realtimePayload(0), output, IntArray(1)))
        val aliased = ByteArray(60)
        assertEquals(1, RustVescControllerDataBridge.decode(1, aliased, aliased, IntArray(2)))
        assertEquals(3, RustVescControllerDataBridge.decode(1, ByteArray(4_097), output, IntArray(2)))

        val unsupportedResult = IntArray(2)
        assertEquals(2, RustVescControllerDataBridge.decode(1, byteArrayOf(14), output, unsupportedResult))
        assertEquals(listOf(0, 14), unsupportedResult.toList())
        assertEquals(3, RustVescControllerDataBridge.decode(1, byteArrayOf(4), output, IntArray(2)))

        val smallResult = IntArray(2)
        assertEquals(
            4,
            RustVescControllerDataBridge.decode(1, realtimePayload(0), ByteArray(8), smallResult),
        )
        assertEquals(listOf(60, 0), smallResult.toList())
    }

    @Test
    fun kotlinAdapterValidatesEveryTaggedRecordBoundary() {
        val realtime = realtimeRecord(255)
        val values = VescControllerDataDecoder.realtimeRecord(realtime)
        assertEquals(25.0, values.controllerTempC, 0.0)
        assertEquals(-1_234, values.erpm)
        assertEquals(VescFault(255, VescFault.Kind.UNKNOWN), values.fault)

        val current = currentLimitsRecord(18)
        assertEquals(
            ControllerCurrentLimits(100.0, -100.0, 50.0, -20.0, 18),
            VescControllerDataDecoder.currentLimitsRecord(current),
        )

        for (invalid in listOf(
            realtime.copyOf(59),
            realtime.copyOf().also { it[0] = 2 },
            realtime.copyOf().also { it[1] = 2 },
            realtime.copyOf().also { it[2] = 1 },
            realtime.copyOf().also { it[3] = 0 },
            realtime.copyOf().also { it[5] = 1 },
            realtime.copyOf().also { Double.NaN.writeTo(it, 8) },
        )) {
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                VescControllerDataDecoder.realtimeRecord(invalid)
            }
        }
        for (invalid in listOf(
            current.copyOf(39),
            current.copyOf().also { it[1] = 1 },
            current.copyOf().also { it[2] = 2 },
            current.copyOf().also { it[5] = 0 },
            current.copyOf().also { 0.0.writeTo(it, 8) },
            current.copyOf().also { Double.POSITIVE_INFINITY.writeTo(it, 8) },
        )) {
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                VescControllerDataDecoder.currentLimitsRecord(invalid)
            }
        }

        assertThrows(VescProtocolException.UnsupportedCommand::class.java) {
            VescControllerDataDecoder.decodeBridgeResult(2, ByteArray(60), intArrayOf(0, 14))
        }
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            VescControllerDataDecoder.decodeBridgeResult(0, ByteArray(60), intArrayOf(61, 0))
        }
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            verifyVescControllerDataAbiVersion(2)
        }
    }

    private fun realtimePayload(fault: Int): ByteArray = ByteArray(30).also { payload ->
        payload[0] = 4
        250.toShort().writeTo(payload, 1)
        (-55).toShort().writeTo(payload, 3)
        1_234.writeTo(payload, 5)
        (-250).writeTo(payload, 9)
        500.toShort().writeTo(payload, 21)
        (-1_234).writeTo(payload, 23)
        504.toShort().writeTo(payload, 27)
        payload[29] = fault.toByte()
    }

    private fun currentLimitsPayload(cells: Int): ByteArray = ByteArray(464).also { payload ->
        payload[0] = 14
        0x57ad8f53.writeTo(payload, 1)
        0x42c80000.writeTo(payload, 9)
        0xc2c80000L.toInt().writeTo(payload, 13)
        0x42480000.writeTo(payload, 17)
        0xc1a00000L.toInt().writeTo(payload, 21)
        payload[463] = cells.toByte()
    }

    private fun realtimeRecord(fault: Int): ByteArray = ByteArray(60).also { record ->
        record[0] = 1
        record[1] = 1
        record[3] = when (fault) {
            0 -> 0
            1 -> 1
            else -> 2
        }
        record[4] = fault.toByte()
        for ((index, value) in listOf(25.0, -5.5, 12.34, -2.5, 0.5, 50.4).withIndex()) {
            value.writeTo(record, 8 + index * 8)
        }
        (-1_234).writeTo(record, 56)
    }

    private fun currentLimitsRecord(cells: Int): ByteArray = ByteArray(40).also { record ->
        record[0] = 1
        record[1] = 2
        record[2] = 1
        record[5] = cells.toByte()
        for ((index, value) in listOf(100.0, -100.0, 50.0, -20.0).withIndex()) {
            value.writeTo(record, 8 + index * 8)
        }
    }

    private fun Short.writeTo(bytes: ByteArray, offset: Int) {
        bytes[offset] = (toInt() ushr 8).toByte()
        bytes[offset + 1] = toByte()
    }

    private fun Int.writeTo(bytes: ByteArray, offset: Int) {
        repeat(4) { index -> bytes[offset + index] = (this ushr (24 - index * 8)).toByte() }
    }

    private fun Double.writeTo(bytes: ByteArray, offset: Int) {
        val bits = toBits()
        repeat(8) { index -> bytes[offset + index] = (bits ushr (56 - index * 8)).toByte() }
    }

    private fun ByteArray.readDouble(offset: Int): Double {
        var bits = 0L
        repeat(8) { index -> bits = (bits shl 8) or this[offset + index].unsigned().toLong() }
        return Double.fromBits(bits)
    }

    private fun Byte.unsigned(): Int = toInt() and 0xff
}
