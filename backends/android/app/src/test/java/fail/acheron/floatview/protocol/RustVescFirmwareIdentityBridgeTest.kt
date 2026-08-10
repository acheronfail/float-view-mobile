package fail.acheron.floatview.protocol

import fail.acheron.floatview.protocol.vesc.VescFirmwareIdentityDecoder
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Test

class RustVescFirmwareIdentityBridgeTest {
    @Test
    fun `raw bridge reports exact ABI and rejects a mismatched adapter version`() {
        assertEquals(1, RustVescFirmwareIdentityBridge.abiVersion())
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            verifyVescFirmwareIdentityAbiVersion(2)
        }
    }

    @Test
    fun `raw bridge validates null aliases shapes and excessive capacities`() {
        val input = stockBmsPayload()
        val output = ByteArray(410) { 0x5a }
        val unchanged = output.copyOf()

        assertEquals(1, RustVescFirmwareIdentityBridge.decode(null, output, IntArray(2)))
        assertEquals(1, RustVescFirmwareIdentityBridge.decode(input, null, IntArray(2)))
        assertEquals(1, RustVescFirmwareIdentityBridge.decode(input, output, null))
        assertEquals(1, RustVescFirmwareIdentityBridge.decode(input, output, IntArray(0)))
        assertEquals(1, RustVescFirmwareIdentityBridge.decode(input, output, IntArray(1)))
        assertEquals(1, RustVescFirmwareIdentityBridge.decode(input, output, IntArray(3)))
        assertEquals(1, RustVescFirmwareIdentityBridge.decode(input, input, IntArray(2)))
        assertEquals(1, RustVescFirmwareIdentityBridge.decode(input, ByteArray(411), IntArray(2)))
        assertArrayEquals(unchanged, output)
    }

    @Test
    fun `raw bridge initializes metadata and never partially writes failures`() {
        val unchanged = ByteArray(410) { 0x5a }
        val metadata = intArrayOf(99, 99)

        assertEquals(3, RustVescFirmwareIdentityBridge.decode(ByteArray(158), unchanged, metadata))
        assertArrayEquals(intArrayOf(0, 0), metadata)
        assertArrayEquals(ByteArray(410) { 0x5a }, unchanged)

        metadata[0] = 99
        metadata[1] = 99
        assertEquals(2, RustVescFirmwareIdentityBridge.decode(byteArrayOf(37), unchanged, metadata))
        assertArrayEquals(intArrayOf(0, 37), metadata)
        assertArrayEquals(ByteArray(410) { 0x5a }, unchanged)

        metadata[0] = 99
        metadata[1] = 99
        assertEquals(4, RustVescFirmwareIdentityBridge.decode(stockBmsPayload(), ByteArray(2), metadata))
        assertArrayEquals(intArrayOf(30, 0), metadata)

        metadata[0] = 99
        metadata[1] = 99
        assertEquals(3, RustVescFirmwareIdentityBridge.decode(byteArrayOf(0), unchanged, metadata))
        assertArrayEquals(intArrayOf(0, 0), metadata)
        assertArrayEquals(ByteArray(410) { 0x5a }, unchanged)
    }

    @Test
    fun `public decoder maps the compiled record and replacement UTF8`() {
        val stock = VescFirmwareIdentityDecoder.decode(stockBmsPayload())
        assertEquals(6, stock.major)
        assertEquals(0, stock.minor)
        assertEquals("VBMS", stock.hardwareName)
        assertEquals(1, stock.hardwareType)
        assertEquals(1, stock.customConfigurationCount)
        assertArrayEquals(ByteArray(12) { it.toByte() }, stock.controllerUuid)

        val replacement = VescFirmwareIdentityDecoder.decode(replacementPayload())
        assertEquals("H\uFFFD", replacement.hardwareName)
        assertEquals("F\uFFFD", replacement.firmwareName)
        assertEquals(9, replacement.testVersion)
        assertEquals(2, replacement.hardwareType)
        assertEquals(7, replacement.customConfigurationCount)
        assertEquals(0x1234_5678L, replacement.hardwareConfigurationCrc)
    }

    @Test
    fun `adapter preserves unsupported and structural error classes without private output`() {
        val unsupported = assertThrows(VescProtocolException.UnsupportedCommand::class.java) {
            VescFirmwareIdentityDecoder.decode(byteArrayOf(37))
        }
        assertEquals(37, unsupported.command)

        val privatePayload = replacementPayload().also {
            "SECRET-HARDWARE".encodeToByteArray().copyInto(it, destinationOffset = 3)
        }
        val malformed = assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            VescFirmwareIdentityDecoder.decode(privatePayload)
        }
        assertFalse(malformed.toString().contains("SECRET"))
    }

    @Test
    fun `adapter rejects impossible statuses metadata and records`() {
        val validRecord = stockRecord()
        listOf(
            Triple(0, intArrayOf(30, 1), validRecord),
            Triple(0, intArrayOf(411, 0), ByteArray(410)),
            Triple(2, intArrayOf(0, 0), validRecord),
            Triple(2, intArrayOf(1, 37), validRecord),
            Triple(99, intArrayOf(0, 0), validRecord),
        ).forEach { (status, result, record) ->
            assertThrows(VescProtocolException::class.java) {
                VescFirmwareIdentityDecoder.decodeBridgeResult(status, record, result)
            }
        }

        val malformedRecords = listOf(
            ByteArray(25),
            validRecord.copyOf().also { it[5] = 4 },
            validRecord.copyOf().also { it[7] = 5 },
            validRecord.copyOf().also { it[5] = 1 },
            validRecord.copyOf().also { it[22] = 1 },
            validRecord.copyOf().also {
                it[7] = 1
                it[26] = 0xff.toByte()
            }.copyOf(27),
        )
        malformedRecords.forEach { record ->
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                VescFirmwareIdentityDecoder.firmwareInfo(record)
            }
        }
    }

    private fun stockBmsPayload(): ByteArray =
        byteArrayOf(0, 6, 0) + "VBMS".encodeToByteArray() + byteArrayOf(0) +
            ByteArray(12) { it.toByte() } + byteArrayOf(0, 0, 1, 1)

    private fun replacementPayload(): ByteArray =
        byteArrayOf(0, 7, 0, 'H'.code.toByte(), 0xff.toByte(), 0) +
            ByteArray(12) { (it + 1).toByte() } + byteArrayOf(1, 9, 2, 7) +
            byteArrayOf(1, 0, 1, 0, 'F'.code.toByte(), 0xfe.toByte(), 0, 0x12, 0x34, 0x56, 0x78)

    private fun stockRecord(): ByteArray =
        byteArrayOf(6, 0, 0, 1, 1, 0, 0, 4, 0, 0) +
            ByteArray(12) { it.toByte() } + ByteArray(4) + "VBMS".encodeToByteArray()
}
