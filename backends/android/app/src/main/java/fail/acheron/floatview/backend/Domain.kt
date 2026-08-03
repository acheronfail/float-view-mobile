package fail.acheron.floatview.backend

import java.time.Instant
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import fail.acheron.floatview.protocol.vesc.ControllerRoute

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
    IDENTIFYING,
    SELECTING_CONTROLLER,
    READY,
    RECONNECTING,
    DISCONNECTING,
    FAILED
}

data class ControllerCandidate(
    val canId: Int,
    val hardwareName: String?,
    val firmwareName: String?,
    val vescVersion: String,
    val vescSupported: Boolean,
)

enum class ConnectionIssueCode {
    TRANSPORT_SETUP_FAILED,
    IDENTIFICATION_TIMEOUT,
    NO_CONTROLLER_FOUND,
    TOO_MANY_CAN_DEVICES,
    TOO_MANY_CONTROLLERS,
    PREFERRED_CONTROLLER_UNAVAILABLE,
    CONTROLLER_CHANGED,
    UNSUPPORTED_CONTROLLER,
    UNSUPPORTED_REFLOAT,
}

data class ConnectionIssue(
    val code: ConnectionIssueCode,
    val message: String,
)

enum class RideState { IDLE, RIDING, UNKNOWN }

enum class PackageState(val wireValue: String) {
    DISABLED("disabled"), STARTUP("startup"), READY("ready"), RUNNING("running"), UNKNOWN("unknown")
}

enum class PackageMode(val wireValue: String) {
    NORMAL("normal"), HAND_TEST("handTest"), FLYWHEEL("flywheel"), UNKNOWN("unknown")
}

enum class FootpadState(val wireValue: String) {
    NONE("none"), LEFT("left"), RIGHT("right"), BOTH("both"), UNKNOWN("unknown")
}

enum class StopCondition(val wireValue: String) {
    NONE("none"), PITCH("pitch"), ROLL("roll"), SWITCH_HALF("switchHalf"),
    SWITCH_FULL("switchFull"), REVERSE_STOP("reverseStop"), QUICK_STOP("quickStop"),
    UNKNOWN("unknown")
}

enum class SetpointAdjustment(val wireValue: String) {
    NONE("none"), CENTERING("centering"), REVERSE_STOP("reverseStop"), SPEED("speed"),
    DUTY("duty"), ERROR("error"), HIGH_VOLTAGE("highVoltage"), LOW_VOLTAGE("lowVoltage"),
    TEMPERATURE("temperature"), UNKNOWN("unknown")
}

enum class AlertReason(val wireValue: String) {
    NONE("none"), LOW_VOLTAGE("lowVoltage"), HIGH_VOLTAGE("highVoltage"),
    CONTROLLER_TEMPERATURE("controllerTemperature"), MOTOR_TEMPERATURE("motorTemperature"),
    CURRENT("current"), DUTY("duty"), SENSORS("sensors"), LOW_BATTERY("lowBattery"),
    IDLE("idle"), ERROR("error"), SPEED("speed"), CELL_UNDER_TEMPERATURE("cellUnderTemperature"),
    CELL_OVER_TEMPERATURE("cellOverTemperature"), CELL_LOW_VOLTAGE("cellLowVoltage"),
    CELL_HIGH_VOLTAGE("cellHighVoltage"), CELL_IMBALANCE("cellImbalance"),
    BMS_CONNECTION("bmsConnection"), BMS_OVER_TEMPERATURE("bmsOverTemperature"),
    FIRMWARE_FAULT("firmwareFault"), UNKNOWN("unknown")
}

enum class FirmwareFault(val wireValue: String) {
    OVER_VOLTAGE("overVoltage"), UNDER_VOLTAGE("underVoltage"), DRIVER("driver"),
    ABSOLUTE_OVER_CURRENT("absoluteOverCurrent"),
    CONTROLLER_OVER_TEMPERATURE("controllerOverTemperature"),
    MOTOR_OVER_TEMPERATURE("motorOverTemperature"),
    GATE_DRIVER_OVER_VOLTAGE("gateDriverOverVoltage"),
    GATE_DRIVER_UNDER_VOLTAGE("gateDriverUnderVoltage"), MCU_UNDER_VOLTAGE("mcuUnderVoltage"),
    WATCHDOG_RESET("watchdogReset"), ENCODER_SPI("encoderSpi"),
    ENCODER_SINCOS_BELOW_MINIMUM("encoderSinCosBelowMinimum"),
    ENCODER_SINCOS_ABOVE_MAXIMUM("encoderSinCosAboveMaximum"),
    FLASH_CORRUPTION("flashCorruption"), CURRENT_SENSOR_OFFSET_1("currentSensorOffset1"),
    CURRENT_SENSOR_OFFSET_2("currentSensorOffset2"), CURRENT_SENSOR_OFFSET_3("currentSensorOffset3"),
    UNBALANCED_CURRENTS("unbalancedCurrents"), BRAKE("brake"),
    RESOLVER_LOSS_OF_TRACKING("resolverLossOfTracking"),
    RESOLVER_DEGRADATION_OF_SIGNAL("resolverDegradationOfSignal"),
    RESOLVER_LOSS_OF_SIGNAL("resolverLossOfSignal"),
    APP_CONFIG_FLASH_CORRUPTION("appConfigFlashCorruption"),
    MOTOR_CONFIG_FLASH_CORRUPTION("motorConfigFlashCorruption"),
    ENCODER_NO_MAGNET("encoderNoMagnet"), ENCODER_MAGNET_TOO_STRONG("encoderMagnetTooStrong"),
    PHASE_FILTER("phaseFilter"), UNKNOWN("unknown")
}

enum class RefloatTelemetryKind { REALTIME, COMPACT }

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
    val savedDevice: SavedDeviceRecord? = null,
    val activeSession: SessionSummaryRecord? = null,
    val latestSample: TelemetrySample? = null,
    val currentLimits: ControllerCurrentLimits? = null,
    val controllerRoute: ControllerRoute? = null,
    val controllerCandidates: List<ControllerCandidate> = emptyList(),
    val connectionIssue: ConnectionIssue? = null,
)

data class ControllerCurrentLimits(
    val motorCurrentMaxA: Double,
    val motorCurrentMinA: Double,
    val batteryCurrentMaxA: Double,
    val batteryCurrentMinA: Double,
    val batterySeriesCells: Int? = null,
)

data class FirmwareRecord(
    val major: Int,
    val minor: Int,
    val hardwareName: String?,
    val firmwareName: String?,
    val hardwareType: Int = 0,
    val canId: Int? = null,
    /** Private controller identity; never serialize into API/log payloads. */
    val controllerUuid: ByteArray? = null,
    val testVersion: Int = 0,
)

sealed interface VescProtocolMessage {
    data class Firmware(val value: FirmwareRecord) : VescProtocolMessage
    data class CurrentLimits(val value: ControllerCurrentLimits) : VescProtocolMessage
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
    val kind: RefloatTelemetryKind = RefloatTelemetryKind.REALTIME,
    val packageState: PackageState? = null,
    val packageMode: PackageMode? = null,
    val footpadState: FootpadState? = null,
    val stopCondition: StopCondition? = null,
    val setpointAdjustment: SetpointAdjustment? = null,
    val wheelslip: Boolean? = null,
    val darkride: Boolean? = null,
    val charging: Boolean? = null,
    val alertReason: AlertReason? = null,
    val firmwareFault: FirmwareFault? = null,
    val packVoltageV: Double? = null,
    val dutyCycle: Double? = null,
    val controllerTempC: Double? = null,
    val motorTempC: Double? = null,
    val motorCurrentA: Double? = null,
    val batteryCurrentA: Double? = null,
    val erpm: Double? = null,
    val speedKph: Double? = null,
    val pitchDeg: Double? = null,
    val balancePitchDeg: Double? = null,
    val rollDeg: Double? = null,
    val directionalMotorCurrentA: Double? = null,
    val filteredMotorCurrentA: Double? = null,
    val requestedBalanceCurrentA: Double? = null,
    val boosterCurrentA: Double? = null,
    val fieldWeakeningCurrentA: Double? = null,
    val footpadAdc1V: Double? = null,
    val footpadAdc2V: Double? = null,
    val remoteInput: Double? = null,
    val setpointDeg: Double? = null,
    val atrSetpointDeg: Double? = null,
    val brakeTiltSetpointDeg: Double? = null,
    val torqueTiltSetpointDeg: Double? = null,
    val turnTiltSetpointDeg: Double? = null,
    val remoteSetpointDeg: Double? = null,
    val atrAccelerationDifferenceErpmPerSecond: Double? = null,
    val atrSpeedBoost: Double? = null,
    val distanceM: Double? = null,
    val odometerM: Double? = null,
    val ampHoursDrawnAh: Double? = null,
    val ampHoursChargedAh: Double? = null,
    val wattHoursDrawnWh: Double? = null,
    val wattHoursChargedWh: Double? = null,
    val batteryLevel: Double? = null,
    val chargingCurrentA: Double? = null,
    val chargingVoltageV: Double? = null,
    val extensions: Map<String, Double> = emptyMap(),
)

data class TelemetrySample(
    val sequence: Long,
    val observedAt: Instant,
    val elapsedMs: Long,
    val rideState: RideState,
    val slowObservedAt: Instant? = null,
    val packageState: PackageState? = null,
    val packageMode: PackageMode? = null,
    val footpadState: FootpadState? = null,
    val stopCondition: StopCondition? = null,
    val setpointAdjustment: SetpointAdjustment? = null,
    val wheelslip: Boolean? = null,
    val darkride: Boolean? = null,
    val charging: Boolean? = null,
    val alertReason: AlertReason? = null,
    val firmwareFault: FirmwareFault? = null,
    val packVoltageV: Double? = null,
    val dutyCycle: Double? = null,
    val controllerTempC: Double? = null,
    val motorTempC: Double? = null,
    val motorCurrentA: Double? = null,
    val batteryCurrentA: Double? = null,
    val erpm: Double? = null,
    val speedKph: Double? = null,
    val pitchDeg: Double? = null,
    val balancePitchDeg: Double? = null,
    val rollDeg: Double? = null,
    val directionalMotorCurrentA: Double? = null,
    val filteredMotorCurrentA: Double? = null,
    val requestedBalanceCurrentA: Double? = null,
    val boosterCurrentA: Double? = null,
    val fieldWeakeningCurrentA: Double? = null,
    val footpadAdc1V: Double? = null,
    val footpadAdc2V: Double? = null,
    val remoteInput: Double? = null,
    val setpointDeg: Double? = null,
    val atrSetpointDeg: Double? = null,
    val brakeTiltSetpointDeg: Double? = null,
    val torqueTiltSetpointDeg: Double? = null,
    val turnTiltSetpointDeg: Double? = null,
    val remoteSetpointDeg: Double? = null,
    val atrAccelerationDifferenceErpmPerSecond: Double? = null,
    val atrSpeedBoost: Double? = null,
    val distanceM: Double? = null,
    val odometerM: Double? = null,
    val ampHoursDrawnAh: Double? = null,
    val ampHoursChargedAh: Double? = null,
    val wattHoursDrawnWh: Double? = null,
    val wattHoursChargedWh: Double? = null,
    val batteryLevel: Double? = null,
    val chargingCurrentA: Double? = null,
    val chargingVoltageV: Double? = null,
    val extension: Map<String, Double> = emptyMap(),
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

data class SessionPublishedSnapshotRecord(
    val historyRevision: Long,
    val committedSampleSequence: Long,
    val committedEventSequence: Long,
    val committedElapsedMs: Long,
    val recordedSampleCount: Long,
    val storedSampleCount: Long,
)

data class SessionHistoryConfiguration(
    val expectedRidingPersistenceMs: Long,
    val expectedIdlePersistenceMs: Long,
    val retainedResolutionSeconds: Long = 0,
)

data class SessionPruneMetadataRecord(
    val targetResolutionSeconds: Long?,
    val recordedSampleCount: Long,
    val storedSampleCount: Long,
    val policy: String?,
)

data class SessionPrunePreviewRecord(
    val recordedSampleCount: Long,
    val estimatedStoredSampleCount: Long,
    val estimatedReclaimableBytes: Long?,
    val skipped: Boolean,
    val skipReason: String?,
)

data class SessionMaintenanceJobRecord(
    val operationId: String,
    val kind: String,
    val scope: String,
    val state: String,
    val sessionId: String?,
    val targetResolutionSeconds: Long,
    val upperSessionId: String?,
    val lastSessionId: String?,
    val completedSessions: Long,
    val skippedSessions: Long,
    val retainedSamples: Long,
    val reclaimableBytes: Long,
    val error: String?,
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
    val sequence: Long = 0,
    val rideState: String? = null,
)

data class SessionSeriesReductionRecord(
    val columns: Map<String, SessionSeriesColumnRecord>,
    val sourceRows: Int,
    val sourceMs: Double,
    val reducerMs: Double,
    val elapsedMs: Double,
)

data class SessionEventRecord(
    val eventId: String,
    val elapsedMs: Long,
    val kind: String,
    val detailsJson: String,
    val sourceSampleSequence: Long? = null,
)

data class SessionExportRecord(
    val exportId: String,
    val fileName: String,
    val filePath: String,
)

enum class ThemePreference(val wireValue: String) {
    SYSTEM("system"),
    LIGHT("light"),
    DARK("dark");

    companion object {
        fun fromWire(value: String): ThemePreference? = entries.find { it.wireValue == value }
    }
}

enum class UnitPreference(val wireValue: String) {
    METRIC("metric"),
    IMPERIAL("imperial");

    companion object {
        fun fromWire(value: String): UnitPreference? = entries.find { it.wireValue == value }
    }
}

data class SettingsRecord(
    val revision: Int = 0,
    val idleProbeIntervalMs: Long = 500,
    val idlePersistIntervalMs: Long = 1_000,
    val ridingPersistIntervalMs: Long = 100,
    val postRideTailMs: Long = 5_000,
    val reconnectDurationMs: Long = 300_000,
    val minimumSessionDistance: Double = 100.0,
    val themePreference: ThemePreference = ThemePreference.SYSTEM,
    val unitPreference: UnitPreference = UnitPreference.METRIC,
    val historyGapThresholdSeconds: Double? = null,
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
    fun configureSavedDeviceIds(deviceIds: Set<DeviceId>) = Unit
}

interface VescProtocolPort {
    fun reset()
    fun firmwareRequest(route: ControllerRoute): ByteArray
    fun motorConfigurationRequest(route: ControllerRoute): ByteArray =
        throw UnsupportedOperationException("Motor configuration reads are unavailable")
    fun canPingRequest(): ByteArray
    fun refloatInfoRequest(route: ControllerRoute): ByteArray
    fun refloatTelemetryIdsRequest(route: ControllerRoute): ByteArray
    fun refloatTelemetryRequest(route: ControllerRoute): ByteArray
    fun refloatLegacyTelemetryRequest(route: ControllerRoute): ByteArray =
        refloatTelemetryRequest(route)
    fun refloatCompactTelemetryRequest(route: ControllerRoute): ByteArray =
        refloatLegacyTelemetryRequest(route)
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
    fun create(deviceId: DeviceId, advertisedName: String?, at: Instant): SessionRecord
    fun append(jobId: JobId, samples: List<TelemetrySample>)
    fun close(jobId: JobId, at: Instant, reason: String): SessionRecord
    fun active(): SessionRecord?
}

data class SavedDeviceRecord(
    val savedDeviceId: String,
    val deviceHistoryId: String,
    val revision: Int,
    val displayName: String,
    val customDisplayName: String? = null,
    val advertisedName: String? = null,
    val bluetoothIdentifier: String,
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

data class ControllerRoutingHint(
    val savedDeviceId: String,
    val canId: Int,
    /** Private 12-byte controller identity. */
    val controllerUuid: ByteArray?,
) {
    init {
        require(canId in 0..253)
        require(controllerUuid == null || controllerUuid.size == 12)
    }
}

sealed class ProductStoreException(message: String) : RuntimeException(message) {
    data object SavedDeviceNotFound : ProductStoreException("Saved device not found")
    data class SavedDeviceRevisionConflict(val currentRevision: Int) :
        ProductStoreException("Saved device revision conflict")
    data object InvalidSavedDevicePatch : ProductStoreException("Invalid saved device patch")
    data class ProfileRevisionConflict(val currentRevision: Int) :
        ProductStoreException("Profile revision conflict")
    data object InvalidProfilePatch : ProductStoreException("Invalid profile patch")
    data class SettingsRevisionConflict(val currentRevision: Int) :
        ProductStoreException("Settings revision conflict")
    data object InvalidSettingsPatch : ProductStoreException("Invalid settings patch")
}

interface ProductStorePort {
    fun ensureSavedDevice(
        deviceId: DeviceId,
        advertisedName: String?,
        at: Instant,
    ): SavedDeviceRecord = throw ProductStoreException.SavedDeviceNotFound
    fun savedDevices(): List<SavedDeviceRecord>
    fun updateSavedDevice(
        savedDeviceId: String,
        expectedRevision: Int,
        customDisplayName: String?,
        at: Instant,
    ): SavedDeviceRecord = throw ProductStoreException.InvalidSavedDevicePatch
    fun boardProfile(savedDeviceId: String): BoardProfileRecord
    fun updateBoardProfile(
        savedDeviceId: String,
        expectedRevision: Int,
        patchJson: String,
        at: Instant,
    ): BoardProfileRecord
    fun selectController(
        savedDeviceId: String,
        canId: Int,
        controllerUuid: ByteArray?,
        at: Instant,
    ): BoardProfileRecord {
        val current = boardProfile(savedDeviceId)
        return updateBoardProfile(
            savedDeviceId,
            current.revision,
            """{"preferredCanId":$canId}""",
            at,
        )
    }
    fun controllerRoutingHint(savedDeviceId: String): ControllerRoutingHint? = null
    fun storeControllerRoutingHint(hint: ControllerRoutingHint, at: Instant) = Unit
    fun clearControllerRoutingHint(savedDeviceId: String) = Unit
    fun settings(): SettingsRecord = SettingsRecord()
    fun updateSettings(expectedRevision: Int, patchJson: String): SettingsRecord =
        throw ProductStoreException.InvalidSettingsPatch
}

sealed class SessionStoreException(message: String) : RuntimeException(message) {
    data object SessionNotFound : SessionStoreException("Session not found")
    data object SessionActive : SessionStoreException("Active session cannot be changed")
    data object MaintenanceInProgress : SessionStoreException("Session maintenance is in progress")
    data object InvalidCursor : SessionStoreException("Invalid session cursor")
    data object InvalidRange : SessionStoreException("Invalid session range")
    data object InvalidExportFormat : SessionStoreException("Invalid export format")
    data object InvalidArchive : SessionStoreException("Invalid session archive")
    data class UnsupportedField(val field: String) :
        SessionStoreException("Unsupported session field: $field")
}

interface SessionStorePort {
    fun sessions(query: SessionListQuery): SessionPage
    fun session(sessionId: String): SessionSummaryRecord
    fun sessionSnapshot(sessionId: String): SessionPublishedSnapshotRecord =
        throw SessionStoreException.SessionNotFound
    fun sessionHistoryConfiguration(sessionId: String): SessionHistoryConfiguration =
        SessionHistoryConfiguration(1_000, 1_000)
    fun sessionPruneMetadata(sessionId: String): SessionPruneMetadataRecord =
        SessionPruneMetadataRecord(null, 0, 0, null)
    fun cancelHistoricalReads(sessionId: String) = Unit
    fun prunePreview(sessionId: String, targetResolutionSeconds: Long): SessionPrunePreviewRecord =
        throw SessionStoreException.SessionNotFound
    fun pruneSession(sessionId: String, targetResolutionSeconds: Long): SessionPrunePreviewRecord =
        throw SessionStoreException.SessionNotFound
    fun maintenanceJob(): SessionMaintenanceJobRecord? = null
    fun saveMaintenanceJob(job: SessionMaintenanceJobRecord) = Unit
    fun cancelMaintenanceJob(operationId: String): SessionMaintenanceJobRecord? = null
    fun querySeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
    ): List<SessionSeriesPoint>
    fun querySeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
        publishedSampleSequence: Long?,
    ): List<SessionSeriesPoint> = querySeries(
        sessionId, startElapsedMs, endElapsedMs, fields, maxPoints,
    )
    fun querySeriesSource(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxRows: Int,
        publishedSampleSequence: Long?,
    ): List<SessionSeriesPoint> = querySeries(
        sessionId, startElapsedMs, endElapsedMs, fields, maxRows.coerceAtMost(2_000),
        publishedSampleSequence,
    )
    fun reduceSeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        buckets: Int,
        gapPolicy: SessionGapPolicyRecord,
        expectedPersistenceMs: Long,
        expectedRidingPersistenceMs: Long?,
        expectedIdlePersistenceMs: Long?,
        retainedResolutionSeconds: Long,
        publishedSampleSequence: Long?,
    ): SessionSeriesReductionRecord {
        val started = System.nanoTime()
        val rows = querySeriesSource(
            sessionId, startElapsedMs, endElapsedMs, fields, 200_000, publishedSampleSequence,
        )
        val columns = SessionHistoryReducer.reduceSeries(
            rows,
            fields,
            SessionElapsedRange(startElapsedMs, endElapsedMs),
            buckets,
            gapPolicy,
            expectedPersistenceMs,
            expectedRidingPersistenceMs,
            expectedIdlePersistenceMs,
            retainedResolutionSeconds,
            rowsAreOrdered = true,
        )
        val elapsedMs = (System.nanoTime() - started) / 1_000_000.0
        return SessionSeriesReductionRecord(columns, rows.size, 0.0, elapsedMs, elapsedMs)
    }
    fun queryNearestSample(
        sessionId: String,
        targetElapsedMs: Long,
        fields: List<String>,
        publishedSampleSequence: Long?,
    ): SessionSeriesPoint? = querySeriesSource(
        sessionId,
        maxOf(0, targetElapsedMs - 1),
        targetElapsedMs + 1,
        fields,
        2,
        publishedSampleSequence,
    ).minWithOrNull(compareBy<SessionSeriesPoint> {
        kotlin.math.abs(it.elapsedMs - targetElapsedMs)
    }.thenBy { it.elapsedMs }.thenBy { it.sequence })
    fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
    ): List<SessionEventRecord>
    fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        publishedEventSequence: Long?,
    ): List<SessionEventRecord> = sessionEvents(sessionId, startElapsedMs, endElapsedMs)
    fun sessionExtensionFields(sessionId: String): List<String> = emptyList()
    fun querySampleWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
    ): List<SessionSeriesPoint>
    fun querySampleWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
        publishedSampleSequence: Long?,
    ): List<SessionSeriesPoint> = querySampleWindow(
        sessionId, centerElapsedMs, beforeMs, afterMs, fields,
    )
    fun deleteSession(sessionId: String) {
        throw SessionStoreException.SessionNotFound
    }
    fun cancelPendingDelete(operationId: String): Boolean = false
    fun exportSession(sessionId: String, format: String): SessionExportRecord =
        throw SessionStoreException.SessionNotFound
    fun importSessionArchive(data: ByteArray): String =
        throw SessionStoreException.InvalidArchive
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
