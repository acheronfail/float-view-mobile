package fail.acheron.floatview.bluetooth

import fail.acheron.floatview.core.BluetoothPermissionState

/**
 * Runtime permission requirements only. Manifest-only permissions are declared
 * by the host but intentionally surface as NOT_REQUIRED in the domain API.
 */
data class AndroidBluetoothPermissionPolicy(
    val scanPermission: String?,
    val connectPermission: String?,
) {
    companion object {
        const val FINE_LOCATION = "android.permission.ACCESS_FINE_LOCATION"
        const val BLUETOOTH_SCAN = "android.permission.BLUETOOTH_SCAN"
        const val BLUETOOTH_CONNECT = "android.permission.BLUETOOTH_CONNECT"

        fun forApi(apiLevel: Int): AndroidBluetoothPermissionPolicy =
            if (apiLevel >= 31) {
                AndroidBluetoothPermissionPolicy(BLUETOOTH_SCAN, BLUETOOTH_CONNECT)
            } else {
                AndroidBluetoothPermissionPolicy(FINE_LOCATION, null)
            }

        fun state(
            permission: String?,
            granted: Boolean,
            requestResolved: Boolean,
        ): BluetoothPermissionState = when {
            permission == null -> BluetoothPermissionState.NOT_REQUIRED
            granted -> BluetoothPermissionState.GRANTED
            requestResolved -> BluetoothPermissionState.DENIED
            else -> BluetoothPermissionState.NOT_DETERMINED
        }
    }
}
