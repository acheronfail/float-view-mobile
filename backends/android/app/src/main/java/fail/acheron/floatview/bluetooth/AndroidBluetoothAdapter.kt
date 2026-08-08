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
import fail.acheron.floatview.backend.BluetoothEvent
import fail.acheron.floatview.backend.BluetoothPermissionState
import fail.acheron.floatview.backend.BluetoothPort
import fail.acheron.floatview.backend.BluetoothRadioState
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.DeviceId
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
    private val devicesById = mutableMapOf<DeviceId, BluetoothDevice>()
    private var activeDeviceId: DeviceId? = null
    private var activeGatt: BluetoothGatt? = null
    private var writeCharacteristic: BluetoothGattCharacteristic? = null
    private var notifyCharacteristic: BluetoothGattCharacteristic? = null
    private val pendingWrites = ArrayDeque<ByteArray>()
    private var pendingWriteBytes = 0
    private var writeInFlight = false
    private var safeWritePayloadBytes = DEFAULT_WRITE_PAYLOAD_BYTES
    private var serviceDiscoveryStarted = false
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
            val advertisesUart = result.scanRecord?.serviceUuids
                ?.any { it.uuid == VescGattProfile.service } == true
            if (!advertisesUart) return
            val deviceId = DeviceId(address.uppercase())
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

    override fun connect(deviceId: DeviceId) {
        diagnosticHandler("connect_requested")
        val device = synchronized(this) {
            if (activeGatt != null && activeDeviceId != deviceId) {
                null
            } else {
                devicesById[deviceId] ?: runCatching {
                    manager.adapter?.getRemoteDevice(deviceId.value)
                }.getOrNull()
            }
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
                pendingWriteBytes = 0
                writeInFlight = false
                safeWritePayloadBytes = DEFAULT_WRITE_PAYLOAD_BYTES
                serviceDiscoveryStarted = false
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
            val chunks = VescBluetoothPolicy.chunks(bytes, safeWritePayloadBytes)
            if (
                !VescBluetoothPolicy.acceptsQueuedWrite(
                    pendingWrites.size,
                    pendingWriteBytes,
                    chunks.size,
                    bytes.size,
                )
            ) {
                true
            } else {
                chunks.forEach(pendingWrites::addLast)
                pendingWriteBytes += bytes.size
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
            pendingWriteBytes -= bytes.size
            writeInFlight = true
            Triple(gatt, characteristic, bytes)
        }
        val (gatt, characteristic, bytes) = pending
        try {
            val writeMode = VescBluetoothPolicy.writeMode(
                supportsWriteWithResponse =
                    characteristic.properties and
                        BluetoothGattCharacteristic.PROPERTY_WRITE != 0,
                supportsWriteWithoutResponse =
                    characteristic.properties and
                        BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0,
            )
            val completionRequired = VescBluetoothPolicy.writeCompletionRequired(writeMode)
            characteristic.writeType = if (completionRequired) {
                BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
            } else {
                BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
            }
            // Keep the queue occupied for both write modes. A write without
            // response has no callback we can use for flow control, so its
            // delayed continuation below provides the required pacing.
            synchronized(this) { writeInFlight = true }
            @Suppress("DEPRECATION")
            characteristic.value = bytes
            @Suppress("DEPRECATION")
            if (!gatt.writeCharacteristic(characteristic)) {
                synchronized(this) { writeInFlight = false }
                failConnection(deviceId, "vesc_write_start_failed")
            } else if (!completionRequired) {
                // Android does not deliver a completion callback for a
                // write-without-response operation. Advance after a small
                // bounded delay so consecutive commands do not overrun the
                // platform GATT queue.
                handler.postDelayed({
                    synchronized(this) {
                        if (deviceId == activeDeviceId) writeInFlight = false
                    }
                    drainWriteQueue(deviceId)
                }, NO_RESPONSE_WRITE_DELAY_MS)
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
                    diagnosticHandler("connected")
                    eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.CONNECTED))
                    eventHandler(
                        BluetoothEvent.ConnectionChanged(
                            deviceId,
                            ConnectionState.DISCOVERING_SERVICES,
                        ),
                    )
                    try {
                        // VESC Express' Nordic UART forwarding path drops a
                        // three-byte slice of long notifications on some
                        // Android/Samsung combinations after an MTU 247
                        // exchange. Keep the default ATT payload (20 bytes)
                        // so long VESC frames arrive intact and are reassembled
                        // by VescFrameStreamDecoder.
                        diagnosticHandler("gatt_mtu=default_23")
                        discoverServices(deviceId, gatt)
                    } catch (_: SecurityException) {
                        failConnection(deviceId, "permission_required")
                    }
                }
                status != BluetoothGatt.GATT_SUCCESS -> {
                    cancelConnectionTimeout()
                    diagnosticHandler("connect_failed_$status")
                    eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.FAILED))
                    if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                        clearConnection(gatt)
                        eventHandler(
                            BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED),
                        )
                        return
                    }
                    try {
                        gatt.disconnect()
                    } catch (_: SecurityException) {
                        clearConnection(gatt)
                        eventHandler(
                            BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED),
                        )
                    }
                }
                newState == BluetoothProfile.STATE_DISCONNECTED -> {
                    cancelConnectionTimeout()
                    clearConnection(gatt)
                    eventHandler(
                        BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.DISCONNECTED),
                    )
                }
            }
        }

        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt) activeDeviceId else null
            } ?: return
            val service = gatt.getService(VescGattProfile.service)
            val write = service?.getCharacteristic(VescGattProfile.write)
            val notify = service?.getCharacteristic(VescGattProfile.notify)
            val writeMode = VescBluetoothPolicy.writeMode(
                supportsWriteWithResponse =
                    (write?.properties?.and(BluetoothGattCharacteristic.PROPERTY_WRITE) ?: 0) != 0,
                supportsWriteWithoutResponse =
                    (
                        write?.properties?.and(
                            BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE,
                        ) ?: 0
                    ) != 0,
            )
            val setupFailure = VescBluetoothPolicy.setupFailure(
                hasService = status == BluetoothGatt.GATT_SUCCESS && service != null,
                hasWriteCharacteristic = write != null,
                writeMode = writeMode,
                hasNotifyCharacteristic = notify != null,
                hasNotifyProperty =
                    (notify?.properties?.and(BluetoothGattCharacteristic.PROPERTY_NOTIFY) ?: 0) != 0,
            )
            if (setupFailure != null) {
                failConnection(deviceId, setupFailure)
                return
            }
            diagnosticHandler(
                "gatt_profile writeMode=${writeMode.name.lowercase()} " +
                    "notify=${(notify?.properties?.and(BluetoothGattCharacteristic.PROPERTY_NOTIFY) ?: 0) != 0}",
            )
            val validatedWrite = requireNotNull(write)
            val validatedNotify = requireNotNull(notify)
            synchronized(this@AndroidBluetoothAdapter) {
                writeCharacteristic = validatedWrite
                notifyCharacteristic = validatedNotify
            }
            eventHandler(
                BluetoothEvent.ConnectionChanged(
                    deviceId,
                    ConnectionState.ENABLING_NOTIFICATIONS,
                ),
            )
            try {
                val descriptor =
                    validatedNotify.getDescriptor(CLIENT_CHARACTERISTIC_CONFIGURATION)
                val hasNotify = true
                val locallyEnabled =
                    hasNotify && gatt.setCharacteristicNotification(validatedNotify, true)
                when (
                    VescBluetoothPolicy.notificationSetup(
                        hasNotify,
                        locallyEnabled,
                        descriptor != null,
                    )
                ) {
                    VescBluetoothPolicy.NotificationSetup.REJECT -> {
                        failConnection(deviceId, "vesc_notification_enable_failed")
                        return
                    }
                    VescBluetoothPolicy.NotificationSetup.COMPATIBILITY_READY -> {
                        // Some VESC Express Android GATT tables expose the complete
                        // Nordic UART profile and NOTIFY capability but omit only
                        // the standard CCCD. Keep local routing enabled in this
                        // one compatibility case.
                        diagnosticHandler("notifications_enabled compatibility=no_cccd")
                        cancelConnectionTimeout()
                        eventHandler(
                            BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.READY),
                        )
                        return
                    }
                    VescBluetoothPolicy.NotificationSetup.WRITE_DESCRIPTOR -> Unit
                }
                @Suppress("DEPRECATION")
                requireNotNull(descriptor).value =
                    BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                @Suppress("DEPRECATION")
                if (!gatt.writeDescriptor(requireNotNull(descriptor))) {
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
                diagnosticHandler("notifications_enabled descriptor_status=$status")
                cancelConnectionTimeout()
                eventHandler(BluetoothEvent.ConnectionChanged(deviceId, ConnectionState.READY))
            } else {
                diagnosticHandler("notifications_failed descriptor_status=$status")
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

        private fun handleCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray,
        ) {
            val deviceId = synchronized(this@AndroidBluetoothAdapter) {
                if (gatt === activeGatt && characteristic === notifyCharacteristic) {
                    activeDeviceId
                } else {
                    null
                }
            } ?: return
            eventHandler(BluetoothEvent.Notification(deviceId, value.copyOf()))
        }

        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray,
        ) {
            handleCharacteristicChanged(gatt, characteristic, value)
        }

        @Deprecated("Deprecated in Android 13, retained for Android 10 compatibility")
        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
        ) {
            @Suppress("DEPRECATION")
            handleCharacteristicChanged(gatt, characteristic, characteristic.value.copyOf())
        }
    }

    private fun failConnection(deviceId: DeviceId, reason: String) {
        cancelConnectionTimeout()
        diagnosticHandler("connection_failed reason=$reason")
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
        pendingWriteBytes = 0
        writeInFlight = false
        safeWritePayloadBytes = DEFAULT_WRITE_PAYLOAD_BYTES
        serviceDiscoveryStarted = false
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

    private fun discoverServices(deviceId: DeviceId, gatt: BluetoothGatt) {
        val shouldStart = synchronized(this) {
            if (gatt !== activeGatt || serviceDiscoveryStarted) {
                false
            } else {
                serviceDiscoveryStarted = true
                true
            }
        }
        if (!shouldStart) return
        try {
            if (!gatt.discoverServices()) {
                failConnection(deviceId, "vesc_service_discovery_failed")
            }
        } catch (_: SecurityException) {
            failConnection(deviceId, "permission_required")
        }
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
        private const val CONNECTION_ATTEMPT_TIMEOUT_MS = 10_000L
        private const val NO_RESPONSE_WRITE_DELAY_MS = 20L
        private const val DEFAULT_WRITE_PAYLOAD_BYTES = 20
        private val CLIENT_CHARACTERISTIC_CONFIGURATION =
            UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
    }
}
