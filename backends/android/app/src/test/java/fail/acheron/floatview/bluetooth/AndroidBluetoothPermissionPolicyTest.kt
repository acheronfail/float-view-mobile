package fail.acheron.floatview.bluetooth

import fail.acheron.floatview.backend.BluetoothPermissionState
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

    @Test
    fun `missing permission becomes denied only after a request resolves`() {
        assertEquals(
            BluetoothPermissionState.NOT_DETERMINED,
            AndroidBluetoothPermissionPolicy.state(
                AndroidBluetoothPermissionPolicy.BLUETOOTH_SCAN,
                granted = false,
                requestResolved = false,
            ),
        )
        assertEquals(
            BluetoothPermissionState.DENIED,
            AndroidBluetoothPermissionPolicy.state(
                AndroidBluetoothPermissionPolicy.BLUETOOTH_SCAN,
                granted = false,
                requestResolved = true,
            ),
        )
        assertEquals(
            BluetoothPermissionState.GRANTED,
            AndroidBluetoothPermissionPolicy.state(
                AndroidBluetoothPermissionPolicy.BLUETOOTH_SCAN,
                granted = true,
                requestResolved = true,
            ),
        )
        assertEquals(
            BluetoothPermissionState.NOT_REQUIRED,
            AndroidBluetoothPermissionPolicy.state(
                permission = null,
                granted = false,
                requestResolved = false,
            ),
        )
    }
}
