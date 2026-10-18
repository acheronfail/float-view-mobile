package fail.acheron.floatview.bluetooth

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import fail.acheron.floatview.application.BluetoothConnection
import fail.acheron.floatview.application.BluetoothEvent
import fail.acheron.floatview.application.BluetoothPermissionState
import fail.acheron.floatview.application.BluetoothPort
import fail.acheron.floatview.application.BluetoothRadioState
import fail.acheron.floatview.application.BluetoothState
import fail.acheron.floatview.application.ConnectionState
import fail.acheron.floatview.application.DeviceId
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.UUID

fun interface BluetoothPermissionHistory {
    fun wasResolved(permission: String): Boolean
}

class AndroidBluetoothAdapter(
    context: Context,
    private val permissionHistory: BluetoothPermissionHistory,
    private val eventHandler: (BluetoothEvent) -> Unit,
    private val diagnosticHandler: (String) -> Unit = {},
) : BluetoothPort, AutoCloseable {
    private val applicationContext = context.applicationContext
    private val manager = applicationContext.getSystemService(BluetoothManager::class.java)
    private val handler = Handler(Looper.getMainLooper())
    private val deviceIdentity = AndroidDeviceIdentity.from(applicationContext)
    private val devicesById = mutableMapOf<DeviceId, BluetoothDevice>()
    private var connection: Connection? = null
    private val radioReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == BluetoothAdapter.ACTION_STATE_CHANGED) {
                eventHandler(BluetoothEvent.StateChanged(currentState()))
            }
        }
    }
    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            val address = try {
                result.device.address
            } catch (_: SecurityException) {
                eventHandler(BluetoothEvent.ScanFailed("permission_required"))
                return
            }
            val deviceId = deviceIdentity.idFor(address)
            val advertisesUart = result.scanRecord?.serviceUuids
                ?.any { it.uuid == VescGattProfile.service } == true
            synchronized(this@AndroidBluetoothAdapter) {
                if (devicesById.size >= 50 && deviceId !in devicesById) devicesById.remove(devicesById.keys.first())
                devicesById[deviceId] = result.device
            }
            val name = result.scanRecord?.deviceName ?: try {
                result.device.name
            } catch (_: SecurityException) {
                null
            }
            eventHandler(BluetoothEvent.DeviceFound(deviceId, name, result.rssi, advertisesUart))
        }

        override fun onScanFailed(errorCode: Int) {
            eventHandler(BluetoothEvent.ScanFailed("android_scan_error_$errorCode"))
        }
    }

    init {
        val filter = IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED)
        if (Build.VERSION.SDK_INT >= 33) {
            applicationContext.registerReceiver(
                radioReceiver,
                filter,
                Context.RECEIVER_NOT_EXPORTED,
            )
        } else {
            @Suppress("DEPRECATION")
            applicationContext.registerReceiver(radioReceiver, filter)
        }
    }

    override fun close() {
        applicationContext.unregisterReceiver(radioReceiver)
        stopDiscovery()
        synchronized(this) { connection?.close(); connection = null }
    }

    override fun currentState(): BluetoothState {
        val policy = AndroidBluetoothPermissionPolicy.forApi(Build.VERSION.SDK_INT)
        val scanPermission = permissionState(policy.scanPermission)
        val connectPermission = permissionState(policy.connectPermission)
        val radio = when {
            manager.adapter == null -> BluetoothRadioState.UNSUPPORTED
            Build.VERSION.SDK_INT >= 31 &&
                connectPermission != BluetoothPermissionState.GRANTED -> BluetoothRadioState.UNKNOWN
            else -> try {
                if (manager.adapter.isEnabled) {
                    BluetoothRadioState.POWERED_ON
                } else {
                    BluetoothRadioState.POWERED_OFF
                }
            } catch (_: SecurityException) {
                BluetoothRadioState.UNKNOWN
            }
        }
        return BluetoothState(radio, scanPermission, connectPermission)
    }

    override fun startDiscovery() {
        val scanner = manager.adapter?.bluetoothLeScanner
        if (scanner == null) {
            eventHandler(BluetoothEvent.ScanFailed("bluetooth_unavailable"))
            return
        }
        try {
            scanner.startScan(
                null,
                ScanSettings.Builder()
                    .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                    .build(),
                scanCallback,
            )
        } catch (_: SecurityException) {
            eventHandler(BluetoothEvent.ScanFailed("permission_required"))
        }
    }

    override fun stopDiscovery() {
        try {
            manager.adapter?.bluetoothLeScanner?.stopScan(scanCallback)
        } catch (_: SecurityException) {
            eventHandler(BluetoothEvent.ScanFailed("permission_required"))
        }
    }

    override fun connect(deviceId: DeviceId, generation: BluetoothConnection) {
        handler.post {
            connection?.close()
            val device = synchronized(this) { devicesById[deviceId] }
            if (device == null) {
                eventHandler(BluetoothEvent.ConnectionChanged(deviceId, generation, ConnectionState.FAILED))
            } else {
                stopDiscovery()
                connection = Connection(deviceId, generation).also { it.start(device) }
            }
        }
    }
    override fun disconnect(deviceId: DeviceId, generation: BluetoothConnection) {
        handler.post {
            if (connection?.generation == generation) { connection?.close(); connection = null }
        }
    }
    override fun enableNotifications(generation: BluetoothConnection, operation: Long) {
        handler.post { connection?.takeIf { it.generation == generation }?.enable(operation) }
    }
    override fun write(deviceId: DeviceId, generation: BluetoothConnection, operation: Long, response: Boolean, bytes: ByteArray) {
        handler.post { connection?.takeIf { it.generation == generation }?.write(operation, response, bytes) }
    }

    /** One OS operation; callbacks retain this attempt's identity, never the current connection's. */
    private inner class Connection(val device: DeviceId, val generation: BluetoothConnection) : BluetoothGattCallback() {
        private var gatt: BluetoothGatt? = null
        private var tx: BluetoothGattCharacteristic? = null
        private var rx: BluetoothGattCharacteristic? = null
        private var setup: Long? = null
        private var current: Long? = null
        private var response = false
        private var reportedWrite = false
        private var closed = false
        private val pacing = Runnable {
            val op = current
            if (!closed && !response && op != null) { current = null; progress(op, "available") }
        }
        private fun link(state: ConnectionState) = eventHandler(BluetoothEvent.ConnectionChanged(device, generation, state))
        private fun progress(op: Long, result: String) = eventHandler(BluetoothEvent.WriteProgress(generation, op, result))
        private fun fail() { if (!closed) { link(ConnectionState.FAILED); close() } }
        fun start(device: BluetoothDevice) {
            try {
                gatt = device.connectGatt(applicationContext, false, this, BluetoothDevice.TRANSPORT_LE, BluetoothDevice.PHY_LE_1M_MASK, handler)
                if (gatt == null) fail()
            } catch (_: SecurityException) { fail() }
        }
        fun close() {
            closed = true; current = null; handler.removeCallbacks(pacing)
            try { gatt?.disconnect(); gatt?.close() } catch (_: SecurityException) { }
            gatt = null; tx = null; rx = null
        }
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            if (closed || gatt !== this.gatt) return
            if (status != BluetoothGatt.GATT_SUCCESS) { fail(); return }
            when (newState) {
                BluetoothProfile.STATE_CONNECTED -> {
                    link(ConnectionState.DISCOVERING_SERVICES)
                    // Keep ATT MTU 23: larger MTUs have corrupted VESC Express notifications on Samsung.
                    try { if (!gatt.discoverServices()) fail() } catch (_: SecurityException) { fail() }
                }
                BluetoothProfile.STATE_DISCONNECTED -> { link(ConnectionState.DISCONNECTED); close() }
            }
        }
        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            if (closed || gatt !== this.gatt) return
            val service = gatt.getService(VescGattProfile.service)
            tx = service?.getCharacteristic(VescGattProfile.write)
            rx = service?.getCharacteristic(VescGattProfile.notify)
            val properties = tx?.properties ?: 0
            eventHandler(BluetoothEvent.Profile(generation, fail.acheron.floatview.application.BluetoothProfile(
                status == BluetoothGatt.GATT_SUCCESS && service != null,
                ((rx?.properties ?: 0) and BluetoothGattCharacteristic.PROPERTY_NOTIFY) != 0,
                if (properties and BluetoothGattCharacteristic.PROPERTY_WRITE != 0) 20 else 0,
                if (properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0) 20 else 0,
            )))
        }
        fun enable(operation: Long) {
            if (closed || setup != null) return
            setup = operation
            val gatt = gatt ?: return
            val notify = rx ?: return
            try {
                if (!gatt.setCharacteristicNotification(notify, true)) { notifications(false); return }
                val descriptor = notify.getDescriptor(CLIENT_CHARACTERISTIC_CONFIGURATION)
                if (descriptor == null) {
                    // Supported VESC Express tables omit CCCD; local notification routing must still succeed.
                    notifications(true); return
                }
                @Suppress("DEPRECATION")
                descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                @Suppress("DEPRECATION")
                if (!gatt.writeDescriptor(descriptor)) notifications(false)
            } catch (_: SecurityException) { notifications(false) }
        }
        private fun notifications(success: Boolean) {
            setup?.let { eventHandler(BluetoothEvent.Notifications(generation, it, success)) }
        }
        override fun onDescriptorWrite(gatt: BluetoothGatt, descriptor: BluetoothGattDescriptor, status: Int) {
            if (!closed && gatt === this.gatt && descriptor === rx?.getDescriptor(CLIENT_CHARACTERISTIC_CONFIGURATION)) {
                notifications(status == BluetoothGatt.GATT_SUCCESS)
            }
        }
        fun write(operation: Long, response: Boolean, bytes: ByteArray) {
            if (closed || current != null || bytes.size > 20) { fail(); return }
            val gatt = gatt ?: return
            val characteristic = tx ?: return
            current = operation; this.response = response
            if (!reportedWrite) {
                diagnosticHandler("gatt_write mode=${if (response) "with_response" else "without_response"} ceiling=20 payload=${bytes.size}")
                reportedWrite = true
            }
            characteristic.writeType = if (response) BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT else BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
            try {
                @Suppress("DEPRECATION")
                characteristic.value = bytes
                @Suppress("DEPRECATION")
                val accepted = gatt.writeCharacteristic(characteristic)
                progress(operation, if (accepted) "submitted" else "failed")
                if (accepted && !response) handler.postDelayed(pacing, 20)
            } catch (_: SecurityException) { progress(operation, "failed") }
        }
        override fun onCharacteristicWrite(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, status: Int) {
            if (closed || gatt !== this.gatt || characteristic !== tx || !response) return
            current?.let { current = null; progress(it, if (status == BluetoothGatt.GATT_SUCCESS) "acknowledged" else "failed") }
        }
        override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, value: ByteArray) {
            if (value.size > 8192) return
            val bytes = value.copyOf()
            if (!closed && gatt === this.gatt && characteristic === rx) eventHandler(BluetoothEvent.Notification(device, generation, bytes))
        }
        @Deprecated("Retained for Android 10")
        override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic) {
            @Suppress("DEPRECATION")
            onCharacteristicChanged(gatt, characteristic, characteristic.value ?: byteArrayOf())
        }
    }

    private fun permissionState(permission: String?): BluetoothPermissionState =
        AndroidBluetoothPermissionPolicy.state(
            permission = permission,
            granted = permission != null &&
                applicationContext.checkSelfPermission(permission) ==
                PackageManager.PERMISSION_GRANTED,
            requestResolved = permission?.let(permissionHistory::wasResolved) ?: false,
        )

    companion object {
        private val CLIENT_CHARACTERISTIC_CONFIGURATION =
            UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
    }
}

internal class AndroidDeviceIdentity(private val installationSalt: String) {
    fun idFor(platformAddress: String): DeviceId {
        val digest = MessageDigest.getInstance("SHA-256").digest(
            "$installationSalt\u0000${platformAddress.uppercase()}"
                .toByteArray(StandardCharsets.UTF_8),
        )
        return DeviceId("android-" + digest.take(16).joinToString("") { "%02x".format(it) })
    }

    companion object {
        private const val PREFERENCES = "float-view-device-identity"
        private const val INSTALLATION_SALT = "installation-salt"

        fun from(context: Context): AndroidDeviceIdentity {
            val preferences = context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)
            val salt = preferences.getString(INSTALLATION_SALT, null)
                ?: UUID.randomUUID().toString().also {
                    preferences.edit().putString(INSTALLATION_SALT, it).apply()
                }
            return AndroidDeviceIdentity(salt)
        }
    }
}
