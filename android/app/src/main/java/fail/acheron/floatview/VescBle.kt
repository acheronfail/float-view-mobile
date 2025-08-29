package fail.acheron.floatview

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.*
import android.bluetooth.le.BluetoothLeScanner
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
import android.util.Log
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

class VescBle(private val context: Context) {

    companion object {
        val VescService = UUID.fromString("6e400001-b5a3-f393-e0a9-e50e24dcca9e")
        val VescCharacteristicsRx = UUID.fromString("6e400002-b5a3-f393-e0a9-e50e24dcca9e")
        val VescCharacteristicsTx = UUID.fromString("6e400003-b5a3-f393-e0a9-e50e24dcca9e")
        val RescueService = UUID.fromString("99eb1511-a9e9-4024-b0a4-3dc4b4fabfb0")
        val RescueCharacteristicConf = UUID.fromString("99eb1513-a9e9-4024-b0a4-3dc4b4fabfb0")
        val RescueCharacteristicHwVer = UUID.fromString("99eb1515-a9e9-4024-b0a4-3dc4b4fabfb0")
        val CharacteristicFw = UUID.fromString("99eb1514-a9e9-4024-b0a4-3dc4b4fabfb0")
    }

    interface ScanListener {
        fun onDeviceFound(device: BluetoothDevice) {}
        fun onScanStopped(devices: List<BluetoothDevice>) {}
        fun onError(message: String)
    }

    interface ConnectionListener {
        fun onConnected(gatt: BluetoothGatt)
        /** services discovered and contain target service(s) */
        fun onReady(gatt: BluetoothGatt)
        fun onDisconnected()
        fun onError(message: String)
    }

    private val logTag = "FloatView_BluetoothModule"
    private val bleManager =
            context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
    private val bluetoothAdapter = bleManager?.adapter
    private val foundDevices = ConcurrentHashMap<String, BluetoothDevice>()

    private var bluetoothGatt: BluetoothGatt? = null
    private var isScanning = false
    private val mainHandler = Handler(Looper.getMainLooper())

    // --- Permissions ---
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

    fun hasAllPermissions(activity: Activity): Boolean =
            requiredRuntimePermissions().all {
                ContextCompat.checkSelfPermission(activity, it) == PackageManager.PERMISSION_GRANTED
            }

    fun requestPermissionsIfNeeded(activity: Activity, requestCode: Int): Boolean {
        if (hasAllPermissions(activity)) return true
        val needed =
                requiredRuntimePermissions().filter {
                    ContextCompat.checkSelfPermission(activity, it) !=
                            PackageManager.PERMISSION_GRANTED
                }

        if (needed.isNotEmpty()) {
            ActivityCompat.requestPermissions(activity, needed.toTypedArray(), requestCode)
        }

        return false
    }

    // --- Scanning ---
    fun startScan(serviceUuids: List<UUID>, durationMs: Long = 10_000L, listener: ScanListener) {
        if (isScanning) {
            listener.onError("Scan already in progress")
            return
        }
        if (bluetoothAdapter == null) {
            listener.onError("Bluetooth not supported")
            return
        }
        if (!bluetoothAdapter.isEnabled) {
            listener.onError("Bluetooth disabled")
            return
        }
        if (!hasAllPermissions(context as? Activity ?: return)) {
            listener.onError("Permissions not granted")
            return
        }

        val scanner: BluetoothLeScanner =
                bluetoothAdapter.bluetoothLeScanner
                        ?: run {
                            listener.onError("BLE scanner unavailable")
                            return
                        }

        foundDevices.clear()

        val filters =
                if (serviceUuids.isNotEmpty()) {
                    serviceUuids.map { uuid ->
                        Log.d(logTag, "Adding service filter for UUID: $uuid")
                        ScanFilter.Builder().setServiceUuid(ParcelUuid(uuid)).build()
                    }
                } else {
                    Log.d(logTag, "No service filters - scanning for all devices")
                    emptyList()
                }

        val settings =
                ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()

        val callback =
                object : ScanCallback() {
                    @SuppressLint("MissingPermission")
                    override fun onScanResult(callbackType: Int, result: ScanResult) {
                        val device = result.device ?: return
                        if (device.address.isNullOrBlank()) return
                        if (foundDevices.putIfAbsent(device.address, device) == null) {
                            Log.d(
                                    logTag,
                                    "Found device ${device.address} (${device.name ?: "Unnamed"})"
                            )
                            listener.onDeviceFound(device)
                        }
                    }

                    override fun onBatchScanResults(results: MutableList<ScanResult>) {
                        results.forEach { onScanResult(ScanSettings.CALLBACK_TYPE_ALL_MATCHES, it) }
                    }

                    override fun onScanFailed(errorCode: Int) {
                        isScanning = false
                        listener.onError("Scan failed: $errorCode")
                    }
                }

        try {
            isScanning = true
            scanner.startScan(filters, settings, callback)
            Log.d(logTag, "Scan started")
        } catch (e: SecurityException) {
            isScanning = false
            listener.onError("SecurityException: ${e.message}")
            return
        }

        // Stop scan after duration
        mainHandler.postDelayed({ stopScanInternal(scanner, callback, listener) }, durationMs)
    }

    @SuppressLint("MissingPermission")
    private fun stopScanInternal(
            scanner: BluetoothLeScanner,
            callback: ScanCallback,
            listener: ScanListener
    ) {
        if (!isScanning) return
        try {
            scanner.stopScan(callback)
        } catch (_: Exception) {}
        isScanning = false
        listener.onScanStopped(foundDevices.values.toList())
        Log.d(logTag, "Scan stopped. Devices found: ${foundDevices.size}")
    }

    fun stopScan() {
        // This simplified module doesn't retain scanner/callback references externally.
        // For manual stop before timeout, we could restart with 0 duration; instead just set flag.
        isScanning = false
    }

    // --- Connection ---
    fun connect(
            address: String,
            requiredServices: List<UUID>,
            listener: ConnectionListener
    ): Boolean {
        if (bluetoothGatt != null) {
            listener.onError("Already connected or connecting")
            return false
        }

        val device = foundDevices[address]
        if (device == null) {
            listener.onError("Device $address not known!")
            return false
        }

        try {
            bluetoothGatt =
                    device.connectGatt(
                            context,
                            false,
                            object : BluetoothGattCallback() {
                                override fun onConnectionStateChange(
                                        gatt: BluetoothGatt,
                                        status: Int,
                                        newState: Int
                                ) {
                                    if (status != BluetoothGatt.GATT_SUCCESS) {
                                        Log.e(logTag, "Connection failed: status=$status")
                                        listener.onError("Connection failed: $status")
                                        closeGatt()
                                        return
                                    }
                                    if (newState == BluetoothProfile.STATE_CONNECTED) {
                                        Log.d(logTag, "Connected to GATT server")
                                        listener.onConnected(gatt)
                                        gatt.discoverServices()
                                    } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                                        Log.d(logTag, "Disconnected from GATT server")
                                        listener.onDisconnected()
                                        closeGatt()
                                    }
                                }

                                override fun onServicesDiscovered(
                                        gatt: BluetoothGatt,
                                        status: Int
                                ) {
                                    if (status != BluetoothGatt.GATT_SUCCESS) {
                                        listener.onError("Service discovery failed: $status")
                                        return
                                    }
                                    val gattServices = gatt.services ?: emptyList()
                                    val present =
                                            requiredServices.all { req ->
                                                gattServices.any { it.uuid == req }
                                            }
                                    if (!present) {
                                        listener.onError("Required service missing")
                                        gatt.disconnect()
                                        return
                                    }
                                    listener.onReady(gatt)
                                }
                            }
                    )
            return true
        } catch (se: SecurityException) {
            listener.onError("Missing permissions: ${se.message}")
            closeGatt()
            return false
        } catch (e: Exception) {
            listener.onError("Connect error: ${e.message}")
            closeGatt()
            return false
        }
    }

    @SuppressLint("MissingPermission")
    fun disconnect() {
        try {
            bluetoothGatt?.disconnect()
        } catch (_: Exception) {}
    }

    @SuppressLint("MissingPermission")
    private fun closeGatt() {
        try {
            bluetoothGatt?.close()
        } catch (_: Exception) {}
        bluetoothGatt = null
    }
}
