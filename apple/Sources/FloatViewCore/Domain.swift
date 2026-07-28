import Foundation

public struct DeviceID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) { self.value = value }
}

public struct SessionID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) { self.value = value }
}

public struct JobID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) { self.value = value }
}

public enum ConnectionState: String, Sendable, Codable {
  case disconnected
  case connecting
  case connected
  case discoveringServices
  case enablingNotifications
  case ready
  case reconnecting
  case disconnecting
  case failed
}

public enum RideState: String, Sendable, Codable {
  case idle
  case riding
  case unknown
}

public enum BluetoothRadioState: String, Sendable, Codable {
  case unknown
  case unsupported
  case poweredOff = "powered_off"
  case poweredOn = "powered_on"
  case resetting
}

public enum BluetoothPermissionState: String, Sendable, Codable {
  case unknown
  case notRequired = "not_required"
  case notDetermined = "not_determined"
  case denied
  case restricted
  case granted
}

public struct BluetoothState: Sendable, Codable, Equatable {
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

public struct KnownDevice: Sendable, Codable, Equatable {
  public let deviceID: DeviceID
  public let name: String?
  public let rssi: Int?
  public let lastSeenAt: Date

  public init(deviceID: DeviceID, name: String?, rssi: Int?, lastSeenAt: Date) {
    self.deviceID = deviceID
    self.name = name
    self.rssi = rssi
    self.lastSeenAt = lastSeenAt
  }
}

public struct DiscoverySnapshot: Sendable, Equatable {
  public let bluetoothState: BluetoothState
  public let activeScanID: String?
  public let knownDevices: [KnownDevice]

  public init(
    bluetoothState: BluetoothState,
    activeScanID: String?,
    knownDevices: [KnownDevice]
  ) {
    self.bluetoothState = bluetoothState
    self.activeScanID = activeScanID
    self.knownDevices = knownDevices
  }
}

public struct ActiveConnectionSnapshot: Sendable, Equatable {
  public let deviceID: DeviceID
  public let name: String?
  public let state: ConnectionState

  public init(deviceID: DeviceID, name: String?, state: ConnectionState) {
    self.deviceID = deviceID
    self.name = name
    self.state = state
  }
}

public struct FirmwareRecord: Sendable, Codable, Equatable {
  public let major: UInt8
  public let minor: UInt8
  public let hardwareName: String?
  public let firmwareName: String?
  public let hardwareType: UInt8
  public let canID: UInt8?

  public init(
    major: UInt8,
    minor: UInt8,
    hardwareName: String?,
    firmwareName: String?,
    hardwareType: UInt8 = 0,
    canID: UInt8? = nil
  ) {
    self.major = major
    self.minor = minor
    self.hardwareName = hardwareName
    self.firmwareName = firmwareName
    self.hardwareType = hardwareType
    self.canID = canID
  }
}

public enum VescProtocolMessage: Sendable, Equatable {
  case firmware(FirmwareRecord)
  case canDevices([UInt8])
  case refloatInfo(version: String)
  case refloatTelemetryLayout(fields: [String], runtimeFields: [String])
  case refloatTelemetry(RefloatTelemetryRecord)
}

public struct RefloatTelemetryRecord: Sendable, Equatable {
  public let rideState: RideState
  public let boardState: UInt8
  public let footpadState: UInt8
  public let packVoltageV: Double
  public let dutyCycle: Double
  public let controllerTempC: Double?
  public let motorTempC: Double?
  public let motorCurrentA: Double
  public let batteryCurrentA: Double
  public let erpm: Int
  public let speedKph: Double

  public init(
    rideState: RideState,
    boardState: UInt8,
    footpadState: UInt8,
    packVoltageV: Double,
    dutyCycle: Double,
    controllerTempC: Double?,
    motorTempC: Double?,
    motorCurrentA: Double,
    batteryCurrentA: Double,
    erpm: Int,
    speedKph: Double
  ) {
    self.rideState = rideState
    self.boardState = boardState
    self.footpadState = footpadState
    self.packVoltageV = packVoltageV
    self.dutyCycle = dutyCycle
    self.controllerTempC = controllerTempC
    self.motorTempC = motorTempC
    self.motorCurrentA = motorCurrentA
    self.batteryCurrentA = batteryCurrentA
    self.erpm = erpm
    self.speedKph = speedKph
  }
}

public struct TelemetrySample: Sendable, Codable, Equatable {
  public let sequence: Int64
  public let observedAt: Date
  public let elapsedMs: Int64
  public let rideState: RideState
  public let packVoltageV: Double?
  public let dutyCycle: Double?
  public let controllerTempC: Double?
  public let motorTempC: Double?
  public let footpadState: Int?
  public let extensionFields: [String: Double]

  public init(
    sequence: Int64,
    observedAt: Date,
    elapsedMs: Int64,
    rideState: RideState,
    packVoltageV: Double? = nil,
    dutyCycle: Double? = nil,
    controllerTempC: Double? = nil,
    motorTempC: Double? = nil,
    footpadState: Int? = nil,
    extensionFields: [String: Double] = [:]
  ) {
    self.sequence = sequence
    self.observedAt = observedAt
    self.elapsedMs = elapsedMs
    self.rideState = rideState
    self.packVoltageV = packVoltageV
    self.dutyCycle = dutyCycle
    self.controllerTempC = controllerTempC
    self.motorTempC = motorTempC
    self.footpadState = footpadState
    self.extensionFields = extensionFields
  }
}

public struct SessionRecord: Sendable, Codable, Equatable {
  public let sessionID: SessionID
  public let jobID: JobID
  public let deviceID: DeviceID
  public let startedAt: Date
  public let committedSequence: Int64
  public let active: Bool

  public init(
    sessionID: SessionID,
    jobID: JobID,
    deviceID: DeviceID,
    startedAt: Date,
    committedSequence: Int64,
    active: Bool
  ) {
    self.sessionID = sessionID
    self.jobID = jobID
    self.deviceID = deviceID
    self.startedAt = startedAt
    self.committedSequence = committedSequence
    self.active = active
  }
}

public enum BluetoothEvent: Sendable {
  case stateChanged(BluetoothState)
  case deviceFound(DeviceID, name: String?, rssi: Int?)
  case scanFailed(String)
  case connectionChanged(DeviceID, ConnectionState)
  case notification(DeviceID, Data)
}

public protocol BluetoothPort: AnyObject, Sendable {
  func currentState() -> BluetoothState
  func requestPermission()
  func startDiscovery()
  func stopDiscovery()
  func connect(_ deviceID: DeviceID)
  func write(_ bytes: Data, to deviceID: DeviceID)
  func disconnect(_ deviceID: DeviceID)
}

public protocol VescProtocolPort: AnyObject, Sendable {
  func reset()
  func firmwareRequest(canID: UInt8?) throws -> Data
  func canPingRequest() throws -> Data
  func refloatInfoRequest(canID: UInt8?) throws -> Data
  func refloatTelemetryIDsRequest(canID: UInt8) throws -> Data
  func refloatTelemetryRequest(canID: UInt8) throws -> Data
  func receive(_ bytes: Data) throws -> [VescProtocolMessage]
}

public extension VescProtocolPort {
  func firmwareRequest() throws -> Data { try firmwareRequest(canID: nil) }
  func refloatInfoRequest() throws -> Data { try refloatInfoRequest(canID: nil) }
}

public protocol CancellationPort: AnyObject, Sendable {
  func cancel()
}

public protocol SchedulerPort: Sendable {
  func scheduleRepeating(
    everyMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort
}

public final class SystemScheduler: SchedulerPort, @unchecked Sendable {
  private let queue: DispatchQueue

  public init(
    queue: DispatchQueue = DispatchQueue(label: "fail.acheron.float-view.telemetry")
  ) {
    self.queue = queue
  }

  public func scheduleRepeating(
    everyMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort {
    precondition(everyMilliseconds > 0)
    let source = DispatchSource.makeTimerSource(queue: queue)
    source.schedule(
      deadline: .now() + .milliseconds(Int(everyMilliseconds)),
      repeating: .milliseconds(Int(everyMilliseconds))
    )
    source.setEventHandler(handler: action)
    source.resume()
    return DispatchTimerCancellation(source)
  }
}

private final class DispatchTimerCancellation: CancellationPort, @unchecked Sendable {
  private let lock = NSLock()
  private var source: DispatchSourceTimer?

  init(_ source: DispatchSourceTimer) {
    self.source = source
  }

  func cancel() {
    lock.lock()
    let current = source
    source = nil
    lock.unlock()
    current?.setEventHandler {}
    current?.cancel()
  }

  deinit { cancel() }
}

public protocol ClockPort: Sendable {
  func monotonicMilliseconds() -> Int64
  func now() -> Date
}

public protocol LogStorePort: AnyObject, Sendable {
  func create(deviceID: DeviceID, at: Date) throws -> SessionRecord
  func append(jobID: JobID, samples: [TelemetrySample]) throws
  func close(jobID: JobID, at: Date, reason: String) throws -> SessionRecord
  func active() -> SessionRecord?
}

public protocol EventSink: Sendable {
  func publish(_ event: any Sendable)
}
