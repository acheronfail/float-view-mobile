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

data class SessionSummaryRecord(
    val sessionId: String,
    val deviceHistoryId: String,
    val displayName: String,
    val startedAt: Instant,
    val endedAt: Instant?,
    val state: String,
    val integrity: String,
    val sampleCount: Long,
    val eventCount: Long,
    val ridingElapsedMs: Long,
    val totalElapsedMs: Long,
    val distanceM: Double?,
    val maxSpeedMps: Double?,
    val maxDutyCycle: Double?,
)

data class SessionPage(
    val items: List<SessionSummaryRecord>,
    val nextCursor: String?,
)

data class SessionListQuery(
    val cursor: String? = null,
    val limit: Int = 20,
    val deviceHistoryId: String? = null,
    val startedAfter: Instant? = null,
    val startedBefore: Instant? = null,
    val minimumTotalElapsedMs: Long? = null,
    val maximumTotalElapsedMs: Long? = null,
    val minimumRidingElapsedMs: Long? = null,
    val maximumRidingElapsedMs: Long? = null,
    val minimumDistanceM: Double? = null,
    val maximumDistanceM: Double? = null,
    val state: String? = null,
    val integrity: String? = null,
    val sortField: String = "startedAt",
    val sortDescending: Boolean = true,
)

data class SessionSeriesPoint(
    val elapsedMs: Long,
    val values: Map<String, Double?>,
)

data class SessionEventRecord(
    val eventId: String,
    val elapsedMs: Long,
    val kind: String,
    val detailsJson: String,
)

data class SessionGroupRecord(
    val sessionGroupId: String,
    val deviceHistoryId: String,
    val sessionIds: List<String>,
    val totalElapsedMs: Long,
    val distanceM: Double?,
)

data class SessionExportRecord(
    val exportId: String,
    val fileName: String,
    val filePath: String,
)

data class SettingsRecord(
    val revision: Int = 0,
    val idleProbeIntervalMs: Long = 500,
    val idlePersistIntervalMs: Long = 1_000,
    val ridingPersistIntervalMs: Long = 100,
    val postRideTailMs: Long = 15_000,
    val reconnectDurationMs: Long = 300_000,
    val automaticSessionLinkingEnabled: Boolean = false,
    val sessionLinkWindowMs: Long = 7_200_000,
    val rawPacketCaptureEnabled: Boolean = false,
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
    fun schedule(delayMs: Long, action: () -> Unit): CancellationPort
    fun scheduleRepeating(intervalMs: Long, action: () -> Unit): CancellationPort
}

class SystemScheduler : SchedulerPort {
    private val executor = Executors.newSingleThreadScheduledExecutor { runnable ->
        Thread(runnable, "float-view-telemetry").apply { isDaemon = true }
    }

    override fun schedule(delayMs: Long, action: () -> Unit): CancellationPort {
        require(delayMs >= 0)
        val future = executor.schedule(action, delayMs, TimeUnit.MILLISECONDS)
        return CancellationPort { future.cancel(false) }
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

data class SavedDeviceRecord(
    val savedDeviceId: String,
    val deviceHistoryId: String,
    val displayName: String,
    val advertisedName: String? = null,
    val lastSeenAt: Instant? = null,
    val lastConnectedAt: Instant? = null,
    val profileComplete: Boolean,
)

data class BoardProfileRecord(
    val savedDeviceId: String,
    val revision: Int,
    val valuesJson: String,
    val missingFields: List<String>,
)

sealed class ProductStoreException(message: String) : RuntimeException(message) {
    data object SavedDeviceNotFound : ProductStoreException("Saved device not found")
    data class ProfileRevisionConflict(val currentRevision: Int) :
        ProductStoreException("Profile revision conflict")
    data object InvalidProfilePatch : ProductStoreException("Invalid profile patch")
    data class SettingsRevisionConflict(val currentRevision: Int) :
        ProductStoreException("Settings revision conflict")
    data object InvalidSettingsPatch : ProductStoreException("Invalid settings patch")
}

interface ProductStorePort {
    fun savedDevices(): List<SavedDeviceRecord>
    fun boardProfile(savedDeviceId: String): BoardProfileRecord
    fun updateBoardProfile(
        savedDeviceId: String,
        expectedRevision: Int,
        patchJson: String,
        at: Instant,
    ): BoardProfileRecord
    fun settings(): SettingsRecord = SettingsRecord()
    fun updateSettings(expectedRevision: Int, patchJson: String): SettingsRecord =
        throw ProductStoreException.InvalidSettingsPatch
}

sealed class SessionStoreException(message: String) : RuntimeException(message) {
    data object SessionNotFound : SessionStoreException("Session not found")
    data object SessionActive : SessionStoreException("Active session cannot be changed")
    data object InvalidCursor : SessionStoreException("Invalid session cursor")
    data object InvalidRange : SessionStoreException("Invalid session range")
    data object InvalidExportFormat : SessionStoreException("Invalid export format")
    data class UnsupportedField(val field: String) :
        SessionStoreException("Unsupported session field: $field")
}

interface SessionStorePort {
    fun sessions(query: SessionListQuery): SessionPage
    fun session(sessionId: String): SessionSummaryRecord
    fun querySeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
    ): List<SessionSeriesPoint>
    fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
    ): List<SessionEventRecord>
    fun diagnosticWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
    ): List<SessionSeriesPoint>
    fun deleteSession(sessionId: String) {
        throw SessionStoreException.SessionNotFound
    }
    fun exportSession(sessionId: String, format: String): SessionExportRecord =
        throw SessionStoreException.SessionNotFound
    fun sessionGroups(): List<SessionGroupRecord> = emptyList()
    fun sessionGroup(sessionGroupId: String): SessionGroupRecord =
        throw SessionStoreException.SessionNotFound
    fun unlinkSessionGroup(sessionGroupId: String) = Unit
}

fun interface EventSink {
    fun publish(event: Any)
}

class EventHub : EventSink {
    private val listeners = java.util.concurrent.CopyOnWriteArrayList<(Any) -> Unit>()

    override fun publish(event: Any) {
        listeners.forEach { it(event) }
    }

    fun subscribe(listener: (Any) -> Unit): AutoCloseable {
        listeners += listener
        return AutoCloseable { listeners -= listener }
    }
}
