package fail.acheron.floatview.protocol

import fail.acheron.floatview.protocol.vesc.VescBmsValuesDecoder
import java.time.Instant
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Test

class RustVescBmsValuesBridgeTest {
    @Test
    fun `raw bridge reports exact ABI and rejects a mismatched adapter version`() {
        assertEquals(1, RustVescBmsValuesBridge.abiVersion())
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            verifyVescBmsValuesAbiVersion(2)
        }
    }

    @Test
    fun `raw bridge validates null aliases shapes capacities and expected CAN`() {
        val input = payload(0, 0, null)
        val output = ByteArray(MAXIMUM_RECORD_BYTES) { 0x5a }
        val unchanged = output.copyOf()

        assertEquals(1, RustVescBmsValuesBridge.decode(null, 42, output, IntArray(2)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 42, null, IntArray(2)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 42, output, null))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 42, output, IntArray(0)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 42, output, IntArray(1)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 42, output, IntArray(3)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 42, input, IntArray(2)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 42, ByteArray(1_035), IntArray(2)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, -1, output, IntArray(2)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 254, output, IntArray(2)))
        assertEquals(1, RustVescBmsValuesBridge.decode(input, 256, output, IntArray(2)))
        assertArrayEquals(unchanged, output)
    }

    @Test
    fun `raw bridge initializes metadata and never partially writes failures`() {
        val output = ByteArray(MAXIMUM_RECORD_BYTES) { 0x5a }
        val unchanged = output.copyOf()
        val metadata = intArrayOf(99, 99)

        assertEquals(3, RustVescBmsValuesBridge.decode(ByteArray(351), 42, output, metadata))
        assertArrayEquals(intArrayOf(0, 0), metadata)
        assertArrayEquals(unchanged, output)

        metadata.fill(99)
        assertEquals(2, RustVescBmsValuesBridge.decode(byteArrayOf(37), 42, output, metadata))
        assertArrayEquals(intArrayOf(0, 37), metadata)
        assertArrayEquals(unchanged, output)

        metadata.fill(99)
        assertEquals(4, RustVescBmsValuesBridge.decode(payload(0, 0, null), 42, ByteArray(2), metadata))
        assertArrayEquals(intArrayOf(MINIMUM_RECORD_BYTES, 0), metadata)

        metadata.fill(99)
        assertEquals(3, RustVescBmsValuesBridge.decode(payload(0, 0, null), 41, output, metadata))
        assertArrayEquals(intArrayOf(0, 0), metadata)
        assertArrayEquals(unchanged, output)
    }

    @Test
    fun `public decoder maps exact records and observation time`() {
        val observedAt = Instant.ofEpochSecond(1_234)
        val legacy = VescBmsValuesDecoder.decode(payload(2, 1, null), 42, observedAt)
        assertEquals(observedAt, legacy.observedAt)
        assertEquals(50.4, legacy.totalVoltageV, 0.0)
        assertEquals(-12.345, legacy.inputCurrentA, 0.0)
        assertEquals(listOf(4.0, 4.001), legacy.cellVoltagesV)
        assertEquals(listOf(false, true), legacy.balancing)
        assertEquals(listOf(20.0), legacy.temperaturesC)
        assertEquals(42, legacy.reportedCanId)
        assertEquals(101_320.0, legacy.pressurePa, 0.001)
        assertEquals(null, legacy.dataVersion)
        assertEquals(null, legacy.status)

        val extended = VescBmsValuesDecoder.decode(payload(0, 0, 7 to "OK".encodeToByteArray()), null, observedAt)
        assertEquals(7, extended.dataVersion)
        assertEquals("OK", extended.status)
    }

    @Test
    fun `adapter rejects impossible statuses metadata and records without private output`() {
        val observedAt = Instant.EPOCH
        val valid = record(payload(0, 0, null))
        listOf(
            Triple(0, intArrayOf(valid.size, 1), valid),
            Triple(0, intArrayOf(1_035, 0), ByteArray(MAXIMUM_RECORD_BYTES)),
            Triple(2, intArrayOf(1, 37), valid),
            Triple(2, intArrayOf(0, 256), valid),
            Triple(99, intArrayOf(0, 0), valid),
        ).forEach { (status, result, candidate) ->
            assertThrows(VescProtocolException::class.java) {
                VescBmsValuesDecoder.decodeBridgeResult(status, candidate, result, observedAt)
            }
        }

        val oneCell = record(payload(1, 0, null))
        val extended = record(payload(0, 0, 1 to "OK".encodeToByteArray()))
        val malformed = listOf(
            ByteArray(MINIMUM_RECORD_BYTES - 1),
            valid.copyOf().also { it[0] = 2 },
            valid.copyOf().also { it[1] = 2 },
            valid.copyOf().also { it[2] = 51 },
            valid.copyOf().also { it[3] = 51 },
            valid.copyOf().also { it[4] = 254.toByte() },
            valid.copyOf().also { it[5] = 1 },
            valid.copyOf().also { it[6] = 41 },
            valid.copyOf().also { it[7] = 1 },
            valid.copyOf().also {
                byteArrayOf(0x7f, 0xf8.toByte(), 0, 0, 0, 0, 0, 0).copyInto(it, 8)
            },
            oneCell.copyOf().also { it[MINIMUM_RECORD_BYTES + Long.SIZE_BYTES] = 2 },
            extended.copyOf().also { it[it.lastIndex] = 0xff.toByte() },
        )
        malformed.forEach { candidate ->
            val failure = assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                VescBmsValuesDecoder.snapshot(candidate, observedAt)
            }
            assertFalse(failure.toString().contains("OK"))
        }
    }

    private fun record(input: ByteArray): ByteArray {
        val output = ByteArray(MAXIMUM_RECORD_BYTES)
        val result = IntArray(2)
        assertEquals(0, RustVescBmsValuesBridge.decode(input, 42, output, result))
        assertEquals(0, result[1])
        return output.copyOf(result[0])
    }

    private fun payload(
        cells: Int,
        temperatures: Int,
        extended: Pair<Int, ByteArray>?,
    ): ByteArray = buildList<Byte> {
        add(96)
        listOf(50_400_000, 54_600_000, -12_345_000, -12_300_000, 1_250, 62_500).forEach { value ->
            addAll(value.toByteArray().asList())
        }
        add(cells.toByte())
        repeat(cells) { index -> addAll((4_000 + index).toShort().toByteArray().asList()) }
        repeat(cells) { index -> add((index % 2).toByte()) }
        add(temperatures.toByte())
        repeat(temperatures) { index -> addAll((2_000 + index).toShort().toByteArray().asList()) }
        listOf<Short>(2_750, 2_475, 5_250, 2_925, 875, 950).forEach { value ->
            addAll(value.toByteArray().asList())
        }
        add(42)
        listOf(0x4280_0000, 0x4480_0000, 0x4200_0000, 0x4400_0000).forEach { raw ->
            addAll(raw.toByteArray().asList())
        }
        addAll(10_132.toShort().toByteArray().asList())
        extended?.let { (version, status) ->
            add(version.toByte())
            addAll(status.asList())
            add(0)
        }
    }.toByteArray()

    private fun Int.toByteArray(): ByteArray = byteArrayOf(
        (this ushr 24).toByte(),
        (this ushr 16).toByte(),
        (this ushr 8).toByte(),
        toByte(),
    )

    private fun Short.toByteArray(): ByteArray = byteArrayOf(
        (toInt() ushr 8).toByte(),
        toByte(),
    )

    private companion object {
        const val MAXIMUM_RECORD_BYTES = 1_034
        const val MINIMUM_RECORD_BYTES = 8 + 17 * Long.SIZE_BYTES
    }
}
