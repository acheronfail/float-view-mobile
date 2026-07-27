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
  public let maxConnections: Int
  public let minTelemetryIntervalMs: Int
  public let maxMessageBytes: Int

  public init(
    maxConnections: Int,
    minTelemetryIntervalMs: Int,
    maxMessageBytes: Int
  ) {
    self.maxConnections = maxConnections
    self.minTelemetryIntervalMs = minTelemetryIntervalMs
    self.maxMessageBytes = maxMessageBytes
  }
}

public struct Snapshot: Codable, Sendable, Equatable {
  public let revision: Int
  public let bluetooth: BluetoothState
  public let devices: [Device]
  public let telemetrySubscriptions: [TelemetrySubscription]
  public let loggingJobs: [LoggingJob]

  public init(
    revision: Int,
    bluetooth: BluetoothState,
    devices: [Device],
    telemetrySubscriptions: [TelemetrySubscription],
    loggingJobs: [LoggingJob]
  ) {
    self.revision = revision
    self.bluetooth = bluetooth
    self.devices = devices
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
