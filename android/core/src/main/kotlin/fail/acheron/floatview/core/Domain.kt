package fail.acheron.floatview.core

import java.time.Instant
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

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

enum class BluetoothRadioState { UNKNOWN, UNSUPPORTED, POWERED_OFF, POWERED_ON, RESETTING }

enum class BluetoothPermissionState {
    UNKNOWN,
    NOT_REQUIRED,
    NOT_DETERMINED,
    DENIED,
    RESTRICTED,
    GRANTED
}

data class BluetoothState(
    val radio: BluetoothRadioState,
    val scanPermission: BluetoothPermissionState,
    val connectPermission: BluetoothPermissionState,
)

data class DiscoverySnapshot(
    val bluetoothState: BluetoothState,
    val activeScanId: String?,
    val knownDevices: List<DiscoveredDevice>,
)

data class DiscoveredDevice(
    val id: DeviceId,
    val advertisedName: String?,
    val rssi: Int?,
    val lastSeenAt: Instant,
)

data class ActiveConnectionSnapshot(
    val deviceId: DeviceId,
    val advertisedName: String?,
    val state: ConnectionState,
)

data class FirmwareRecord(
    val major: Int,
    val minor: Int,
    val hardwareName: String?,
    val firmwareName: String?,
    val hardwareType: Int = 0,
    val canId: Int? = null,
)

sealed interface VescProtocolMessage {
    data class Firmware(val value: FirmwareRecord) : VescProtocolMessage
    data class CanDevices(val canIds: List<Int>) : VescProtocolMessage
    data class RefloatInfo(val version: String) : VescProtocolMessage
    data class RefloatTelemetryLayout(
        val fields: List<String>,
        val runtimeFields: List<String>,
    ) : VescProtocolMessage
    data class RefloatTelemetry(val value: RefloatTelemetryRecord) : VescProtocolMessage
}

data class RefloatTelemetryRecord(
    val rideState: RideState,
    val boardState: Int,
    val footpadState: Int,
    val packVoltageV: Double,
    val dutyCycle: Double,
    val controllerTempC: Double?,
    val motorTempC: Double?,
    val motorCurrentA: Double,
    val batteryCurrentA: Double,
    val erpm: Int,
    val speedKph: Double,
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
    data class StateChanged(val state: BluetoothState) : BluetoothEvent
    data class DeviceFound(
        val deviceId: DeviceId,
        val advertisedName: String?,
        val rssi: Int?,
    ) : BluetoothEvent
    data class ScanFailed(val reason: String) : BluetoothEvent
    data class ConnectionChanged(val deviceId: DeviceId, val state: ConnectionState) : BluetoothEvent
    data class Notification(val deviceId: DeviceId, val bytes: ByteArray) : BluetoothEvent
}

interface BluetoothPort {
    fun currentState(): BluetoothState
    fun requestPermission()
    fun startDiscovery()
    fun stopDiscovery()
    fun connect(deviceId: DeviceId)
    fun write(deviceId: DeviceId, bytes: ByteArray)
    fun disconnect(deviceId: DeviceId)
}

interface VescProtocolPort {
    fun reset()
    fun firmwareRequest(canId: Int? = null): ByteArray
    fun canPingRequest(): ByteArray
    fun refloatInfoRequest(canId: Int? = null): ByteArray
    fun refloatTelemetryIdsRequest(canId: Int): ByteArray
    fun refloatTelemetryRequest(canId: Int): ByteArray
    fun receive(bytes: ByteArray): List<VescProtocolMessage>
}

fun interface CancellationPort {
    fun cancel()
}

interface SchedulerPort {
    fun scheduleRepeating(intervalMs: Long, action: () -> Unit): CancellationPort
}

class SystemScheduler : SchedulerPort {
    private val executor = Executors.newSingleThreadScheduledExecutor { runnable ->
        Thread(runnable, "float-view-telemetry").apply { isDaemon = true }
    }

    override fun scheduleRepeating(intervalMs: Long, action: () -> Unit): CancellationPort {
        require(intervalMs > 0)
        val future = executor.scheduleAtFixedRate(
            action,
            intervalMs,
            intervalMs,
            TimeUnit.MILLISECONDS,
        )
        return CancellationPort { future.cancel(false) }
    }
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
