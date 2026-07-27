package fail.acheron.floatview.core

import java.time.Instant

@JvmInline
value class DeviceId(val value: String)

@JvmInline
value class SessionId(val value: String)

@JvmInline
value class JobId(val value: String)

enum class ConnectionState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    DISCOVERING_SERVICES,
    ENABLING_NOTIFICATIONS,
    READY,
    RECONNECTING,
    DISCONNECTING,
    FAILED
}

enum class RideState { IDLE, RIDING, UNKNOWN }

data class DiscoveredDevice(
    val id: DeviceId,
    val advertisedName: String?,
    val rssi: Int?
)

data class TelemetrySample(
    val sequence: Long,
    val observedAt: Instant,
    val elapsedMs: Long,
    val rideState: RideState,
    val packVoltageV: Double?,
    val dutyCycle: Double?,
    val controllerTempC: Double?,
    val motorTempC: Double?,
    val footpadState: Int?,
    val extension: Map<String, Double> = emptyMap()
)

data class SessionRecord(
    val sessionId: SessionId,
    val jobId: JobId,
    val deviceId: DeviceId,
    val startedAt: Instant,
    val committedSequence: Long,
    val active: Boolean
)

sealed interface BluetoothEvent {
    data class StateChanged(val powered: Boolean) : BluetoothEvent
    data class DeviceFound(val device: DiscoveredDevice) : BluetoothEvent
    data class ConnectionChanged(val deviceId: DeviceId, val state: ConnectionState) : BluetoothEvent
    data class Notification(val deviceId: DeviceId, val bytes: ByteArray) : BluetoothEvent
}

interface BluetoothPort {
    fun startDiscovery()
    fun stopDiscovery()
    fun connect(deviceId: DeviceId)
    fun write(deviceId: DeviceId, bytes: ByteArray)
    fun disconnect(deviceId: DeviceId)
}

interface ClockPort {
    fun monotonicMillis(): Long
    fun now(): Instant
}

interface LogStorePort {
    fun create(deviceId: DeviceId, at: Instant): SessionRecord
    fun append(jobId: JobId, samples: List<TelemetrySample>)
    fun close(jobId: JobId, at: Instant, reason: String): SessionRecord
    fun active(): SessionRecord?
}

fun interface EventSink {
    fun publish(event: Any)
}
