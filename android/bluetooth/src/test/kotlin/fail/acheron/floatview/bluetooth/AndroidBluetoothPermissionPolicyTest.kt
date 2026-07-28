package fail.acheron.floatview.bluetooth

import org.junit.Assert.assertEquals
import org.junit.Test

class AndroidBluetoothPermissionPolicyTest {
    @Test
    fun `android 10 requires location to scan but no runtime connect permission`() {
        assertEquals(
            AndroidBluetoothPermissionPolicy(
                AndroidBluetoothPermissionPolicy.FINE_LOCATION,
                null,
            ),
            AndroidBluetoothPermissionPolicy.forApi(29),
        )
    }

    @Test
    fun `android 12 and newer require scan and connect runtime permissions`() {
        val expected = AndroidBluetoothPermissionPolicy(
            AndroidBluetoothPermissionPolicy.BLUETOOTH_SCAN,
            AndroidBluetoothPermissionPolicy.BLUETOOTH_CONNECT,
        )
        assertEquals(expected, AndroidBluetoothPermissionPolicy.forApi(31))
        assertEquals(expected, AndroidBluetoothPermissionPolicy.forApi(36))
    }
}
