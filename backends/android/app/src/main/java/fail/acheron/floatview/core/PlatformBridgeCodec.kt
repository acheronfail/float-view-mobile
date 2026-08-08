package fail.acheron.floatview.core

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

internal const val MAX_BRIDGE_PAYLOAD = 64 * 1024

internal enum class BluetoothRadio(val tag: Int) { UNKNOWN(1), POWERED_OFF(2), POWERED_ON(3) }
internal enum class BluetoothAuthorization(val tag: Int) { UNKNOWN(1), DENIED(2), GRANTED(3) }
internal enum class BluetoothConnection(val tag: Int) { CONNECTING(1), READY(2), DISCONNECTED(3), FAILED(4) }
internal enum class PermissionKind(val tag: Int) { BLUETOOTH(1), LOCATION(2), BACKGROUND_LOCATION(3), NOTIFICATIONS(4) }
internal enum class PlatformTheme(val tag: Int) { SYSTEM(1), LIGHT(2), DARK(3) }
internal enum class LocationAuthorization(val tag: Int) { UNKNOWN(1), DENIED(2), FOREGROUND(3), BACKGROUND(4) }

internal sealed interface PlatformCommand {
    data object BluetoothStartScan : PlatformCommand
    data object BluetoothStopScan : PlatformCommand
    data class BluetoothConnect(val deviceToken: ByteArray) : PlatformCommand
    data class BluetoothDisconnect(val connectionGeneration: Long) : PlatformCommand
    data class BluetoothWrite(val connectionGeneration: Long, val bytes: ByteArray) : PlatformCommand
    data class Host(val tag: Int, val payload: ByteArray = byteArrayOf()) : PlatformCommand
}

internal val PlatformCommand.kind: Int
    get() = when (this) {
        PlatformCommand.BluetoothStartScan -> 1
        PlatformCommand.BluetoothStopScan -> 2
        is PlatformCommand.BluetoothConnect -> 3
        is PlatformCommand.BluetoothDisconnect -> 4
        is PlatformCommand.BluetoothWrite -> 5
        is PlatformCommand.Host -> tag
    }

internal data class PlatformCommandEnvelope(
    val runtimeGeneration: Long,
    val operationId: Long,
    val command: PlatformCommand,
) {
    companion object {
        fun decode(command: NativeHostCommand): PlatformCommandEnvelope {
            val reader = BridgeReader(command.payload)
            val decoded =
                when (val tag = command.kind) {
                    1 -> PlatformCommand.BluetoothStartScan
                    2 -> PlatformCommand.BluetoothStopScan
                    3 -> PlatformCommand.BluetoothConnect(reader.bytes(256))
                    4 -> PlatformCommand.BluetoothDisconnect(reader.i64())
                    5 -> PlatformCommand.BluetoothWrite(reader.i64(), reader.bytes(MAX_BRIDGE_PAYLOAD))
                    in 6..11 -> PlatformCommand.Host(tag, reader.remaining())
                    else -> error("Unknown platform command tag")
                }
            reader.finish()
            return PlatformCommandEnvelope(command.runtimeGeneration, command.operationId, decoded)
        }
    }
}

internal sealed interface PlatformEvent {
    data class BluetoothState(
        val radio: BluetoothRadio,
        val authorization: BluetoothAuthorization,
    ) : PlatformEvent
    data class BluetoothDevice(val token: ByteArray, val name: String?, val rssi: Short) : PlatformEvent
    data class BluetoothConnectionChanged(
        val connectionGeneration: Long,
        val state: BluetoothConnection,
    ) : PlatformEvent
    data class LocationAuthorizationChanged(val authorization: LocationAuthorization) : PlatformEvent
    data class Location(val latitude: Double, val longitude: Double, val accuracyM: Double?) : PlatformEvent
    data class Lifecycle(val foreground: Boolean, val backgroundEligible: Boolean) : PlatformEvent
    data class LoggingService(val active: Boolean) : PlatformEvent
    data class Theme(val theme: PlatformTheme) : PlatformEvent
}

internal sealed interface AndroidHostCommand {
    data class LocationStart(val background: Boolean) : AndroidHostCommand
    data object LocationStop : AndroidHostCommand
    data class SetBackgroundLogging(val enabled: Boolean) : AndroidHostCommand
    data class PresentPermission(val permission: PermissionKind) : AndroidHostCommand
    data object PresentSettings : AndroidHostCommand
    data class RuntimeReady(val endpoint: String, val launchToken: String) : AndroidHostCommand

    companion object {
        fun decode(value: PlatformCommand.Host): AndroidHostCommand {
            val reader = BridgeReader(value.payload)
            val command =
                when (value.tag) {
                    6 -> LocationStart(reader.boolean())
                    7 -> LocationStop
                    8 -> SetBackgroundLogging(reader.boolean())
                    9 -> PresentPermission(
                        tagged(reader.u8(), PermissionKind.entries) { it.tag },
                    )
                    10 -> PresentSettings
                    11 -> RuntimeReady(reader.string(256), reader.string(128))
                    else -> error("Unknown Android host command")
                }
            reader.finish()
            return command
        }
    }
}

private fun <T> tagged(value: Int, entries: Iterable<T>, tag: (T) -> Int): T =
    entries.single { tag(it) == value }

internal class BridgeWriter {
    private val output = ByteArrayOutputStream()

    fun u8(value: Int) {
        require(value in 0..255)
        reserve(1)
        output.write(value)
    }

    fun i16(value: Short) = raw(ByteBuffer.allocate(2).order(ByteOrder.BIG_ENDIAN).putShort(value).array())
    fun u16(value: Int) {
        require(value in 0..65535)
        raw(ByteBuffer.allocate(2).order(ByteOrder.BIG_ENDIAN).putShort(value.toShort()).array())
    }
    fun i64(value: Long) = raw(ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN).putLong(value).array())
    fun f64(value: Double) = i64(value.toBits())
    fun boolean(value: Boolean) = u8(if (value) 1 else 0)

    fun bytes(value: ByteArray, maximum: Int) {
        require(value.isNotEmpty() && value.size <= maximum)
        raw(ByteBuffer.allocate(4).order(ByteOrder.BIG_ENDIAN).putInt(value.size).array())
        raw(value)
    }

    fun optionalString(value: String?, maximum: Int) {
        u8(if (value == null) 0 else 1)
        if (value != null) bytes(value.encodeToByteArray(), maximum)
    }

    fun optionalF64(value: Double?) {
        boolean(value != null)
        value?.let(::f64)
    }

    fun raw(value: ByteArray) {
        reserve(value.size)
        output.write(value)
    }

    private fun reserve(count: Int) {
        require(count >= 0 && output.size() <= MAX_BRIDGE_PAYLOAD - count)
    }

    fun finish(): ByteArray = output.toByteArray()
}

internal class BridgeReader(private val bytes: ByteArray) {
    private var offset = 0

    init {
        require(bytes.size <= MAX_BRIDGE_PAYLOAD)
    }

    fun u8(): Int = take(1)[0].toInt() and 0xff
    fun u16(): Int = ByteBuffer.wrap(take(2)).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xffff
    fun i16(): Short = ByteBuffer.wrap(take(2)).order(ByteOrder.BIG_ENDIAN).short
    fun i64(): Long = ByteBuffer.wrap(take(8)).order(ByteOrder.BIG_ENDIAN).long
    fun f64(): Double = Double.fromBits(i64())
    fun boolean(): Boolean = when (u8()) { 0 -> false; 1 -> true; else -> error("Invalid Boolean") }
    fun string(maximum: Int): String = bytes(maximum).decodeToString(throwOnInvalidSequence = true)

    fun bytes(maximum: Int): ByteArray {
        val count = ByteBuffer.wrap(take(4)).order(ByteOrder.BIG_ENDIAN).int
        require(count in 1..maximum)
        return take(count)
    }

    fun optionalString(maximum: Int): String? =
        when (u8()) {
            0 -> null
            1 -> bytes(maximum).decodeToString()
            else -> error("Invalid optional string tag")
        }

    fun optionalF64(): Double? = when (u8()) { 0 -> null; 1 -> f64(); else -> error("Invalid optional f64 tag") }

    private fun take(count: Int): ByteArray {
        val end = Math.addExact(offset, count)
        require(end <= bytes.size)
        return bytes.copyOfRange(offset, end).also { offset = end }
    }

    fun remaining(): ByteArray = take(bytes.size - offset)

    fun finish() = require(offset == bytes.size)
}
