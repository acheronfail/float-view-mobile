package fail.acheron.floatview.bluetooth

import fail.acheron.floatview.backend.BluetoothEvent
import fail.acheron.floatview.backend.BluetoothPermissionState
import fail.acheron.floatview.backend.BluetoothPort
import fail.acheron.floatview.backend.BluetoothRadioState
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.DeviceId

/**
 * Deterministic emulator adapter. It exercises the same application service and
 * loopback contract without pretending to test Android's physical BLE stack.
 */
class SoftwareBluetoothAdapter(
    private val emit: (BluetoothEvent) -> Unit,
    private val response: (ByteArray) -> ByteArray? = { null },
) : BluetoothPort {
    private val state = BluetoothState(
        BluetoothRadioState.POWERED_ON,
        BluetoothPermissionState.GRANTED,
        BluetoothPermissionState.GRANTED,
    )

    override fun currentState() = state

    override fun requestPermission() {
        emit(BluetoothEvent.StateChanged(state))
    }

    override fun startDiscovery() {
        emit(BluetoothEvent.DeviceFound(DEVICE_ID, "Float View Simulator", -42))
    }

    override fun stopDiscovery() = Unit

    override fun connect(deviceId: DeviceId) {
        require(deviceId == DEVICE_ID)
        emit(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.CONNECTING))
        emit(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.READY))
    }

    override fun write(deviceId: DeviceId, bytes: ByteArray) {
        response(bytes)?.let {
            emit(BluetoothEvent.Notification(deviceId, it))
        }
    }

    override fun disconnect(deviceId: DeviceId) {
        emit(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED))
    }

    private companion object {
        val DEVICE_ID = DeviceId("software-vesc")
    }
}
