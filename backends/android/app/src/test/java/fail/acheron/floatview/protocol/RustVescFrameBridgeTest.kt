package fail.acheron.floatview.protocol

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class RustVescFrameBridgeTest {
    @Test
    fun `raw bridge reports its exact ABI and rejects a mismatched adapter version`() {
        assertEquals(1, RustVescFrameBridge.abiVersion())
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            verifyVescFrameAbiVersion(2)
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
}
