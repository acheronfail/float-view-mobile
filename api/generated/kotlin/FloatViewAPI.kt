// Generated from api/schemas. Do not edit directly.
@file:Suppress("unused")

package fail.acheron.floatview.api.v1

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull

@Serializable
enum class BluetoothRadioState {
  @SerialName("unknown") UNKNOWN,
  @SerialName("unsupported") UNSUPPORTED,
  @SerialName("powered_off") POWERED_OFF,
  @SerialName("powered_on") POWERED_ON,
  @SerialName("resetting") RESETTING
}

@Serializable
enum class BluetoothPermissionState {
  @SerialName("unknown") UNKNOWN,
  @SerialName("not_required") NOT_REQUIRED,
  @SerialName("not_determined") NOT_DETERMINED,
  @SerialName("denied") DENIED,
  @SerialName("restricted") RESTRICTED,
  @SerialName("granted") GRANTED
}

@Serializable
data class BluetoothState(
  val radio: BluetoothRadioState,
  val scanPermission: BluetoothPermissionState,
  val connectPermission: BluetoothPermissionState
)

typealias JsonRpcId = JsonElement

@Serializable
data class ApiVersion(
  val major: Int,
  val minor: Int
)

@Serializable
data class ClientInfo(
  val name: String,
  val version: String
)

@Serializable
enum class BackendPlatform {
  @SerialName("mock") MOCK,
  @SerialName("android") ANDROID,
  @SerialName("ios") IOS,
  @SerialName("macos") MACOS,
  @SerialName("linux") LINUX,
  @SerialName("windows") WINDOWS
}

@Serializable
data class BackendInfo(
  val name: String,
  val version: String,
  val platform: BackendPlatform
)

@Serializable
data class Capability(
  val name: String,
  val available: Boolean,
  val constraints: Map<String, JsonElement>? = null
)

@Serializable
enum class ErrorKind {
  @SerialName("invalid_state") INVALID_STATE,
  @SerialName("unsupported") UNSUPPORTED,
  @SerialName("permission_required") PERMISSION_REQUIRED,
  @SerialName("bluetooth_unavailable") BLUETOOTH_UNAVAILABLE,
  @SerialName("scan_failed") SCAN_FAILED,
  @SerialName("device_not_found") DEVICE_NOT_FOUND,
  @SerialName("connection_failed") CONNECTION_FAILED,
  @SerialName("connection_timeout") CONNECTION_TIMEOUT,
  @SerialName("device_not_ready") DEVICE_NOT_READY,
  @SerialName("protocol_error") PROTOCOL_ERROR,
  @SerialName("command_timeout") COMMAND_TIMEOUT,
  @SerialName("rate_limited") RATE_LIMITED,
  @SerialName("storage_unavailable") STORAGE_UNAVAILABLE,
  @SerialName("background_unavailable") BACKGROUND_UNAVAILABLE,
  @SerialName("cancelled") CANCELLED,
  @SerialName("internal") INTERNAL
}

@Serializable
data class RpcErrorData(
  val kind: ErrorKind,
  val recoverable: Boolean,
  val details: Map<String, JsonElement>? = null
)

@Serializable
data class RpcError(
  val code: Int,
  val message: String,
  val data: RpcErrorData
)

@Serializable
data class RpcErrorResponse(
  val jsonrpc: String = "2.0",
  val id: JsonElement,
  val error: RpcError
)

@Serializable
enum class DeviceState {
  @SerialName("discovered") DISCOVERED,
  @SerialName("connecting") CONNECTING,
  @SerialName("connected") CONNECTED,
  @SerialName("discovering_services") DISCOVERING_SERVICES,
  @SerialName("ready") READY,
  @SerialName("disconnecting") DISCONNECTING,
  @SerialName("reconnecting") RECONNECTING,
  @SerialName("disconnected") DISCONNECTED,
  @SerialName("failed") FAILED
}

@Serializable
data class Device(
  val deviceId: String,
  val name: String,
  val state: DeviceState,
  val lastSeenAt: String? = null
)

@Serializable
enum class LoggingJobState {
  @SerialName("starting") STARTING,
  @SerialName("running") RUNNING,
  @SerialName("stopping") STOPPING,
  @SerialName("stopped") STOPPED,
  @SerialName("failed") FAILED
}

@Serializable
data class LoggingJob(
  val jobId: String,
  val deviceId: String,
  val state: LoggingJobState,
  val requestedIntervalMs: Int,
  val effectiveIntervalMs: Int,
  val startedAt: String? = null,
  val stoppedAt: String? = null,
  val sampleCount: Int,
  val errorCount: Int,
  val bytesWritten: Int,
  val backgroundCapable: Boolean? = null,
  val lastError: RpcErrorData? = null
)

typealias EmptyParams = Map<String, JsonElement>

@Serializable
data class AcceptedResult(
  val accepted: Boolean
)

@Serializable
data class SavedDevice(
  val savedDeviceId: String,
  val deviceHistoryId: String,
  val displayName: String,
  val advertisedName: JsonElement? = null,
  val lastSeenAt: JsonElement? = null,
  val lastConnectedAt: JsonElement? = null,
  val profileComplete: Boolean
)

@Serializable
data class BoardProfile(
  val savedDeviceId: String,
  val revision: Int,
  val values: Map<String, JsonElement>,
  val missingFields: List<String>
)

@Serializable
enum class SessionState {
  @SerialName("active") ACTIVE,
  @SerialName("completed") COMPLETED,
  @SerialName("interrupted") INTERRUPTED,
  @SerialName("partial") PARTIAL,
  @SerialName("failed") FAILED
}

@Serializable
data class Session(
  val sessionId: String,
  val deviceHistoryId: String,
  val displayName: String,
  val startedAt: String,
  val endedAt: JsonElement? = null,
  val state: SessionState,
  val integrity: String,
  val sampleCount: Int,
  val eventCount: Int,
  val ridingElapsedMs: Int,
  val totalElapsedMs: Int,
  val distanceM: JsonElement? = null,
  val maxSpeedMps: JsonElement? = null,
  val maxDutyCycle: JsonElement? = null
)

@Serializable
data class SessionGroup(
  val sessionGroupId: String,
  val deviceHistoryId: String,
  val sessionIds: List<String>,
  val totalElapsedMs: Int,
  val distanceM: JsonElement? = null
)

@Serializable
data class Settings(
  val revision: Int,
  val idleProbeIntervalMs: Int,
  val idlePersistIntervalMs: Int,
  val ridingPersistIntervalMs: Int,
  val postRideTailMs: Int,
  val reconnectDurationMs: Int,
  val automaticSessionLinkingEnabled: Boolean,
  val sessionLinkWindowMs: Int,
  val rawPacketCaptureEnabled: Boolean
)

@Serializable
data class SettingsPatch(
  val idleProbeIntervalMs: Int? = null,
  val idlePersistIntervalMs: Int? = null,
  val ridingPersistIntervalMs: Int? = null,
  val postRideTailMs: Int? = null,
  val reconnectDurationMs: Int? = null,
  val automaticSessionLinkingEnabled: Boolean? = null,
  val sessionLinkWindowMs: Int? = null,
  val rawPacketCaptureEnabled: Boolean? = null
)

@Serializable
data class AlertRule(
  val ruleId: String,
  val kind: String,
  val enabled: Boolean,
  val threshold: JsonElement,
  val durationMs: Int? = null,
  val hysteresis: Double? = null,
  val cooldownMs: Int
)

@Serializable
data class StorageState(
  val availableBytes: Int,
  val level: String,
  val warningBytes: Int,
  val criticalBytes: Int,
  val stopBytes: Int
)

@Serializable
data class FirmwareInfo(
  val vescVersion: String,
  val packageName: String,
  val packageVersion: String,
  val hardwareName: JsonElement? = null,
  val supported: Boolean
)

@Serializable
data class SeriesPoint(
  val elapsedMs: Int,
  val values: Map<String, JsonElement>
)

@Serializable
enum class SessionSeriesField {
  @SerialName("packVoltageV") PACK_VOLTAGE_V,
  @SerialName("dutyCycle") DUTY_CYCLE,
  @SerialName("controllerTempC") CONTROLLER_TEMP_C,
  @SerialName("motorTempC") MOTOR_TEMP_C,
  @SerialName("footpadState") FOOTPAD_STATE,
  @SerialName("motorCurrentA") MOTOR_CURRENT_A,
  @SerialName("batteryCurrentA") BATTERY_CURRENT_A,
  @SerialName("erpm") ERPM,
  @SerialName("speedKph") SPEED_KPH
}

@Serializable
data class SessionListFilters(
  val deviceHistoryId: String? = null,
  val startedAfter: String? = null,
  val startedBefore: String? = null,
  val minimumRidingElapsedMs: Int? = null,
  val maximumRidingElapsedMs: Int? = null,
  val minimumTotalElapsedMs: Int? = null,
  val maximumTotalElapsedMs: Int? = null,
  val minimumDistanceM: Double? = null,
  val maximumDistanceM: Double? = null,
  val state: SessionState? = null,
  val integrity: String? = null
)

@Serializable
data class SessionListSort(
  val field: String,
  val direction: String
)

@Serializable
data class SessionEvent(
  val eventId: String,
  val elapsedMs: Int,
  val kind: String,
  val details: Map<String, JsonElement>
)

@Serializable
data class CursorPage(
  val items: List<JsonElement>,
  val nextCursor: JsonElement
)

@Serializable
data class BluetoothGetStateRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "bluetooth.getState",
  val params: EmptyParams
)

@Serializable
data class BluetoothGetStateResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: BluetoothState
)

@Serializable
data class BluetoothRequestPermissionRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "bluetooth.requestPermission",
  val params: EmptyParams
)

@Serializable
data class BluetoothRequestPermissionResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: BluetoothState
)

@Serializable
data class DiscoveryStartRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "discovery.start",
  val params: Map<String, JsonElement>
)

@Serializable
data class DiscoveryStartResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Map<String, JsonElement>
)

@Serializable
data class DiscoveryStopRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "discovery.stop",
  val params: Map<String, JsonElement>
)

@Serializable
data class DiscoveryStopResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: AcceptedResult
)

@Serializable
data class DeviceConnectRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "device.connect",
  val params: Map<String, JsonElement>
)

@Serializable
data class DeviceConnectResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Device
)

@Serializable
data class DeviceDisconnectRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "device.disconnect",
  val params: EmptyParams
)

@Serializable
data class DeviceDisconnectResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: AcceptedResult
)

@Serializable
data class DeviceGetActiveRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "device.getActive",
  val params: EmptyParams
)

@Serializable
data class DeviceGetActiveResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: JsonElement
)

@Serializable
data class SavedDevicesListRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "savedDevices.list",
  val params: EmptyParams
)

@Serializable
data class SavedDevicesListResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: List<SavedDevice>
)

@Serializable
data class SavedDevicesGetRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "savedDevices.get",
  val params: Map<String, JsonElement>
)

@Serializable
data class SavedDevicesGetResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: SavedDevice
)

@Serializable
data class SavedDevicesRenameRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "savedDevices.rename",
  val params: Map<String, JsonElement>
)

@Serializable
data class SavedDevicesRenameResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: SavedDevice
)

@Serializable
data class SavedDevicesForgetRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "savedDevices.forget",
  val params: Map<String, JsonElement>
)

@Serializable
data class SavedDevicesForgetResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: AcceptedResult
)

@Serializable
data class BoardProfilesGetRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "boardProfiles.get",
  val params: Map<String, JsonElement>
)

@Serializable
data class BoardProfilesGetResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: BoardProfile
)

@Serializable
data class BoardProfilesUpdateRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "boardProfiles.update",
  val params: Map<String, JsonElement>
)

@Serializable
data class BoardProfilesUpdateResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: BoardProfile
)

@Serializable
data class SettingsGetRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "settings.get",
  val params: EmptyParams
)

@Serializable
data class SettingsGetResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Settings
)

@Serializable
data class SettingsUpdateRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "settings.update",
  val params: Map<String, JsonElement>
)

@Serializable
data class SettingsUpdateResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Settings
)

@Serializable
data class SessionsListRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.list",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsListResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Map<String, JsonElement>
)

@Serializable
data class SessionsGetRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.get",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsGetResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Session
)

@Serializable
data class SessionsGetSummaryRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.getSummary",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsGetSummaryResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Session
)

@Serializable
data class SessionsQuerySeriesRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.querySeries",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsQuerySeriesResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: List<SeriesPoint>
)

@Serializable
data class SessionsListEventsRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.listEvents",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsListEventsResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: List<SessionEvent>
)

@Serializable
data class SessionsGetDiagnosticWindowRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.getDiagnosticWindow",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsGetDiagnosticWindowResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: List<SeriesPoint>
)

@Serializable
data class SessionsExportRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.export",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsExportResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Map<String, JsonElement>
)

@Serializable
data class SessionsDeleteRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessions.delete",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionsDeleteResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: AcceptedResult
)

@Serializable
data class SessionGroupsListRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessionGroups.list",
  val params: EmptyParams
)

@Serializable
data class SessionGroupsListResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: List<SessionGroup>
)

@Serializable
data class SessionGroupsGetRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessionGroups.get",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionGroupsGetResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: SessionGroup
)

@Serializable
data class SessionGroupsUnlinkRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "sessionGroups.unlink",
  val params: Map<String, JsonElement>
)

@Serializable
data class SessionGroupsUnlinkResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: AcceptedResult
)

@Serializable
data class AlertsListRulesRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "alerts.listRules",
  val params: EmptyParams
)

@Serializable
data class AlertsListRulesResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: List<AlertRule>
)

@Serializable
data class AlertsReplaceRulesRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "alerts.replaceRules",
  val params: Map<String, JsonElement>
)

@Serializable
data class AlertsReplaceRulesResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: List<AlertRule>
)

@Serializable
data class LoggingGetStorageStateRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "logging.getStorageState",
  val params: EmptyParams
)

@Serializable
data class LoggingGetStorageStateResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: StorageState
)

@Serializable
data class VescGetFirmwareInfoRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "vesc.getFirmwareInfo",
  val params: EmptyParams
)

@Serializable
data class VescGetFirmwareInfoResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: FirmwareInfo
)

@Serializable
data class ApiRange(
  val major: Int,
  val minMinor: Int,
  val maxMinor: Int
)

@Serializable
data class Limits(
  val minTelemetryIntervalMs: Int,
  val maxMessageBytes: Int,
  val maxReconnectDurationMs: Int
)

@Serializable
data class Snapshot(
  val revision: Int,
  val bluetooth: BluetoothState,
  val knownDevices: List<Device>,
  val activeConnection: JsonElement,
  val telemetrySubscriptions: List<TelemetrySubscription>,
  val loggingJobs: List<LoggingJob>
)

@Serializable
data class HelloParams(
  val client: ClientInfo,
  val api: ApiRange
)

@Serializable
data class HelloResult(
  val api: ApiVersion,
  val backend: BackendInfo,
  val capabilities: List<Capability>,
  val limits: Limits,
  val snapshot: Snapshot
)

@Serializable
data class HelloRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "system.hello",
  val params: HelloParams
)

@Serializable
data class HelloResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: HelloResult
)

@Serializable
data class GetSnapshotRequest(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val method: String = "system.getSnapshot",
  val params: Map<String, JsonElement>? = null
)

@Serializable
data class GetSnapshotResponse(
  val jsonrpc: String = "2.0",
  val id: JsonRpcId,
  val result: Snapshot
)

@Serializable
enum class TelemetryProfile {
  @SerialName("vesc_realtime") VESC_REALTIME,
  @SerialName("refloat_realtime") REFLOAT_REALTIME
}

@Serializable
data class TelemetrySubscription(
  val subscriptionId: String,
  val deviceId: String,
  val profile: TelemetryProfile,
  val intervalMs: Int,
  val fields: List<String>? = null
)
