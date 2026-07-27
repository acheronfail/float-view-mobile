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

@Serializable
data class ApiRange(
  val major: Int,
  val minMinor: Int,
  val maxMinor: Int
)

@Serializable
data class Limits(
  val maxConnections: Int,
  val minTelemetryIntervalMs: Int,
  val maxMessageBytes: Int
)

@Serializable
data class Snapshot(
  val revision: Int,
  val bluetooth: BluetoothState,
  val devices: List<Device>,
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
