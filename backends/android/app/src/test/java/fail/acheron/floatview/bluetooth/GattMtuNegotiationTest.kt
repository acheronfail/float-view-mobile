package fail.acheron.floatview.bluetooth

import org.junit.Assert.*
import org.junit.Test

class GattMtuNegotiationTest {
    @Test fun requestedMtuPreservesExpressNotificationFragments() {
        // Express advances by min(MTU, 255), but ATT can deliver only MTU - 3.
        // Model the observed schema/motor-limit frame sizes without board data.
        fun received(frame: ByteArray, mtu: Int): ByteArray = frame.toList()
            .chunked(minOf(mtu, 255))
            .flatMap { it.take(mtu - 3) }
            .toByteArray()

        for (size in listOf(416, 490)) {
            val frame = ByteArray(size) { (it % 251).toByte() }
            val truncated = received(frame, 247)
            assertEquals(size - 3, truncated.size)
            assertArrayEquals(frame.copyOfRange(0, 244), truncated.copyOfRange(0, 244))
            assertEquals(frame[247], truncated[244])
            assertArrayEquals(frame, received(frame, GattMtuNegotiation.REQUESTED_MTU))
        }
    }

    @Test fun confirmedMtuSuppliesPayloadCapacity() {
        for ((value, expected) in listOf(23 to 20, 185 to 182, 247 to 244, 517 to 512)) {
            val mtu = GattMtuNegotiation()
            assertTrue(mtu.begin())
            assertTrue(mtu.complete(value, true))
            assertEquals(expected, mtu.payloadBytes)
        }
    }

    @Test fun rejectedFailedOrInvalidNegotiationRetainsDefaultPayload() {
        for ((value, success) in listOf(247 to false, 0 to true, 22 to true, 518 to true)) {
            val mtu = GattMtuNegotiation()
            assertTrue(mtu.begin())
            assertTrue(mtu.complete(value, success))
            assertEquals(20, mtu.payloadBytes)
        }
    }

    @Test fun completionStartsDiscoveryOnlyOnce() {
        val mtu = GattMtuNegotiation()
        assertFalse(mtu.complete(247, true))
        assertTrue(mtu.begin())
        assertFalse(mtu.begin())
        assertTrue(mtu.complete(185, true))
        assertFalse(mtu.complete(247, true))
        assertEquals(182, mtu.payloadBytes)
    }

    @Test fun canceledConnectionIgnoresLateResultsAndReplacementStartsAtDefault() {
        val prior = GattMtuNegotiation()
        assertTrue(prior.begin())
        prior.close()
        assertFalse(prior.complete(247, true))
        assertFalse(prior.begin())
        val replacement = GattMtuNegotiation()
        assertEquals(20, replacement.payloadBytes)
        assertTrue(replacement.begin())
        assertTrue(replacement.complete(247, true))
        assertEquals(244, replacement.payloadBytes)
    }
}
