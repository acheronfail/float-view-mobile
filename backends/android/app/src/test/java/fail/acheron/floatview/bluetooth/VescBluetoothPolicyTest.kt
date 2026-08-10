package fail.acheron.floatview.bluetooth

import fail.acheron.floatview.application.DeviceId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VescBluetoothPolicyTest {
    @Test
    fun `discovery publishes only UART or exact saved identity`() {
        val saved = DeviceId("saved")
        assertTrue(VescBluetoothPolicy.shouldPublish(true, DeviceId("unknown"), emptySet()))
        assertTrue(VescBluetoothPolicy.shouldPublish(false, saved, setOf(saved)))
        assertFalse(
            VescBluetoothPolicy.shouldPublish(false, DeviceId("unknown"), setOf(saved)),
        )
        // A restarted adapter recovers only identifiers loaded from persistence.
        assertTrue(VescBluetoothPolicy.shouldPublish(false, saved, setOf(saved)))
        assertFalse(VescBluetoothPolicy.shouldPublish(false, saved, emptySet()))
    }

    @Test
    fun `writes use default and enlarged payloads within queue bounds`() {
        val bytes = ByteArray(50) { it.toByte() }
        assertEquals(listOf(20, 20, 10), VescBluetoothPolicy.chunks(bytes, 20).map(ByteArray::size))
        assertEquals(listOf(50), VescBluetoothPolicy.chunks(bytes, 244).map(ByteArray::size))
        assertTrue(VescBluetoothPolicy.acceptsQueuedWrite(127, 65_535, 1, 1))
        assertFalse(VescBluetoothPolicy.acceptsQueuedWrite(128, 0, 1, 1))
        assertFalse(VescBluetoothPolicy.acceptsQueuedWrite(0, 65_536, 1, 1))
        assertFalse(VescBluetoothPolicy.acceptsQueuedWrite(Int.MAX_VALUE, 0, Int.MAX_VALUE, 1))
        assertTrue(
            VescBluetoothPolicy.chunks(bytes, 20)
                .fold(byteArrayOf()) { complete, chunk -> complete + chunk }
                .contentEquals(bytes),
        )
    }

    @Test
    fun `setup validates profile elements and write response modes`() {
        assertEquals(
            VescBluetoothPolicy.WriteMode.WITH_RESPONSE,
            VescBluetoothPolicy.writeMode(true, true),
        )
        assertEquals(
            VescBluetoothPolicy.WriteMode.WITHOUT_RESPONSE,
            VescBluetoothPolicy.writeMode(false, true),
        )
        assertEquals(
            VescBluetoothPolicy.WriteMode.UNSUPPORTED,
            VescBluetoothPolicy.writeMode(false, false),
        )
        assertTrue(
            VescBluetoothPolicy.writeCompletionRequired(
                VescBluetoothPolicy.WriteMode.WITH_RESPONSE,
            ),
        )
        assertFalse(
            VescBluetoothPolicy.writeCompletionRequired(
                VescBluetoothPolicy.WriteMode.WITHOUT_RESPONSE,
            ),
        )
        assertEquals(
            null,
            VescBluetoothPolicy.setupFailure(
                true,
                true,
                VescBluetoothPolicy.WriteMode.WITHOUT_RESPONSE,
                true,
                true,
            ),
        )
        assertEquals(
            "vesc_gatt_profile_unavailable",
            VescBluetoothPolicy.setupFailure(
                false,
                false,
                VescBluetoothPolicy.WriteMode.UNSUPPORTED,
                false,
                false,
            ),
        )
        assertEquals(
            "vesc_gatt_profile_unavailable",
            VescBluetoothPolicy.setupFailure(
                true,
                true,
                VescBluetoothPolicy.WriteMode.WITH_RESPONSE,
                true,
                false,
            ),
        )
    }

    @Test
    fun `cancellation and new generation reject stale callbacks`() {
        assertTrue(VescBluetoothPolicy.acceptsCallback(7, 7, false))
        assertFalse(VescBluetoothPolicy.acceptsCallback(7, 8, false))
        assertFalse(VescBluetoothPolicy.acceptsCallback(7, 7, true))
        assertFalse(VescBluetoothPolicy.acceptsCallback(7, null, false))
    }

    @Test
    fun `missing CCCD compatibility requires every other notification capability`() {
        assertEquals(
            VescBluetoothPolicy.NotificationSetup.COMPATIBILITY_READY,
            VescBluetoothPolicy.notificationSetup(
                hasNotifyProperty = true,
                localNotificationEnabled = true,
                hasDescriptor = false,
            ),
        )
        assertEquals(
            VescBluetoothPolicy.NotificationSetup.REJECT,
            VescBluetoothPolicy.notificationSetup(false, true, false),
        )
        assertEquals(
            VescBluetoothPolicy.NotificationSetup.REJECT,
            VescBluetoothPolicy.notificationSetup(true, false, false),
        )
        assertEquals(
            VescBluetoothPolicy.NotificationSetup.WRITE_DESCRIPTOR,
            VescBluetoothPolicy.notificationSetup(true, true, true),
        )
    }
}
