package fail.acheron.floatview.backend

/** Focused Android Bluetooth port vocabulary transported to the Rust core. */
@JvmInline
value class DeviceId(val value: String)

enum class ConnectionState {
    DISCONNECTED,
    READY,
    FAILED,
}

enum class BluetoothRadioState { UNKNOWN, UNSUPPORTED, POWERED_OFF, POWERED_ON, RESETTING }

enum class BluetoothPermissionState {
    UNKNOWN,
    NOT_REQUIRED,
    NOT_DETERMINED,
    DENIED,
    RESTRICTED,
    GRANTED,
}

data class BluetoothState(
    val radio: BluetoothRadioState,
    val scanPermission: BluetoothPermissionState,
    val connectPermission: BluetoothPermissionState,
)

sealed interface BluetoothEvent {
    data class StateChanged(val state: BluetoothState) : BluetoothEvent
    data class DeviceFound(
        val deviceId: DeviceId,
        val advertisedName: String?,
        val rssi: Int?,
    ) : BluetoothEvent
    data class ScanFailed(val reason: String) : BluetoothEvent
    data class ConnectionChanged(val deviceId: DeviceId, val state: ConnectionState) : BluetoothEvent
    data class Notification(val deviceId: DeviceId, val bytes: ByteArray) : BluetoothEvent
}

interface BluetoothPort {
    fun currentState(): BluetoothState
    fun requestPermission()
    fun startDiscovery()
    fun stopDiscovery()
    fun connect(deviceId: DeviceId)
    fun write(deviceId: DeviceId, bytes: ByteArray)
    fun disconnect(deviceId: DeviceId)
}
