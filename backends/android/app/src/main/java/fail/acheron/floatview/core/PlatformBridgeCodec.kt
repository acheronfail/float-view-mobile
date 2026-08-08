package fail.acheron.floatview.core

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

internal const val MAX_BRIDGE_PAYLOAD = 64 * 1024

internal enum class BluetoothRadio(val tag: Int) { UNKNOWN(1), POWERED_OFF(2), POWERED_ON(3) }
internal enum class BluetoothAuthorization(val tag: Int) { UNKNOWN(1), DENIED(2), GRANTED(3) }
internal enum class BluetoothConnection(val tag: Int) { CONNECTING(1), READY(2), DISCONNECTED(3), FAILED(4) }
internal enum class ResourceNamespace(val tag: Int) { FRONTEND(1), SQL(2), SOFTWARE_FIXTURE(3) }
internal enum class StorageRootKind(val tag: Int) { DURABLE(1), CACHE(2), TEMPORARY(3) }
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
    fun encode(): ByteArray {
        val writer = BridgeWriter()
        when (val value = command) {
            PlatformCommand.BluetoothStartScan -> Unit
            PlatformCommand.BluetoothStopScan -> Unit
            is PlatformCommand.BluetoothConnect -> {
                writer.bytes(value.deviceToken, 256)
            }
            is PlatformCommand.BluetoothDisconnect -> {
                writer.i64(value.connectionGeneration)
            }
            is PlatformCommand.BluetoothWrite -> {
                writer.i64(value.connectionGeneration)
                writer.bytes(value.bytes, MAX_BRIDGE_PAYLOAD)
            }
            is PlatformCommand.Host -> {
                writer.raw(value.payload)
            }
        }
        return writer.finish()
    }

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
                    in 6..22 -> PlatformCommand.Host(tag, reader.remaining())
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
    data class ReadResource(
        val namespace: ResourceNamespace,
        val logicalName: String,
        val maximumBytes: Long,
    ) : AndroidHostCommand
    data class StageDocument(val destination: String, val maximumBytes: Long) : AndroidHostCommand
    data class ResolveStorageRoot(val kind: StorageRootKind) : AndroidHostCommand
    data class AvailableSpace(val root: String) : AndroidHostCommand
    data object MonotonicClock : AndroidHostCommand
    data object WallClock : AndroidHostCommand
    data class SecureRandom(val count: Int) : AndroidHostCommand
    data class Diagnostic(val stage: String, val code: String) : AndroidHostCommand
    data class PresentPermission(val permission: PermissionKind) : AndroidHostCommand
    data object PresentSettings : AndroidHostCommand
    data class PresentTheme(val theme: PlatformTheme) : AndroidHostCommand
    data class PresentServiceState(val active: Boolean) : AndroidHostCommand
    data class PresentExport(val stagedPath: String) : AndroidHostCommand
    data class RuntimeReady(val endpoint: String, val launchToken: String) : AndroidHostCommand

    companion object {
        fun decode(value: PlatformCommand.Host): AndroidHostCommand {
            val reader = BridgeReader(value.payload)
            val command =
                when (value.tag) {
                    6 -> LocationStart(reader.boolean())
                    7 -> LocationStop
                    8 -> SetBackgroundLogging(reader.boolean())
                    9 -> ReadResource(
                        tagged(reader.u8(), ResourceNamespace.entries) { it.tag },
                        reader.string(128),
                        reader.i64(),
                    )
                    10 -> StageDocument(reader.string(1024), reader.i64())
                    11 -> ResolveStorageRoot(
                        tagged(reader.u8(), StorageRootKind.entries) { it.tag },
                    )
                    12 -> AvailableSpace(reader.string(1024))
                    13 -> MonotonicClock
                    14 -> WallClock
                    15 -> SecureRandom(reader.u16())
                    16 -> Diagnostic(reader.string(64), reader.string(64))
                    17 -> PresentPermission(
                        tagged(reader.u8(), PermissionKind.entries) { it.tag },
                    )
                    18 -> PresentSettings
                    19 -> PresentTheme(
                        tagged(reader.u8(), PlatformTheme.entries) { it.tag },
                    )
                    20 -> PresentServiceState(reader.boolean())
                    21 -> PresentExport(reader.string(1024))
                    22 -> RuntimeReady(reader.string(256), reader.string(128))
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
