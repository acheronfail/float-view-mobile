// Generated from api/schemas. Do not edit directly.
import Foundation

public enum JSONValue: Codable, Sendable, Equatable {
  case string(String)
  case integer(Int)
  case number(Double)
  case boolean(Bool)
  case object([String: JSONValue])
  case array([JSONValue])
  case null

  public init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()
    if container.decodeNil() { self = .null }
    else if let value = try? container.decode(Bool.self) { self = .boolean(value) }
    else if let value = try? container.decode(Int.self) { self = .integer(value) }
    else if let value = try? container.decode(Double.self) { self = .number(value) }
    else if let value = try? container.decode(String.self) { self = .string(value) }
    else if let value = try? container.decode([JSONValue].self) { self = .array(value) }
    else if let value = try? container.decode([String: JSONValue].self) { self = .object(value) }
    else {
      throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unsupported JSON value")
    }
  }

  public func encode(to encoder: Encoder) throws {
    var container = encoder.singleValueContainer()
    switch self {
    case .string(let value): try container.encode(value)
    case .integer(let value): try container.encode(value)
    case .number(let value): try container.encode(value)
    case .boolean(let value): try container.encode(value)
    case .object(let value): try container.encode(value)
    case .array(let value): try container.encode(value)
    case .null: try container.encodeNil()
    }
  }
}

public enum BluetoothRadioState: String, Codable, Sendable, Equatable {
  case unknown = "unknown"
  case unsupported = "unsupported"
  case poweredOff = "powered_off"
  case poweredOn = "powered_on"
  case resetting = "resetting"
}

public enum BluetoothPermissionState: String, Codable, Sendable, Equatable {
  case unknown = "unknown"
  case notRequired = "not_required"
  case notDetermined = "not_determined"
  case denied = "denied"
  case restricted = "restricted"
  case granted = "granted"
}

public struct BluetoothState: Codable, Sendable, Equatable {
  public let radio: BluetoothRadioState
  public let scanPermission: BluetoothPermissionState
  public let connectPermission: BluetoothPermissionState

  public init(
    radio: BluetoothRadioState,
    scanPermission: BluetoothPermissionState,
    connectPermission: BluetoothPermissionState
  ) {
    self.radio = radio
    self.scanPermission = scanPermission
    self.connectPermission = connectPermission
  }
}

public typealias JsonRpcId = JSONValue

public struct ApiVersion: Codable, Sendable, Equatable {
  public let major: Int
  public let minor: Int

  public init(
    major: Int,
    minor: Int
  ) {
    self.major = major
    self.minor = minor
  }
}

public struct ClientInfo: Codable, Sendable, Equatable {
  public let name: String
  public let version: String

  public init(
    name: String,
    version: String
  ) {
    self.name = name
    self.version = version
  }
}

public enum BackendPlatform: String, Codable, Sendable, Equatable {
  case mock = "mock"
  case android = "android"
  case ios = "ios"
  case macos = "macos"
  case linux = "linux"
  case windows = "windows"
}

public struct BackendInfo: Codable, Sendable, Equatable {
  public let name: String
  public let version: String
  public let platform: BackendPlatform

  public init(
    name: String,
    version: String,
    platform: BackendPlatform
  ) {
    self.name = name
    self.version = version
    self.platform = platform
  }
}

public struct Capability: Codable, Sendable, Equatable {
  public let name: String
  public let available: Bool
  public let constraints: [String: JSONValue]?

  public init(
    name: String,
    available: Bool,
    constraints: [String: JSONValue]? = nil
  ) {
    self.name = name
    self.available = available
    self.constraints = constraints
  }
}

public enum ErrorKind: String, Codable, Sendable, Equatable {
  case invalidState = "invalid_state"
  case unsupported = "unsupported"
  case permissionRequired = "permission_required"
  case bluetoothUnavailable = "bluetooth_unavailable"
  case scanFailed = "scan_failed"
  case deviceNotFound = "device_not_found"
  case connectionFailed = "connection_failed"
  case connectionTimeout = "connection_timeout"
  case deviceNotReady = "device_not_ready"
  case protocolError = "protocol_error"
  case commandTimeout = "command_timeout"
  case rateLimited = "rate_limited"
  case storageUnavailable = "storage_unavailable"
  case backgroundUnavailable = "background_unavailable"
  case cancelled = "cancelled"
  case `internal` = "internal"
}

public struct RpcErrorData: Codable, Sendable, Equatable {
  public let kind: ErrorKind
  public let recoverable: Bool
  public let details: [String: JSONValue]?

  public init(
    kind: ErrorKind,
    recoverable: Bool,
    details: [String: JSONValue]? = nil
  ) {
    self.kind = kind
    self.recoverable = recoverable
    self.details = details
  }
}

public struct RpcError: Codable, Sendable, Equatable {
  public let code: Int
  public let message: String
  public let data: RpcErrorData

  public init(
    code: Int,
    message: String,
    data: RpcErrorData
  ) {
    self.code = code
    self.message = message
    self.data = data
  }
}

public struct RpcErrorResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JSONValue
  public let error: RpcError

  public init(
    jsonrpc: String = "2.0",
    id: JSONValue,
    error: RpcError
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.error = error
  }
}

public enum DeviceState: String, Codable, Sendable, Equatable {
  case discovered = "discovered"
  case connecting = "connecting"
  case connected = "connected"
  case discoveringServices = "discovering_services"
  case ready = "ready"
  case disconnecting = "disconnecting"
  case reconnecting = "reconnecting"
  case disconnected = "disconnected"
  case failed = "failed"
}

public struct Device: Codable, Sendable, Equatable {
  /// Opaque backend-issued identifier; never interpret as a MAC address or UUID.
  public let deviceId: String
  public let name: String
  public let state: DeviceState
  public let lastSeenAt: String?

  public init(
    deviceId: String,
    name: String,
    state: DeviceState,
    lastSeenAt: String? = nil
  ) {
    self.deviceId = deviceId
    self.name = name
    self.state = state
    self.lastSeenAt = lastSeenAt
  }
}

public enum LoggingJobState: String, Codable, Sendable, Equatable {
  case starting = "starting"
  case running = "running"
  case stopping = "stopping"
  case stopped = "stopped"
  case failed = "failed"
}

public struct LoggingJob: Codable, Sendable, Equatable {
  public let jobId: String
  public let deviceId: String
  public let state: LoggingJobState
  public let requestedIntervalMs: Int
  public let effectiveIntervalMs: Int
  public let startedAt: String?
  public let stoppedAt: String?
  public let sampleCount: Int
  public let errorCount: Int
  public let bytesWritten: Int
  public let backgroundCapable: Bool?
  public let lastError: RpcErrorData?

  public init(
    jobId: String,
    deviceId: String,
    state: LoggingJobState,
    requestedIntervalMs: Int,
    effectiveIntervalMs: Int,
    startedAt: String? = nil,
    stoppedAt: String? = nil,
    sampleCount: Int,
    errorCount: Int,
    bytesWritten: Int,
    backgroundCapable: Bool? = nil,
    lastError: RpcErrorData? = nil
  ) {
    self.jobId = jobId
    self.deviceId = deviceId
    self.state = state
    self.requestedIntervalMs = requestedIntervalMs
    self.effectiveIntervalMs = effectiveIntervalMs
    self.startedAt = startedAt
    self.stoppedAt = stoppedAt
    self.sampleCount = sampleCount
    self.errorCount = errorCount
    self.bytesWritten = bytesWritten
    self.backgroundCapable = backgroundCapable
    self.lastError = lastError
  }
}

public typealias EmptyParams = [String: JSONValue]

public struct AcceptedResult: Codable, Sendable, Equatable {
  public let accepted: Bool

  public init(
    accepted: Bool
  ) {
    self.accepted = accepted
  }
}

public struct SavedDevice: Codable, Sendable, Equatable {
  public let savedDeviceId: String
  public let deviceHistoryId: String
  public let displayName: String
  public let advertisedName: JSONValue?
  public let lastSeenAt: JSONValue?
  public let lastConnectedAt: JSONValue?
  public let profileComplete: Bool

  public init(
    savedDeviceId: String,
    deviceHistoryId: String,
    displayName: String,
    advertisedName: JSONValue? = nil,
    lastSeenAt: JSONValue? = nil,
    lastConnectedAt: JSONValue? = nil,
    profileComplete: Bool
  ) {
    self.savedDeviceId = savedDeviceId
    self.deviceHistoryId = deviceHistoryId
    self.displayName = displayName
    self.advertisedName = advertisedName
    self.lastSeenAt = lastSeenAt
    self.lastConnectedAt = lastConnectedAt
    self.profileComplete = profileComplete
  }
}

public struct BoardProfile: Codable, Sendable, Equatable {
  public let savedDeviceId: String
  public let revision: Int
  public let values: [String: JSONValue]
  public let missingFields: [String]

  public init(
    savedDeviceId: String,
    revision: Int,
    values: [String: JSONValue],
    missingFields: [String]
  ) {
    self.savedDeviceId = savedDeviceId
    self.revision = revision
    self.values = values
    self.missingFields = missingFields
  }
}

public enum SessionState: String, Codable, Sendable, Equatable {
  case active = "active"
  case completed = "completed"
  case interrupted = "interrupted"
  case partial = "partial"
  case failed = "failed"
}

public struct Session: Codable, Sendable, Equatable {
  public let sessionId: String
  public let deviceHistoryId: String
  public let displayName: String
  public let startedAt: String
  public let endedAt: JSONValue?
  public let state: SessionState
  public let integrity: String
  public let sampleCount: Int
  public let eventCount: Int
  public let ridingElapsedMs: Int
  public let totalElapsedMs: Int
  public let distanceM: JSONValue?
  public let maxSpeedMps: JSONValue?
  public let maxDutyCycle: JSONValue?

  public init(
    sessionId: String,
    deviceHistoryId: String,
    displayName: String,
    startedAt: String,
    endedAt: JSONValue? = nil,
    state: SessionState,
    integrity: String,
    sampleCount: Int,
    eventCount: Int,
    ridingElapsedMs: Int,
    totalElapsedMs: Int,
    distanceM: JSONValue? = nil,
    maxSpeedMps: JSONValue? = nil,
    maxDutyCycle: JSONValue? = nil
  ) {
    self.sessionId = sessionId
    self.deviceHistoryId = deviceHistoryId
    self.displayName = displayName
    self.startedAt = startedAt
    self.endedAt = endedAt
    self.state = state
    self.integrity = integrity
    self.sampleCount = sampleCount
    self.eventCount = eventCount
    self.ridingElapsedMs = ridingElapsedMs
    self.totalElapsedMs = totalElapsedMs
    self.distanceM = distanceM
    self.maxSpeedMps = maxSpeedMps
    self.maxDutyCycle = maxDutyCycle
  }
}

public struct SessionGroup: Codable, Sendable, Equatable {
  public let sessionGroupId: String
  public let deviceHistoryId: String
  public let sessionIds: [String]
  public let totalElapsedMs: Int
  public let distanceM: JSONValue?

  public init(
    sessionGroupId: String,
    deviceHistoryId: String,
    sessionIds: [String],
    totalElapsedMs: Int,
    distanceM: JSONValue? = nil
  ) {
    self.sessionGroupId = sessionGroupId
    self.deviceHistoryId = deviceHistoryId
    self.sessionIds = sessionIds
    self.totalElapsedMs = totalElapsedMs
    self.distanceM = distanceM
  }
}

public struct Settings: Codable, Sendable, Equatable {
  public let revision: Int
  public let idleProbeIntervalMs: Int
  public let idlePersistIntervalMs: Int
  public let ridingPersistIntervalMs: Int
  public let postRideTailMs: Int
  public let reconnectDurationMs: Int
  public let automaticSessionLinkingEnabled: Bool
  public let sessionLinkWindowMs: Int
  public let rawPacketCaptureEnabled: Bool

  public init(
    revision: Int,
    idleProbeIntervalMs: Int,
    idlePersistIntervalMs: Int,
    ridingPersistIntervalMs: Int,
    postRideTailMs: Int,
    reconnectDurationMs: Int,
    automaticSessionLinkingEnabled: Bool,
    sessionLinkWindowMs: Int,
    rawPacketCaptureEnabled: Bool
  ) {
    self.revision = revision
    self.idleProbeIntervalMs = idleProbeIntervalMs
    self.idlePersistIntervalMs = idlePersistIntervalMs
    self.ridingPersistIntervalMs = ridingPersistIntervalMs
    self.postRideTailMs = postRideTailMs
    self.reconnectDurationMs = reconnectDurationMs
    self.automaticSessionLinkingEnabled = automaticSessionLinkingEnabled
    self.sessionLinkWindowMs = sessionLinkWindowMs
    self.rawPacketCaptureEnabled = rawPacketCaptureEnabled
  }
}

public struct AlertRule: Codable, Sendable, Equatable {
  public let ruleId: String
  public let kind: String
  public let enabled: Bool
  public let threshold: JSONValue
  public let durationMs: Int?
  public let hysteresis: Double?
  public let cooldownMs: Int

  public init(
    ruleId: String,
    kind: String,
    enabled: Bool,
    threshold: JSONValue,
    durationMs: Int? = nil,
    hysteresis: Double? = nil,
    cooldownMs: Int
  ) {
    self.ruleId = ruleId
    self.kind = kind
    self.enabled = enabled
    self.threshold = threshold
    self.durationMs = durationMs
    self.hysteresis = hysteresis
    self.cooldownMs = cooldownMs
  }
}

public struct StorageState: Codable, Sendable, Equatable {
  public let availableBytes: Int
  public let level: String
  public let warningBytes: Int
  public let criticalBytes: Int
  public let stopBytes: Int

  public init(
    availableBytes: Int,
    level: String,
    warningBytes: Int,
    criticalBytes: Int,
    stopBytes: Int
  ) {
    self.availableBytes = availableBytes
    self.level = level
    self.warningBytes = warningBytes
    self.criticalBytes = criticalBytes
    self.stopBytes = stopBytes
  }
}

public struct FirmwareInfo: Codable, Sendable, Equatable {
  public let vescVersion: String
  public let packageName: String
  public let packageVersion: String
  public let hardwareName: JSONValue?
  public let supported: Bool

  public init(
    vescVersion: String,
    packageName: String,
    packageVersion: String,
    hardwareName: JSONValue? = nil,
    supported: Bool
  ) {
    self.vescVersion = vescVersion
    self.packageName = packageName
    self.packageVersion = packageVersion
    self.hardwareName = hardwareName
    self.supported = supported
  }
}

public struct SeriesPoint: Codable, Sendable, Equatable {
  public let elapsedMs: Int
  public let values: [String: JSONValue]

  public init(
    elapsedMs: Int,
    values: [String: JSONValue]
  ) {
    self.elapsedMs = elapsedMs
    self.values = values
  }
}

public enum SessionSeriesField: String, Codable, Sendable, Equatable {
  case packVoltageV = "packVoltageV"
  case dutyCycle = "dutyCycle"
  case controllerTempC = "controllerTempC"
  case motorTempC = "motorTempC"
  case footpadState = "footpadState"
  case motorCurrentA = "motorCurrentA"
  case batteryCurrentA = "batteryCurrentA"
  case erpm = "erpm"
  case speedKph = "speedKph"
}

public struct SessionListFilters: Codable, Sendable, Equatable {
  public let deviceHistoryId: String?
  public let startedAfter: String?
  public let startedBefore: String?
  public let minimumRidingElapsedMs: Int?
  public let maximumRidingElapsedMs: Int?
  public let minimumTotalElapsedMs: Int?
  public let maximumTotalElapsedMs: Int?
  public let minimumDistanceM: Double?
  public let maximumDistanceM: Double?
  public let state: SessionState?
  public let integrity: String?

  public init(
    deviceHistoryId: String? = nil,
    startedAfter: String? = nil,
    startedBefore: String? = nil,
    minimumRidingElapsedMs: Int? = nil,
    maximumRidingElapsedMs: Int? = nil,
    minimumTotalElapsedMs: Int? = nil,
    maximumTotalElapsedMs: Int? = nil,
    minimumDistanceM: Double? = nil,
    maximumDistanceM: Double? = nil,
    state: SessionState? = nil,
    integrity: String? = nil
  ) {
    self.deviceHistoryId = deviceHistoryId
    self.startedAfter = startedAfter
    self.startedBefore = startedBefore
    self.minimumRidingElapsedMs = minimumRidingElapsedMs
    self.maximumRidingElapsedMs = maximumRidingElapsedMs
    self.minimumTotalElapsedMs = minimumTotalElapsedMs
    self.maximumTotalElapsedMs = maximumTotalElapsedMs
    self.minimumDistanceM = minimumDistanceM
    self.maximumDistanceM = maximumDistanceM
    self.state = state
    self.integrity = integrity
  }
}

public struct SessionListSort: Codable, Sendable, Equatable {
  public let field: String
  public let direction: String

  public init(
    field: String,
    direction: String
  ) {
    self.field = field
    self.direction = direction
  }
}

public struct SessionEvent: Codable, Sendable, Equatable {
  public let eventId: String
  public let elapsedMs: Int
  public let kind: String
  public let details: [String: JSONValue]

  public init(
    eventId: String,
    elapsedMs: Int,
    kind: String,
    details: [String: JSONValue]
  ) {
    self.eventId = eventId
    self.elapsedMs = elapsedMs
    self.kind = kind
    self.details = details
  }
}

public struct CursorPage: Codable, Sendable, Equatable {
  public let items: [JSONValue]
  public let nextCursor: JSONValue

  public init(
    items: [JSONValue],
    nextCursor: JSONValue
  ) {
    self.items = items
    self.nextCursor = nextCursor
  }
}

public struct BluetoothGetStateRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "bluetooth.getState",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct BluetoothGetStateResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: BluetoothState

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: BluetoothState
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct BluetoothRequestPermissionRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "bluetooth.requestPermission",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct BluetoothRequestPermissionResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: BluetoothState

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: BluetoothState
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct DiscoveryStartRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "discovery.start",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct DiscoveryStartResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct DiscoveryStopRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "discovery.stop",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct DiscoveryStopResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: AcceptedResult

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: AcceptedResult
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct DeviceConnectRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "device.connect",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct DeviceConnectResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: Device

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: Device
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct DeviceDisconnectRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "device.disconnect",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct DeviceDisconnectResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: AcceptedResult

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: AcceptedResult
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct DeviceGetActiveRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "device.getActive",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct DeviceGetActiveResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: JSONValue

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: JSONValue
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SavedDevicesListRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "savedDevices.list",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SavedDevicesListResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [SavedDevice]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [SavedDevice]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SavedDevicesGetRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "savedDevices.get",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SavedDevicesGetResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: SavedDevice

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: SavedDevice
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SavedDevicesRenameRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "savedDevices.rename",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SavedDevicesRenameResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: SavedDevice

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: SavedDevice
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SavedDevicesForgetRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "savedDevices.forget",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SavedDevicesForgetResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: AcceptedResult

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: AcceptedResult
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct BoardProfilesGetRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "boardProfiles.get",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct BoardProfilesGetResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: BoardProfile

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: BoardProfile
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct BoardProfilesUpdateRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "boardProfiles.update",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct BoardProfilesUpdateResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: BoardProfile

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: BoardProfile
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SettingsGetRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "settings.get",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SettingsGetResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: Settings

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: Settings
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SettingsUpdateRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "settings.update",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SettingsUpdateResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: Settings

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: Settings
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsListRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.list",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsListResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsGetRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.get",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsGetResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: Session

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: Session
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsGetSummaryRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.getSummary",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsGetSummaryResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: Session

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: Session
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsQuerySeriesRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.querySeries",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsQuerySeriesResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [SeriesPoint]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [SeriesPoint]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsListEventsRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.listEvents",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsListEventsResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [SessionEvent]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [SessionEvent]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsGetDiagnosticWindowRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.getDiagnosticWindow",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsGetDiagnosticWindowResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [SeriesPoint]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [SeriesPoint]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsExportRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.export",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsExportResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionsDeleteRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessions.delete",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionsDeleteResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: AcceptedResult

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: AcceptedResult
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct SessionGroupsListRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "sessionGroups.list",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct SessionGroupsListResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [SessionGroup]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [SessionGroup]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct AlertsListRulesRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "alerts.listRules",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct AlertsListRulesResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [AlertRule]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [AlertRule]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct AlertsReplaceRulesRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "alerts.replaceRules",
    params: [String: JSONValue]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct AlertsReplaceRulesResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: [AlertRule]

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: [AlertRule]
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct LoggingGetStorageStateRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "logging.getStorageState",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct LoggingGetStorageStateResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: StorageState

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: StorageState
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct VescGetFirmwareInfoRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: EmptyParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "vesc.getFirmwareInfo",
    params: EmptyParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct VescGetFirmwareInfoResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: FirmwareInfo

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: FirmwareInfo
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct ApiRange: Codable, Sendable, Equatable {
  public let major: Int
  public let minMinor: Int
  public let maxMinor: Int

  public init(
    major: Int,
    minMinor: Int,
    maxMinor: Int
  ) {
    self.major = major
    self.minMinor = minMinor
    self.maxMinor = maxMinor
  }
}

public struct Limits: Codable, Sendable, Equatable {
  public let minTelemetryIntervalMs: Int
  public let maxMessageBytes: Int
  public let maxReconnectDurationMs: Int

  public init(
    minTelemetryIntervalMs: Int,
    maxMessageBytes: Int,
    maxReconnectDurationMs: Int
  ) {
    self.minTelemetryIntervalMs = minTelemetryIntervalMs
    self.maxMessageBytes = maxMessageBytes
    self.maxReconnectDurationMs = maxReconnectDurationMs
  }
}

public struct Snapshot: Codable, Sendable, Equatable {
  public let revision: Int
  public let bluetooth: BluetoothState
  public let knownDevices: [Device]
  public let activeConnection: JSONValue
  public let telemetrySubscriptions: [TelemetrySubscription]
  public let loggingJobs: [LoggingJob]

  public init(
    revision: Int,
    bluetooth: BluetoothState,
    knownDevices: [Device],
    activeConnection: JSONValue,
    telemetrySubscriptions: [TelemetrySubscription],
    loggingJobs: [LoggingJob]
  ) {
    self.revision = revision
    self.bluetooth = bluetooth
    self.knownDevices = knownDevices
    self.activeConnection = activeConnection
    self.telemetrySubscriptions = telemetrySubscriptions
    self.loggingJobs = loggingJobs
  }
}

public struct HelloParams: Codable, Sendable, Equatable {
  public let client: ClientInfo
  public let api: ApiRange

  public init(
    client: ClientInfo,
    api: ApiRange
  ) {
    self.client = client
    self.api = api
  }
}

public struct HelloResult: Codable, Sendable, Equatable {
  public let api: ApiVersion
  public let backend: BackendInfo
  public let capabilities: [Capability]
  public let limits: Limits
  public let snapshot: Snapshot

  public init(
    api: ApiVersion,
    backend: BackendInfo,
    capabilities: [Capability],
    limits: Limits,
    snapshot: Snapshot
  ) {
    self.api = api
    self.backend = backend
    self.capabilities = capabilities
    self.limits = limits
    self.snapshot = snapshot
  }
}

public struct HelloRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: HelloParams

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "system.hello",
    params: HelloParams
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct HelloResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: HelloResult

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: HelloResult
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public struct GetSnapshotRequest: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let method: String
  public let params: [String: JSONValue]?

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    method: String = "system.getSnapshot",
    params: [String: JSONValue]? = nil
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.method = method
    self.params = params
  }
}

public struct GetSnapshotResponse: Codable, Sendable, Equatable {
  public let jsonrpc: String
  public let id: JsonRpcId
  public let result: Snapshot

  public init(
    jsonrpc: String = "2.0",
    id: JsonRpcId,
    result: Snapshot
  ) {
    self.jsonrpc = jsonrpc
    self.id = id
    self.result = result
  }
}

public enum TelemetryProfile: String, Codable, Sendable, Equatable {
  case vescRealtime = "vesc_realtime"
  case refloatRealtime = "refloat_realtime"
}

public struct TelemetrySubscription: Codable, Sendable, Equatable {
  public let subscriptionId: String
  public let deviceId: String
  public let profile: TelemetryProfile
  public let intervalMs: Int
  public let fields: [String]?

  public init(
    subscriptionId: String,
    deviceId: String,
    profile: TelemetryProfile,
    intervalMs: Int,
    fields: [String]? = nil
  ) {
    self.subscriptionId = subscriptionId
    self.deviceId = deviceId
    self.profile = profile
    self.intervalMs = intervalMs
    self.fields = fields
  }
}
