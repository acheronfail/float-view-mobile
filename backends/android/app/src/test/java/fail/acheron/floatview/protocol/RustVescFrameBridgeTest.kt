package fail.acheron.floatview.protocol

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class RustVescFrameBridgeTest {
    @Test
    fun `raw bridge reports its exact ABI and rejects a mismatched adapter version`() {
        assertEquals(2, RustVescFrameBridge.abiVersion())
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            verifyVescFrameAbiVersion(1)
        }
    }

    @Test
    fun `raw bridge validates null aliases length shapes and excessive capacities`() {
        val validInput = byteArrayOf(1)
        val output = ByteArray(6) { 0x5a }

        assertEquals(1, RustVescFrameBridge.encode(null, output, IntArray(1)))
        assertEquals(1, RustVescFrameBridge.encode(validInput, null, IntArray(1)))
        assertEquals(1, RustVescFrameBridge.encode(validInput, output, null))
        assertEquals(1, RustVescFrameBridge.encode(validInput, output, IntArray(0)))
        assertEquals(1, RustVescFrameBridge.encode(validInput, output, IntArray(2)))
        assertEquals(1, RustVescFrameBridge.encode(validInput, validInput, IntArray(1)))
        assertEquals(
            1,
            RustVescFrameBridge.encode(validInput, ByteArray(4_103), IntArray(1)),
        )
        assertArrayEquals(ByteArray(6) { 0x5a }, output)
    }

    @Test
    fun `raw bridge initializes lengths and never partially writes failures`() {
        val unchanged = ByteArray(8) { 0x5a }
        val length = intArrayOf(99)
        assertEquals(2, RustVescFrameBridge.encode(ByteArray(4_097), unchanged, length))
        assertEquals(0, length.single())
        assertArrayEquals(ByteArray(8) { 0x5a }, unchanged)

        length[0] = 99
        assertEquals(5, RustVescFrameBridge.encode(byteArrayOf(1), ByteArray(0), length))
        assertEquals(6, length.single())

        length[0] = 99
        assertEquals(3, RustVescFrameBridge.decodeExact(byteArrayOf(2), unchanged, length))
        assertEquals(0, length.single())
        assertArrayEquals(ByteArray(8) { 0x5a }, unchanged)
    }

    @Test
    fun `raw scanner validates shapes preserves input and reports exact boundaries`() {
        val payload = byteArrayOf(0x11, 0x22)
        val frame = VescFrameCodec.encode(payload)
        val input = byteArrayOf(0xff.toByte(), 1) + frame + frame
        val immutableInput = input.copyOf()
        val result = intArrayOf(99, 99, 99)

        assertEquals(1, RustVescFrameBridge.scanNext(null, result))
        assertArrayEquals(intArrayOf(99, 99, 99), result)
        assertEquals(1, RustVescFrameBridge.scanNext(input, null))
        assertEquals(1, RustVescFrameBridge.scanNext(input, IntArray(0)))
        assertEquals(1, RustVescFrameBridge.scanNext(input, IntArray(2)))
        assertEquals(1, RustVescFrameBridge.scanNext(input, IntArray(4)))

        assertEquals(0, RustVescFrameBridge.scanNext(input, result))
        assertArrayEquals(intArrayOf(2, frame.size, 2 + frame.size), result)
        assertArrayEquals(immutableInput, input)

        val incomplete = intArrayOf(99, 99, 99)
        assertEquals(7, RustVescFrameBridge.scanNext(byteArrayOf(3, 0), incomplete))
        assertArrayEquals(intArrayOf(0, 0, 0), incomplete)

        val noise = intArrayOf(99, 99, 99)
        assertEquals(7, RustVescFrameBridge.scanNext(byteArrayOf(0xff.toByte(), 0), noise))
        assertArrayEquals(intArrayOf(0, 0, 2), noise)
    }

    @Test
    fun `raw scanner rejects declarations and aggregate overflow with exact progress`() {
        val oversizedDeclaration = intArrayOf(99, 99, 99)
        assertEquals(
            2,
            RustVescFrameBridge.scanNext(byteArrayOf(3, 0x10, 0x01), oversizedDeclaration),
        )
        assertArrayEquals(intArrayOf(0, 0, 1), oversizedDeclaration)

        val aggregate = ByteArray(VescFrameCodec.MAXIMUM_PAYLOAD_BYTES * 2 + 1) { 1 }
        val aggregateResult = intArrayOf(99, 99, 99)
        assertEquals(2, RustVescFrameBridge.scanNext(aggregate, aggregateResult))
        assertArrayEquals(intArrayOf(0, 0, aggregate.size), aggregateResult)
    }

    @Test
    fun `public codec covers empty short long and maximum payloads through JNI`() {
        listOf(
            ByteArray(0),
            byteArrayOf(0x11, 0x22, 0x33),
            ByteArray(256) { (it and 0xff).toByte() },
            ByteArray(VescFrameCodec.MAXIMUM_PAYLOAD_BYTES) { (it and 0xff).toByte() },
        ).forEach { payload ->
            assertArrayEquals(payload, VescFrameCodec.decode(VescFrameCodec.encode(payload)))
        }
    }

    @Test
    fun `public codec preserves established semantic errors`() {
        assertThrows(VescProtocolException.PayloadTooLarge::class.java) {
            VescFrameCodec.encode(ByteArray(VescFrameCodec.MAXIMUM_PAYLOAD_BYTES + 1))
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            VescFrameCodec.decode(byteArrayOf(1))
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            VescFrameCodec.decode(byteArrayOf(2, 0, 0, 0, 3, 0))
        }

        val invalidFooter = VescFrameCodec.encode(byteArrayOf(1))
        invalidFooter[invalidFooter.lastIndex] = 0
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            VescFrameCodec.decode(invalidFooter)
        }

        val invalidCrc = VescFrameCodec.encode(byteArrayOf(1))
        invalidCrc[invalidCrc.lastIndex - 2] = (invalidCrc[invalidCrc.lastIndex - 2].toInt() xor 0xff).toByte()
        assertThrows(VescProtocolException.InvalidCrc::class.java) {
            VescFrameCodec.decode(invalidCrc)
        }
    }

    @Test
    fun `stream scanner preserves fragmentation noise and ordering`() {
        val shortPayload = byteArrayOf(0, 6, 6, 0)
        val shortFrame = VescFrameCodec.encode(shortPayload)
        val longPayload = ByteArray(256) { (it and 0xff).toByte() }
        val longFrame = VescFrameCodec.encode(longPayload)
        val fragments = buildList {
            add(byteArrayOf(0xff.toByte(), 1))
            shortFrame.forEach { add(byteArrayOf(it)) }
            add(longFrame.copyOfRange(0, 1))
            add(longFrame.copyOfRange(1, 2))
            add(longFrame.copyOfRange(2, 19))
            add(longFrame.copyOfRange(19, longFrame.size))
            add(shortFrame + shortFrame)
        }
        val rust = VescFrameStreamDecoder()
        val rustPayloads = mutableListOf<ByteArray>()

        fragments.forEach { fragment ->
            rustPayloads += rust.append(fragment)
        }

        assertEquals(4, rustPayloads.size)
        assertArrayEquals(shortPayload, rustPayloads[0])
        assertArrayEquals(longPayload, rustPayloads[1])
        assertArrayEquals(shortPayload, rustPayloads[2])
        assertArrayEquals(shortPayload, rustPayloads[3])
    }

    @Test
    fun `stream scanner consumes malformed and oversized input before recovery`() {
        val valid = VescFrameCodec.encode(byteArrayOf(0x11, 0x22))

        val badFooter = valid.copyOf().also { it[it.lastIndex] = 0 }
        val footerStream = VescFrameStreamDecoder()
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            footerStream.append(badFooter)
        }
        assertArrayEquals(byteArrayOf(0x11, 0x22), footerStream.append(valid).single())

        val badCrc = valid.copyOf().also {
            it[it.lastIndex - 2] = (it[it.lastIndex - 2].toInt() xor 0xff).toByte()
        }
        val crcStream = VescFrameStreamDecoder()
        assertThrows(VescProtocolException.InvalidCrc::class.java) {
            crcStream.append(badCrc)
        }
        assertArrayEquals(byteArrayOf(0x11, 0x22), crcStream.append(valid).single())

        val declarationStream = VescFrameStreamDecoder()
        assertThrows(VescProtocolException.PayloadTooLarge::class.java) {
            declarationStream.append(byteArrayOf(3, 0x10, 0x01))
        }
        assertArrayEquals(byteArrayOf(0x11, 0x22), declarationStream.append(valid).single())

        val aggregateStream = VescFrameStreamDecoder()
        assertThrows(VescProtocolException.PayloadTooLarge::class.java) {
            aggregateStream.append(ByteArray(VescFrameCodec.MAXIMUM_PAYLOAD_BYTES * 2 + 1) { 1 })
        }
        assertArrayEquals(byteArrayOf(0x11, 0x22), aggregateStream.append(valid).single())
    }

    @Test
    fun `stream adapter rejects impossible native results`() {
        listOf(
            intArrayOf(0, 0) to 7,
            intArrayOf(-1, 6, 5) to 0,
            intArrayOf(0, 0, 1) to 0,
            intArrayOf(1, 6, 6) to 0,
            intArrayOf(0, 0, 9) to 7,
            intArrayOf(1, 0, 1) to 7,
            intArrayOf(0, 0, 0) to 2,
            intArrayOf(0, 0, 9) to 2,
        ).forEach { (fields, status) ->
            assertThrows(VescProtocolException.InvalidFrame::class.java) {
                VescFrameCodec.decodeStreamScan(status, fields, 8)
            }
        }
    }
}
