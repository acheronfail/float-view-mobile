package fail.acheron.floatview

import android.app.Activity
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/**
 * High-level manager for VESC device operations.
 * Handles all BLE connection, command sending, and response parsing.
 */
class VescManager(private val activity: Activity) {

    interface VescManagerListener {
        fun onScanDeviceFound(name: String?, address: String)
        fun onScanComplete()
        fun onScanError(message: String)

        fun onConnectionStatusChanged(status: ConnectionStatus, address: String? = null)
        fun onConnectionError(message: String)

        fun onFirmwareVersionReceived(command: Int, data: ByteArray)
        fun onCommandError(command: Int, message: String)
    }

    enum class ConnectionStatus {
        CONNECTING,
        CONNECTED,
        READY,
        DISCONNECTED
    }

    private val vescBle = VescBle(activity)
    private var vescCommands: VescCommands? = null
    private var listener: VescManagerListener? = null
    private val TAG = "VescManager"

    fun setListener(listener: VescManagerListener?) {
        this.listener = listener
    }

    /**
     * Start scanning for VESC devices
     */
    fun startScan(timeoutMs: Long = 3_000L): Boolean {
        if (!vescBle.requestPermissionsIfNeeded(activity, 1001)) {
            listener?.onScanError("no permissions")
            return false
        }

        vescBle.startScan(
            listOf(VescBle.VescService),
            timeoutMs,
            object : VescBle.ScanListener {
                override fun onDeviceFound(device: BluetoothDevice) {
                    listener?.onScanDeviceFound(device.name, device.address)
                }

                override fun onScanStopped(devices: List<BluetoothDevice>) {
                    Log.d(TAG, "BLE scan complete. Devices count: ${devices.size}")
                    listener?.onScanComplete()
                }

                override fun onError(message: String) {
                    Log.e(TAG, "BLE scan error: $message")
                    listener?.onScanError(message)
                }
            }
        )
        return true
    }

    /**
     * Connect to a VESC device and get firmware version
     */
    fun connectAndGetFirmware(address: String) {
        listener?.onConnectionStatusChanged(ConnectionStatus.CONNECTING)

        val didStartConnecting = vescBle.connect(
            address,
            listOf(VescBle.VescService),
            object : VescBle.ConnectionListener {
                override fun onConnected(gatt: BluetoothGatt) {
                    Log.d(TAG, "Connected to device: ${gatt.device.address}")
                    listener?.onConnectionStatusChanged(ConnectionStatus.CONNECTED)
                }

                override fun onReady(gatt: BluetoothGatt) {
                    Log.d(TAG, "Device ready: ${gatt.device.address}")
                    listener?.onConnectionStatusChanged(ConnectionStatus.READY, address)

                    // Initialize VESC commands and get firmware version
                    initializeVescCommands(gatt)
                }

                override fun onDisconnected() {
                    Log.d(TAG, "Disconnected from device")
                    listener?.onConnectionStatusChanged(ConnectionStatus.DISCONNECTED, address)
                    vescCommands?.cleanup()
                    vescCommands = null
                }

                override fun onError(message: String) {
                    Log.e(TAG, "Connection error: $message")
                    listener?.onConnectionError(message)
                    vescCommands?.cleanup()
                    vescCommands = null
                }

                override fun onCharacteristicChanged(
                    characteristic: BluetoothGattCharacteristic,
                    value: ByteArray
                ) {
                    // Forward to VescCommands for processing
                    vescCommands?.onCharacteristicChanged(characteristic, value)
                }
            }
        )

        if (!didStartConnecting) {
            listener?.onConnectionError("no device found")
        }
    }

    /**
     * Initialize VESC commands and request firmware version
     */
    private fun initializeVescCommands(gatt: BluetoothGatt) {
        val vescCommands = VescCommands(gatt)
        this.vescCommands = vescCommands

        // Check if GATT connection is ready
        if (!vescCommands.isReady()) {
            Log.e(TAG, "GATT connection not ready for VESC commands")
            disconnect()
            return
        }

        // Request larger MTU for better performance
        vescCommands.requestMtu(247)

        if (!vescCommands.enableNotifications()) {
            Log.e(TAG, "Failed to enable notifications")
            disconnect()
            return
        }

        // Give a small delay for notifications to be fully set up
        Handler(Looper.getMainLooper()).postDelayed({
            // Send firmware version command
            vescCommands.getFwVersion(object : VescCommands.CommandCallback {
                override fun onResponse(command: Int, data: ByteArray) {
                    Log.d(TAG, "Firmware version response - Command: $command, Data: ${data.joinToString(" ") { "%02X".format(it) }}")
                    listener?.onFirmwareVersionReceived(command, data)
                }

                override fun onError(command: Int, message: String) {
                    Log.e(TAG, "Firmware version error - Command: $command, Error: $message")
                    listener?.onCommandError(command, message)
                }
            })
        }, 100) // 100ms delay

        // Log all available services and characteristics for debugging
        logAvailableCharacteristics(gatt)
    }

    /**
     * Disconnect from the current device
     */
    fun disconnect() {
        vescCommands?.cleanup()
        vescBle.disconnect()
        vescCommands = null
    }

    /**
     * Check if permissions are granted
     */
    fun hasPermissions(): Boolean {
        return vescBle.hasAllPermissions(activity)
    }

    /**
     * Request permissions if needed
     */
    fun requestPermissions(requestCode: Int = 1001): Boolean {
        return vescBle.requestPermissionsIfNeeded(activity, requestCode)
    }

    private fun logAvailableCharacteristics(gatt: BluetoothGatt) {
        Log.d(TAG, "=== Available Services and Characteristics ===")

        val services = gatt.services
        Log.d(TAG, "Total services found: ${services.size}")

        for (service in services) {
            Log.d(TAG, "Service UUID: ${service.uuid}")
            Log.d(TAG, "  Service type: ${service.type}")

            val characteristics = service.characteristics
            Log.d(TAG, "  Characteristics count: ${characteristics.size}")

            for (characteristic in characteristics) {
                Log.d(TAG, "    Characteristic UUID: ${characteristic.uuid}")
                Log.d(TAG, "    Properties: ${getPropertiesString(characteristic.properties)}")
                Log.d(TAG, "    Permissions: ${characteristic.permissions}")

                val descriptors = characteristic.descriptors
                if (descriptors.isNotEmpty()) {
                    Log.d(TAG, "    Descriptors count: ${descriptors.size}")
                    for (descriptor in descriptors) {
                        Log.d(TAG, "      Descriptor UUID: ${descriptor.uuid}")
                    }
                }
            }
        }
        Log.d(TAG, "=== End of Services and Characteristics ===")
    }

    private fun getPropertiesString(properties: Int): String {
        val props = mutableListOf<String>()
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_READ != 0)
            props.add("READ")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_WRITE != 0)
            props.add("WRITE")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0)
            props.add("WRITE_NO_RESPONSE")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_NOTIFY != 0)
            props.add("NOTIFY")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_INDICATE != 0)
            props.add("INDICATE")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_BROADCAST != 0)
            props.add("BROADCAST")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_EXTENDED_PROPS != 0)
            props.add("EXTENDED_PROPS")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_SIGNED_WRITE != 0)
            props.add("SIGNED_WRITE")
        return if (props.isEmpty()) "NONE" else props.joinToString(", ")
    }
}
