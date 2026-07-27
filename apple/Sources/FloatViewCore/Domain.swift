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

  public init(
    sequence: Int64,
    observedAt: Date,
    elapsedMs: Int64,
    rideState: RideState,
    packVoltageV: Double? = nil,
    dutyCycle: Double? = nil,
    controllerTempC: Double? = nil,
    motorTempC: Double? = nil,
    footpadState: Int? = nil
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
  case powered(Bool)
  case deviceFound(DeviceID, name: String?, rssi: Int?)
  case connectionChanged(DeviceID, ConnectionState)
  case notification(DeviceID, Data)
}

public protocol BluetoothPort: AnyObject, Sendable {
  func startDiscovery()
  func stopDiscovery()
  func connect(_ deviceID: DeviceID)
  func write(_ bytes: Data, to deviceID: DeviceID)
  func disconnect(_ deviceID: DeviceID)
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
