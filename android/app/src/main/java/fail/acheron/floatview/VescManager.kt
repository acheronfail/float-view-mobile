package fail.acheron.floatview

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothManager
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

class VescManager(private val activity: Activity) {
    companion object {
        val BleClientCharacteristicConfig = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

        val VescService = UUID.fromString("6e400001-b5a3-f393-e0a9-e50e24dcca9e")
        val VescCharacteristicsRx = UUID.fromString("6e400002-b5a3-f393-e0a9-e50e24dcca9e")
        val VescCharacteristicsTx = UUID.fromString("6e400003-b5a3-f393-e0a9-e50e24dcca9e")
        val RescueService = UUID.fromString("99eb1511-a9e9-4024-b0a4-3dc4b4fabfb0")
        val RescueCharacteristicConf = UUID.fromString("99eb1513-a9e9-4024-b0a4-3dc4b4fabfb0")
        val RescueCharacteristicHwVer = UUID.fromString("99eb1515-a9e9-4024-b0a4-3dc4b4fabfb0")
        val CharacteristicFw = UUID.fromString("99eb1514-a9e9-4024-b0a4-3dc4b4fabfb0")
    }

    private val bleManager =
            activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
    private val bluetoothAdapter = bleManager?.adapter
    private val foundDevices = ConcurrentHashMap<String, BluetoothDevice>()
    private val mainHandler = Handler(Looper.getMainLooper())
    private var isScanning = false
    private var vescBluetoothLayer: VescBluetoothLayer? = null
    private val vescCommandLayer = VescCommandLayer()

    /*
     * Permissions
     */

    private fun requiredRuntimePermissions(): Array<String> {
        val perms = mutableListOf<String>()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            perms += Manifest.permission.BLUETOOTH
            perms += Manifest.permission.BLUETOOTH_SCAN
            perms += Manifest.permission.BLUETOOTH_CONNECT
        }

        // Location permission is required for BLE scanning
        perms += Manifest.permission.ACCESS_COARSE_LOCATION
        perms += Manifest.permission.ACCESS_FINE_LOCATION
        return perms.toTypedArray()
    }

    fun hasAllPermissions(): Boolean =
            requiredRuntimePermissions().all {
                ContextCompat.checkSelfPermission(activity, it) == PackageManager.PERMISSION_GRANTED
            }

    fun requestPermissionsIfNeeded(): Boolean {
        if (hasAllPermissions()) return true
        val needed =
                requiredRuntimePermissions().filter {
                    ContextCompat.checkSelfPermission(activity, it) !=
                            PackageManager.PERMISSION_GRANTED
                }

        if (needed.isNotEmpty()) {
            ActivityCompat.requestPermissions(activity, needed.toTypedArray(), 1729)
        }

        return false
    }


    /*
     * Scanning
     */

    fun startScan(timeoutMs: Long = 3_000L) {
        if (!requestPermissionsIfNeeded()) {
            return LocalServer.broadcastMessage(
                    wsMessage(
                            "scan",
                            "error",
                            buildJsonObject { put("message", "Permissions required") }
                    )
            )
        }

        if (isScanning) {
            return LocalServer.broadcastMessage(
                    wsMessage(
                            "scan",
                            "error",
                            buildJsonObject { put("message", "scan in progress") }
                    )
            )
        }

        if (bluetoothAdapter == null) {
            return LocalServer.broadcastMessage(
                    wsMessage(
                            "scan",
                            "error",
                            buildJsonObject { put("message", "Bluetooth not supported") }
                    )
            )
        }

        if (!bluetoothAdapter.isEnabled) {
            return LocalServer.broadcastMessage(
                    wsMessage(
                            "scan",
                            "error",
                            buildJsonObject { put("message", "Bluetooth disabled") }
                    )
            )
        }

        LocalServer.broadcastMessage(
                wsMessage(
                        "scan",
                        "status",
                        buildJsonObject { put("status", "start") }
                )
        )

        isScanning = true
        foundDevices.clear()

        val filters =
                listOf(
                        ScanFilter.Builder()
                                .setServiceUuid(ParcelUuid(VescManager.VescService))
                                .build()
                )

        val scanSettings =
                ScanSettings.Builder()
                        .setCallbackType(ScanSettings.CALLBACK_TYPE_ALL_MATCHES)
                        .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                        .build()

        try {
            bluetoothAdapter.bluetoothLeScanner.startScan(filters, scanSettings, this.scanCallback)
            mainHandler.postDelayed({ stopScan() }, timeoutMs)
        } catch (error: SecurityException) {
            isScanning = false
            return LocalServer.broadcastMessage(
                    wsMessage(
                            "scan",
                            "error",
                            buildJsonObject {
                                put("message", "Permission denied: ${error.message}")
                            }
                    )
            )
        }
    }

    fun stopScan() {
        if (!isScanning) return

        try {
            bluetoothAdapter?.bluetoothLeScanner?.stopScan(this.scanCallback)
        } catch (_: SecurityException) {}

        isScanning = false
        LocalServer.broadcastMessage(
                wsMessage("scan", "status", buildJsonObject { put("status", "complete") })
        )
    }

    private var connectedDevice: BluetoothGatt? = null

    /*
     * Connection
     */

    fun connect(address: String) {
        if (connectedDevice != null) {
            return LocalServer.broadcastMessage(
                    wsMessage(
                            "connect",
                            "error",
                            buildJsonObject { put("message", "already connected to a device") }
                    )
            )
        }

        val device = foundDevices[address]
        if (device == null) {
            return LocalServer.broadcastMessage(
                    wsMessage(
                            "connect",
                            "error",
                            buildJsonObject { put("message", "no device found") }
                    )
            )
        }

        LocalServer.broadcastMessage(
                wsMessage("connect", "status", buildJsonObject { put("status", "connecting") })
        )

        try {
            // TODO: set a timeout here and bail if we can't connect after a certain amount of time
            connectedDevice = device.connectGatt(activity, false, this.bluetoothGattCallback)
        } catch (error: SecurityException) {
            LocalServer.broadcastMessage(
                wsMessage("connect", "error", buildJsonObject { put("message", "Permission denied: ${error.message}") })
            )
        }
    }

    fun disconnect() {
        try {
            connectedDevice?.disconnect()
        } catch (error: SecurityException) {}
        connectedDevice = null
        isScanning = false
    }

    /*
     * Commands
     */

    fun sendCommand(command: Int, payload: List<Int>) {
        if (vescBluetoothLayer == null) {
            return LocalServer.broadcastMessage(
                wsMessage(
                    "command",
                    "error",
                    buildJsonObject { put("message", "VescBluetoothLayer not ready!") }
                )
            )
        }

        vescBluetoothLayer!!.send(command, payload, object : VescBluetoothLayer.CommandCallback {
            override fun onResponse(command: Int, data: ByteArray) {
                val result = vescCommandLayer.parse(command, data)
                if (result == null) {
                    return LocalServer.broadcastMessage(
                        wsMessage(
                            "command",
                            "error",
                            buildJsonObject {
                                put("command", command)
                                put("message", "Failed to parse command response")
                            }
                        )
                    )
                }

                return LocalServer.broadcastMessage(
                    wsMessage(
                        "command",
                        "result",
                        buildJsonObject {
                            put("command", command)
                            put("data", result)
                        }
                    )
                )
            }

            override fun onError(command: Int, message: String) {
                return LocalServer.broadcastMessage(
                    wsMessage(
                        "command",
                        "error",
                        buildJsonObject {
                            put("command", command)
                            put("message", message)
                        }
                    )
                )
            }
        })
    }

    /*
     * ScanCallback
     */

    private val scanCallback =
            object : ScanCallback() {
                override fun onScanResult(callbackType: Int, result: ScanResult?) {
                    if (callbackType == ScanSettings.CALLBACK_TYPE_ALL_MATCHES && result != null) {
                        if (foundDevices.putIfAbsent(result.device.address, result.device) == null) {
                            try {
                                LocalServer.broadcastMessage(
                                        wsMessage(
                                                "scan",
                                                "device",
                                                buildJsonObject {
                                                    put("address", result.device.address)
                                                    put("name", result.device.name)
                                                }
                                        )
                                )
                            } catch (error: SecurityException) {
                                LocalServer.broadcastMessage(
                                        wsMessage(
                                                "scan",
                                                "error",
                                                buildJsonObject {
                                                    put(
                                                            "message",
                                                            "Permission denied: ${error.message}"
                                                    )
                                                }
                                        )
                                )
                            }
                        }
                    }
                }

                override fun onScanFailed(errorCode: Int) {
                    LocalServer.broadcastMessage(
                            wsMessage(
                                    "scan",
                                    "error",
                                    buildJsonObject { put("message", "Scan failed: $errorCode") }
                            )
                    )
                }
            }

    /*
     * BluetoothGattCallback
     */

    private val bluetoothGattCallback = object : BluetoothGattCallback() {
        /*
         * Once we are connected to the device, find its services
         */
        override fun onConnectionStateChange(gatt: BluetoothGatt?, status: Int, newState: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                return LocalServer.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "Failed to connect: $status") }
                        )
                )
            }

            when (newState) {
                BluetoothGatt.STATE_CONNECTED -> {
                    LocalServer.broadcastMessage(
                            wsMessage(
                                    "connect",
                                    "status",
                                    buildJsonObject { put("status", "connected") }
                            )
                    )

                    try {
                        connectedDevice?.requestMtu(250)
                        connectedDevice?.discoverServices()
                    } catch (error: SecurityException) {
                        disconnect()
                        LocalServer.broadcastMessage(
                            wsMessage(
                        "connect",
                        "error",
                        buildJsonObject { put("message", "Permission denied: ${error.message}") }
                            )
                        )
                    }
                }
                BluetoothGatt.STATE_DISCONNECTED -> {
                    connectedDevice = null
                    stopScan()
                }
            }
        }

        /*
         * When services are discovered, next find the characteristics
         */
        @SuppressLint("NewApi")
        override fun onServicesDiscovered(gatt: BluetoothGatt?, status: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                return LocalServer.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "Failed to discover services: $status") }
                        )
                )
            }

            // setup notifications on the VESC transmission line, so we can get responses to commands
            val btDevice = connectedDevice
            if (btDevice == null) {
                return LocalServer.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "Device was null when discovering services") }
                        )
                )
            }

            val vescTx = btDevice.getService(VescService)?.getCharacteristic(VescCharacteristicsTx)
            if (vescTx == null) {
                return LocalServer.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "Failed to find VESC tx characteristic") }
                        )
                )
            }


            try {
                // TODO:  increase in iOS as well if needed, properly parse chunks as well

                if (!btDevice.setCharacteristicNotification(vescTx, true)) {
                    return LocalServer.broadcastMessage(
                            wsMessage(
                                    "connect",
                                    "error",
                                    buildJsonObject { put("message", "Failed to setup notifications on VESC tx characteristic") }
                            )
                    )
                }

                print("Successfully enabled notifications on TX characteristic: ${vescTx.uuid}")
            } catch (error: SecurityException) {
                return LocalServer.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "Permission denied: ${error.message}") }
                        )
                )
            }

            // unlike iOS, android doesn't have an event for when notifications are enabled on a
            // characteristic, so we just continue on here
            val vescRx = btDevice.getService(VescService)?.getCharacteristic(VescCharacteristicsRx)
            if (vescRx == null) {
                return LocalServer.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "Failed to find VESC rx characteristic") }
                        )
                )
            }

            // Use the connected device's gatt instead of the parameter
            vescBluetoothLayer = VescBluetoothLayer(btDevice, vescRx)
            LocalServer.broadcastMessage(
                    wsMessage(
                            "connect",
                            "status",
                            buildJsonObject { put("status", "ready") }
                    )
            )
        }

        /*
         * We receive VESC responses here since we requested notifications on the characteristic
         */
        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray
        ) {
            if (vescBluetoothLayer == null) {
                return LocalServer.broadcastMessage(
                        wsMessage(
                                "command",
                                "error",
                                buildJsonObject { put("message", "bad state: vesc bluetooth manager is null") }
                        )
                )
            }

            vescBluetoothLayer!!.parseResponse(value)
        }
    }
}
