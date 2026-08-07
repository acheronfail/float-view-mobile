package fail.acheron.floatview.core

import fail.acheron.floatview.backend.BluetoothEvent
import fail.acheron.floatview.backend.BluetoothPermissionState
import fail.acheron.floatview.backend.BluetoothPort
import fail.acheron.floatview.backend.BluetoothRadioState
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.DeviceId
import java.security.SecureRandom
import java.nio.charset.CharacterCodingException

/** Maps Android Bluetooth APIs to raw, bounded, generation-scoped Rust bridge messages. */
internal class RustBluetoothPlatformAdapter(
    private val runtime: CorePlatformRuntime,
    private val bluetooth: BluetoothPort,
    private val diagnostic: (stage: String, code: String) -> Unit = { _, _ -> },
) {
    private val lock = Any()
    private val random = SecureRandom()
    private val tokenByDevice = mutableMapOf<DeviceId, ByteArray>()
    private val deviceByToken = mutableMapOf<String, DeviceId>()
    private var adapterOrder = 0L
    private var connectionGeneration = 0L
    private var activeDevice: DeviceId? = null
    private var pendingConnect: NativeHostCommand? = null
    private var stopped = false

    /** Returns false for non-Bluetooth commands so the composite host can dispatch them. */
    fun dispatch(command: NativeHostCommand): Boolean {
        val envelope =
            try {
                PlatformCommandEnvelope.decode(command)
            } catch (_: Exception) {
                diagnose("bridge_command", "invalid_payload")
                runtime.complete(command, RESULT_FAILED)
                return true
            }
        if (envelope.command is PlatformCommand.Host) return false
        if (synchronized(lock) { stopped }) {
            runtime.complete(command, RESULT_CANCELLED)
            return true
        }
        when (val value = envelope.command) {
            PlatformCommand.BluetoothStartScan -> {
                bluetooth.startDiscovery()
                runtime.complete(command)
            }
            PlatformCommand.BluetoothStopScan -> {
                bluetooth.stopDiscovery()
                runtime.complete(command)
            }
            is PlatformCommand.BluetoothConnect -> connect(command, value.deviceToken)
            is PlatformCommand.BluetoothDisconnect -> disconnect(command, value.connectionGeneration)
            is PlatformCommand.BluetoothWrite -> write(command, value.connectionGeneration, value.bytes)
            is PlatformCommand.Host -> error("Handled above")
        }
        return true
    }

    fun receive(event: BluetoothEvent) {
        if (synchronized(lock) { stopped }) return
        when (event) {
            is BluetoothEvent.StateChanged -> submitState(event.state)
            is BluetoothEvent.DeviceFound -> submitDevice(event)
            is BluetoothEvent.ScanFailed -> diagnose("bluetooth_scan", safeCode(event.reason))
            is BluetoothEvent.ConnectionChanged -> submitConnection(event)
            is BluetoothEvent.Notification -> submitNotification(event)
        }
    }

    fun submitHostEvent(event: PlatformEvent) = submit(event)

    fun requestPermission() = bluetooth.requestPermission()

    fun shutdown() {
        val context = synchronized(lock) {
            if (stopped) return
            stopped = true
            val value = activeDevice to pendingConnect
            activeDevice = null
            pendingConnect = null
            value
        }
        bluetooth.stopDiscovery()
        context.first?.let(bluetooth::disconnect)
        context.second?.let { runtime.complete(it, RESULT_CANCELLED) }
    }

    private fun connect(command: NativeHostCommand, token: ByteArray) {
        val device = synchronized(lock) { deviceByToken[token.key()] }
        if (device == null) {
            diagnose("bluetooth_connect", "unknown_token")
            runtime.complete(command, RESULT_UNAVAILABLE)
            return
        }
        val replaced = synchronized(lock) {
            val previous = pendingConnect
            pendingConnect = command
            activeDevice = device
            connectionGeneration = Math.addExact(connectionGeneration, 1)
            previous
        }
        replaced?.let { runtime.complete(it, RESULT_CANCELLED) }
        bluetooth.connect(device)
    }

    private fun disconnect(command: NativeHostCommand, expectedGeneration: Long) {
        val context = synchronized(lock) {
            Triple(connectionGeneration, activeDevice, pendingConnect).also { pendingConnect = null }
        }
        if (expectedGeneration != context.first || context.second == null) {
            runtime.complete(command, RESULT_CANCELLED)
            return
        }
        context.third?.let { runtime.complete(it, RESULT_CANCELLED) }
        bluetooth.disconnect(context.second!!)
        runtime.complete(command)
    }

    private fun write(command: NativeHostCommand, expectedGeneration: Long, bytes: ByteArray) {
        val context = synchronized(lock) { connectionGeneration to activeDevice }
        if (expectedGeneration != context.first || context.second == null) {
            runtime.complete(command, RESULT_CANCELLED)
            return
        }
        bluetooth.write(context.second!!, bytes.copyOf())
        runtime.complete(command)
    }

    private fun submitState(state: BluetoothState) {
        val radio =
            when (state.radio) {
                BluetoothRadioState.POWERED_ON -> BluetoothRadio.POWERED_ON
                BluetoothRadioState.POWERED_OFF -> BluetoothRadio.POWERED_OFF
                else -> BluetoothRadio.UNKNOWN
            }
        val authorization =
            when (state.connectPermission) {
                BluetoothPermissionState.GRANTED, BluetoothPermissionState.NOT_REQUIRED ->
                    BluetoothAuthorization.GRANTED
                BluetoothPermissionState.DENIED, BluetoothPermissionState.RESTRICTED ->
                    BluetoothAuthorization.DENIED
                else -> BluetoothAuthorization.UNKNOWN
            }
        submit(PlatformEvent.BluetoothState(radio, authorization))
    }

    private fun submitDevice(event: BluetoothEvent.DeviceFound) {
        val token = synchronized(lock) {
            tokenByDevice.getOrPut(event.deviceId) {
                ByteArray(16).also(random::nextBytes).also { deviceByToken[it.key()] = event.deviceId }
            }.copyOf()
        }
        submit(
            PlatformEvent.BluetoothDevice(
                token,
                event.advertisedName?.boundedUtf8(128)?.takeIf(String::isNotEmpty),
                (event.rssi ?: 0).coerceIn(Short.MIN_VALUE.toInt(), Short.MAX_VALUE.toInt()).toShort(),
            ),
        )
    }

    private fun submitConnection(event: BluetoothEvent.ConnectionChanged) {
        val context = synchronized(lock) {
            if (event.deviceId != activeDevice) return
            val mapped =
                when (event.state) {
                    ConnectionState.READY -> BluetoothConnection.READY
                    ConnectionState.FAILED -> BluetoothConnection.FAILED
                    ConnectionState.DISCONNECTED -> BluetoothConnection.DISCONNECTED
                    else -> BluetoothConnection.CONNECTING
                }
            val pending =
                if (
                    mapped == BluetoothConnection.READY ||
                    mapped == BluetoothConnection.FAILED ||
                    mapped == BluetoothConnection.DISCONNECTED
                ) {
                    pendingConnect.also { pendingConnect = null }
                } else {
                    null
                }
            if (mapped == BluetoothConnection.FAILED || mapped == BluetoothConnection.DISCONNECTED) {
                activeDevice = null
            }
            Triple(connectionGeneration, mapped, pending)
        }
        submit(PlatformEvent.BluetoothConnectionChanged(context.first, context.second))
        context.third?.let {
            runtime.complete(
                it,
                when (context.second) {
                    BluetoothConnection.READY -> RESULT_SUCCESS
                    BluetoothConnection.FAILED -> RESULT_FAILED
                    else -> RESULT_CANCELLED
                },
            )
        }
    }

    private fun submitNotification(event: BluetoothEvent.Notification) {
        synchronized(lock) {
            if (event.deviceId != activeDevice) return
            if (event.bytes.isEmpty() || event.bytes.size > MAX_BLUETOOTH_NOTIFICATION) {
                diagnose("bluetooth_notify", "payload_too_large")
                return
            }
            adapterOrder = Math.addExact(adapterOrder, 1)
            runtime.submitBluetoothBytes(
                connectionGeneration,
                adapterOrder,
                event.bytes.copyOf(),
            )
        }
    }

    private fun submit(event: PlatformEvent) {
        synchronized(lock) {
            adapterOrder = Math.addExact(adapterOrder, 1)
            runtime.submitPlatformEvent(
                PlatformEventEnvelope(
                    runtime.runtimeGeneration,
                    connectionGeneration,
                    adapterOrder,
                    event,
                ).encode(),
            )
        }
    }

    private fun diagnose(stage: String, code: String) = diagnostic(safeCode(stage), safeCode(code))

    private companion object {
        const val RESULT_SUCCESS = 0
        const val RESULT_CANCELLED = 1
        const val RESULT_UNAVAILABLE = 3
        const val RESULT_FAILED = 4
        const val MAX_BLUETOOTH_NOTIFICATION = 4096
    }
}

private fun ByteArray.key(): String = joinToString("") { byte -> "%02x".format(byte) }

private fun String.boundedUtf8(maximum: Int): String {
    var bytes = encodeToByteArray().take(maximum).toByteArray()
    while (bytes.isNotEmpty()) {
        try {
            return bytes.decodeToString(throwOnInvalidSequence = true)
        } catch (_: CharacterCodingException) {
            bytes = bytes.copyOf(bytes.size - 1)
        }
    }
    return ""
}

private fun safeCode(value: String): String =
    value.lowercase().map { character ->
        if (character.isLetterOrDigit() || character == '_' || character == '-' || character == '.') {
            character
        } else {
            '_'
        }
    }.take(64).joinToString("")
