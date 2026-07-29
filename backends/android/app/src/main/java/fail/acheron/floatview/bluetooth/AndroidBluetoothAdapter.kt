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
import android.bluetooth.le.ScanFilter
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
import android.os.ParcelUuid
import android.util.Log
import fail.acheron.floatview.backend.BluetoothEvent
import fail.acheron.floatview.backend.BluetoothPermissionState
import fail.acheron.floatview.backend.BluetoothPort
import fail.acheron.floatview.backend.BluetoothRadioState
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.DeviceId
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.UUID

fun interface BluetoothPermissionRequester {
    fun request(permissions: Array<String>)

    fun wasResolved(permission: String): Boolean = false
}

class AndroidBluetoothAdapter(
    context: Context,
    private val permissionRequester: BluetoothPermissionRequester,
    private val eventHandler: (BluetoothEvent) -> Unit,
    private val diagnosticHandler: (String) -> Unit = {},
) : BluetoothPort {
    private val applicationContext = context.applicationContext
    private val manager = applicationContext.getSystemService(BluetoothManager::class.java)
    private val handler = Handler(Looper.getMainLooper())
    private val deviceIdentity = AndroidDeviceIdentity.from(applicationContext)
    private val opaqueIds = mutableMapOf<String, DeviceId>()
    private val devicesById = mutableMapOf<DeviceId, BluetoothDevice>()
    private var activeDeviceId: DeviceId? = null
    private var activeGatt: BluetoothGatt? = null
    private var writeCharacteristic: BluetoothGattCharacteristic? = null
    private var notifyCharacteristic: BluetoothGattCharacteristic? = null
    private val pendingWrites = ArrayDeque<ByteArray>()
    private var writeInFlight = false
    private var connectionTimeout: Runnable? = null
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
            val deviceId = opaqueIds.getOrPut(address) { deviceIdentity.idFor(address) }
            devicesById[deviceId] = result.device
            val name = result.scanRecord?.deviceName ?: try {
                result.device.name
            } catch (_: SecurityException) {
                null
            }
            eventHandler(BluetoothEvent.DeviceFound(deviceId, name, result.rssi))
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

    override fun requestPermission() {
        val policy = AndroidBluetoothPermissionPolicy.forApi(Build.VERSION.SDK_INT)
        val required = listOfNotNull(policy.scanPermission, policy.connectPermission)
            .distinct()
            .filter { applicationContext.checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
        if (required.isNotEmpty()) permissionRequester.request(required.toTypedArray())
        eventHandler(BluetoothEvent.StateChanged(currentState()))
    }

    override fun startDiscovery() {
        val scanner = manager.adapter?.bluetoothLeScanner
        if (scanner == null) {
            eventHandler(BluetoothEvent.ScanFailed("bluetooth_unavailable"))
            return
        }
        try {
            scanner.startScan(
                listOf(
                    ScanFilter.Builder()
                        .setServiceUuid(ParcelUuid(VescGattProfile.service))
                        .build(),
                ),
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

    override fun connect(deviceId: DeviceId) {
        diagnosticHandler("connect_requested")
        val device = synchronized(this) {
            if (activeGatt != null && activeDeviceId != deviceId) null else devicesById[deviceId]
        }
        if (device == null) {
            eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.FAILED))
            return
        }
        try {
            stopDiscovery()
            synchronized(this) {
                activeDeviceId = deviceId
                writeCharacteristic = null
                notifyCharacteristic = null
                pendingWrites.clear()
                writeInFlight = false
                activeGatt = device.connectGatt(
                    applicationContext,
                    false,
                    gattCallback,
                    BluetoothDevice.TRANSPORT_LE,
                )
                activeGatt?.let { scheduleConnectionTimeout(deviceId, it) }
            }
        } catch (_: SecurityException) {
            failConnection(deviceId, "permission_required")
        }
    }

    override fun write(deviceId: DeviceId, bytes: ByteArray) {
        val queueFull = synchronized(this) {
            if (
                deviceId != activeDeviceId ||
                activeGatt == null ||
                writeCharacteristic == null
            ) {
                return
            }
            if (pendingWrites.size >= MAX_PENDING_WRITES) {
                true
            } else {
                pendingWrites.addLast(bytes.copyOf())
                false
            }
        }
        if (queueFull) {
            failConnection(deviceId, "vesc_write_queue_full")
            return
        }
        drainWriteQueue(deviceId)
    }

    private fun drainWriteQueue(deviceId: DeviceId) {
        val pending = synchronized(this) {
            if (deviceId != activeDeviceId || writeInFlight) return
            val gatt = activeGatt ?: return
            val characteristic = writeCharacteristic ?: return
            val bytes = pendingWrites.removeFirstOrNull() ?: return
            writeInFlight = true
            Triple(gatt, characteristic, bytes)
        }
        val (gatt, characteristic, bytes) = pending
        try {
            characteristic.writeType =
                if (
                    characteristic.properties and
                    BluetoothGattCharacteristic.PROPERTY_WRITE != 0
                ) {
                    BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                } else {
                    BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
                }
            @Suppress("DEPRECATION")
            characteristic.value = bytes
            @Suppress("DEPRECATION")
            if (!gatt.writeCharacteristic(characteristic)) {
                synchronized(this) { writeInFlight = false }
                failConnection(deviceId, "vesc_write_start_failed")
            }
        } catch (_: SecurityException) {
            synchronized(this) { writeInFlight = false }
            failConnection(deviceId, "permission_required")
        }
    }

    override fun disconnect(deviceId: DeviceId) {
        cancelConnectionTimeout()
        val gatt = synchronized(this) {
            if (deviceId == activeDeviceId) activeGatt else null
        }
        if (gatt == null) {
            clearConnection()
            eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED))
            return
        }
        try {
            gatt.disconnect()
        } catch (_: SecurityException) {
            clearConnection()
            eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED))
        }
    }

    private val gattCallback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt) activeDeviceId else null
            } ?: return
            when {
                status == BluetoothGatt.GATT_SUCCESS && newState == BluetoothProfile.STATE_CONNECTED -> {
                    eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.CONNECTED))
                    eventHandler(
                        BluetoothEvent.ConnectionChanged(
                            deviceId,
                            ConnectionState.DISCOVERING_SERVICES,
                        ),
                    )
                    try {
                        if (!gatt.requestMtu(VESC_ATT_MTU)) {
                            failConnection(deviceId, "vesc_mtu_request_failed")
                        }
                    } catch (_: SecurityException) {
                        failConnection(deviceId, "permission_required")
                    }
                }
                newState == BluetoothProfile.STATE_DISCONNECTED -> {
                    cancelConnectionTimeout()
                    clearConnection(gatt)
                    eventHandler(
                        BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED),
                    )
                }
                status != BluetoothGatt.GATT_SUCCESS -> {
                    eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.FAILED))
                    try {
                        gatt.disconnect()
                    } catch (_: SecurityException) {
                        clearConnection(gatt)
                        eventHandler(
                            BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED),
                        )
                    }
                }
            }
        }

        override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt) activeDeviceId else null
            } ?: return
            if (status != BluetoothGatt.GATT_SUCCESS || mtu < MINIMUM_VESC_ATT_MTU) {
                failConnection(deviceId, "vesc_mtu_negotiation_failed")
                return
            }
            try {
                if (!gatt.discoverServices()) {
                    failConnection(deviceId, "vesc_service_discovery_failed")
                }
            } catch (_: SecurityException) {
                failConnection(deviceId, "permission_required")
            }
        }

        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt) activeDeviceId else null
            } ?: return
            val service = gatt.getService(VescGattProfile.service)
            val write = service?.getCharacteristic(VescGattProfile.write)
            val notify = service?.getCharacteristic(VescGattProfile.notify)
            if (status != BluetoothGatt.GATT_SUCCESS || write == null || notify == null) {
                failConnection(deviceId, "vesc_gatt_profile_unavailable")
                return
            }
            synchronized(this@AndroidBluetoothAdapter) {
                writeCharacteristic = write
                notifyCharacteristic = notify
            }
            eventHandler(
                BluetoothEvent.ConnectionChanged(
                    deviceId,
                    ConnectionState.ENABLING_NOTIFICATIONS,
                ),
            )
            try {
                val descriptor = notify.getDescriptor(CLIENT_CHARACTERISTIC_CONFIGURATION)
                if (
                    notify.properties and BluetoothGattCharacteristic.PROPERTY_NOTIFY == 0 ||
                    !gatt.setCharacteristicNotification(notify, true)
                ) {
                    failConnection(deviceId, "vesc_notification_enable_failed")
                    return
                }
                if (descriptor == null) {
                    // VESC Express firmware in the initial hardware profile
                    // advertises NOTIFY but omits the standard CCCD from its
                    // discovered table on Android 10. Keep local notification
                    // routing enabled; notification receipt remains the proof
                    // that this compatibility path is usable.
                    Log.w(LOG_TAG, "Notification CCCD absent; using VESC compatibility path")
                    cancelConnectionTimeout()
                    eventHandler(
                        BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.READY),
                    )
                    return
                }
                @Suppress("DEPRECATION")
                descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                @Suppress("DEPRECATION")
                if (!gatt.writeDescriptor(descriptor)) {
                    failConnection(deviceId, "vesc_cccd_write_failed")
                }
            } catch (_: SecurityException) {
                failConnection(deviceId, "permission_required")
            }
        }

        override fun onDescriptorWrite(
            gatt: BluetoothGatt,
            descriptor: BluetoothGattDescriptor,
            status: Int,
        ) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt) activeDeviceId else null
            } ?: return
            if (
                descriptor.uuid == CLIENT_CHARACTERISTIC_CONFIGURATION &&
                status == BluetoothGatt.GATT_SUCCESS
            ) {
                cancelConnectionTimeout()
                eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.READY))
            } else {
                failConnection(deviceId, "vesc_cccd_write_failed")
            }
        }

        override fun onCharacteristicWrite(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int,
        ) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt && characteristic === writeCharacteristic) {
                    writeInFlight = false
                    activeDeviceId
                } else {
                    null
                }
            } ?: return
            if (status == BluetoothGatt.GATT_SUCCESS) {
                drainWriteQueue(deviceId)
            } else {
                failConnection(deviceId, "vesc_write_failed_$status")
            }
        }

        @Deprecated("Deprecated in Android 13, retained for Android 10 compatibility")
        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
        ) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt && characteristic === notifyCharacteristic) {
                    activeDeviceId
                } else {
                    null
                }
            } ?: return
            @Suppress("DEPRECATION")
            eventHandler(BluetoothEvent.Notification(deviceId, characteristic.value.copyOf()))
        }
    }

    private fun failConnection(deviceId: DeviceId, reason: String) {
        cancelConnectionTimeout()
        Log.w(LOG_TAG, "Connection failed: $reason")
        eventHandler(BluetoothEvent.ScanFailed(reason))
        eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.FAILED))
        val gatt = synchronized(this) { activeGatt }
        if (gatt == null) {
            clearConnection()
            eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED))
        } else {
            try {
                gatt.disconnect()
            } catch (_: SecurityException) {
                clearConnection(gatt)
                eventHandler(
                    BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED),
                )
            }
        }
    }

    @Synchronized
    private fun clearConnection(expected: BluetoothGatt? = activeGatt) {
        val gatt = activeGatt
        if (expected != null && gatt !== expected) return
        activeGatt = null
        activeDeviceId = null
        writeCharacteristic = null
        notifyCharacteristic = null
        pendingWrites.clear()
        writeInFlight = false
        try {
            gatt?.close()
        } catch (_: SecurityException) {
            // Local references are still released when permission changes.
        }
    }

    private fun scheduleConnectionTimeout(deviceId: DeviceId, gatt: BluetoothGatt) {
        cancelConnectionTimeout()
        val timeout = Runnable {
            val timedOut = synchronized(this) {
                activeGatt === gatt && activeDeviceId == deviceId
            }
            if (!timedOut) return@Runnable
            diagnosticHandler("connect_timeout")
            eventHandler(BluetoothEvent.ScanFailed("connection_timeout"))
            eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.FAILED))
            clearConnection(gatt)
            eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED))
        }
        connectionTimeout = timeout
        handler.postDelayed(timeout, CONNECTION_ATTEMPT_TIMEOUT_MS)
    }

    private fun cancelConnectionTimeout() {
        val timeout = synchronized(this) {
            connectionTimeout.also { connectionTimeout = null }
        }
        timeout?.let(handler::removeCallbacks)
    }

    private fun permissionState(permission: String?): BluetoothPermissionState =
        AndroidBluetoothPermissionPolicy.state(
            permission = permission,
            granted = permission != null &&
                applicationContext.checkSelfPermission(permission) ==
                PackageManager.PERMISSION_GRANTED,
            requestResolved = permission?.let(permissionRequester::wasResolved) ?: false,
        )

    companion object {
        private const val LOG_TAG = "FloatViewBluetooth"
        private const val MAX_PENDING_WRITES = 32
        private const val CONNECTION_ATTEMPT_TIMEOUT_MS = 10_000L
        private const val VESC_ATT_MTU = 247
        private const val MINIMUM_VESC_ATT_MTU = 64
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
