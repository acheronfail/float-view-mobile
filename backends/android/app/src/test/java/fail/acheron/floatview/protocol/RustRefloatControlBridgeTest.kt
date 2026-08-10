package fail.acheron.floatview.protocol

import fail.acheron.floatview.protocol.refloat.RefloatControlCodec
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class RustRefloatControlBridgeTest {
    @Test
    fun abiAndExactOperationsAreLinked() {
        assertEquals(1, RustRefloatControlBridge.abiVersion())
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            verifyRefloatControlAbiVersion(2)
        }
        assertArrayEquals(byteArrayOf(36, 101, 12, 0), encode(1, 0, 0, 0, 1, 0))
        assertArrayEquals(byteArrayOf(36, 101, 22, 128.toByte(), 0, 0, 0, 0, 1), encode(2, 0, 0, 0, 0, 0))
        assertArrayEquals(byteArrayOf(36, 101, 36, 1), encode(4, 0, 1, 0, 0, 0))
    }

    @Test
    fun exhaustiveLightingAndResponseStatesCrossJni() {
        for ((profile, command) in listOf(1 to 202, 2 to 20)) {
            for (current in 0..3) {
                for (target in 0..2) {
                    for (enabled in 0..1) {
                        val output = ByteArray(9)
                        val result = IntArray(2)
                        val status = RustRefloatControlBridge.encode(
                            3, profile, 0, target, enabled, current, output, result,
                        )
                        if (target == 0 && enabled == 1) {
                            assertEquals(1, status)
                        } else {
                            assertEquals(0, status)
                            assertEquals(command, output[2].toInt() and 0xff)
                            assertEquals(target, output[result[0] - 2].toInt() and 0xff)
                        }
                    }
                }
            }
            for (flags in 0..3) {
                val decoded = IntArray(1)
                assertEquals(
                    0,
                    RustRefloatControlBridge.decodeLighting(
                        byteArrayOf(36, 101, command.toByte(), flags.toByte()), profile, decoded,
                    ),
                )
                assertArrayEquals(intArrayOf(flags), decoded)
            }
        }
    }

    @Test
    fun rejectsInvalidReferencesShapesCapacityAndPrimitives() {
        val output = ByteArray(9)
        assertEquals(1, RustRefloatControlBridge.encode(1, 0, 0, 0, 1, 0, null, IntArray(2)))
        assertEquals(1, RustRefloatControlBridge.encode(1, 0, 0, 0, 1, 0, output, null))
        assertEquals(1, RustRefloatControlBridge.encode(1, 0, 0, 0, 1, 0, output, IntArray(1)))
        assertEquals(1, RustRefloatControlBridge.encode(1, 0, 0, 0, 1, 0, ByteArray(10), IntArray(2)))
        assertEquals(1, RustRefloatControlBridge.encode(1, 0, 0, 0, 2, 0, output, IntArray(2)))
        assertEquals(1, RustRefloatControlBridge.encode(3, 0, 0, 1, 0, 0, output, IntArray(2)))
        assertEquals(1, RustRefloatControlBridge.decodeLighting(null, 2, IntArray(1)))
        assertEquals(1, RustRefloatControlBridge.decodeLighting(byteArrayOf(36), 2, null))
        assertEquals(1, RustRefloatControlBridge.decodeLighting(byteArrayOf(36), 2, IntArray(2)))
        assertEquals(2, RustRefloatControlBridge.decodeLighting(ByteArray(5), 2, IntArray(1)))
    }

    @Test
    fun reportsUnsupportedCapacityAndAdapterResultFailures() {
        val unsupported = IntArray(2)
        assertEquals(3, RustRefloatControlBridge.encode(4, 0, 0, 0, 0, 0, ByteArray(9), unsupported))
        assertArrayEquals(intArrayOf(0, 36), unsupported)
        val capacity = IntArray(2)
        assertEquals(4, RustRefloatControlBridge.encode(2, 0, 0, 0, 1, 0, ByteArray(8), capacity))
        assertArrayEquals(intArrayOf(9, 0), capacity)

        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            RefloatControlCodec.encodeBridgeResult(0, ByteArray(9), intArrayOf(0, 0))
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            RefloatControlCodec.encodeBridgeResult(0, ByteArray(9), intArrayOf(1, 1))
        }
        assertThrows(VescProtocolException.UnsupportedCommand::class.java) {
            RefloatControlCodec.encodeBridgeResult(3, ByteArray(9), intArrayOf(0, 36))
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            RefloatControlCodec.decodeBridgeResult(0, intArrayOf(4))
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            RefloatControlCodec.decodeBridgeResult(5, intArrayOf(0))
        }
    }

    private fun encode(
        operation: Int,
        profile: Int,
        fatal: Int,
        target: Int,
        enabled: Int,
        current: Int,
    ): ByteArray {
        val output = ByteArray(9)
        val result = IntArray(2)
        assertEquals(
            0,
            RustRefloatControlBridge.encode(
                operation, profile, fatal, target, enabled, current, output, result,
            ),
        )
        return output.copyOf(result[0])
    }
}
