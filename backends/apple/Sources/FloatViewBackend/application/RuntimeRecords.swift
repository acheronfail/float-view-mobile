import Foundation

public enum ControllerRoute: Sendable, Equatable {
  case direct
  case can(UInt8)

  public var canID: UInt8? {
    switch self {
    case .direct: nil
    case let .can(canID): canID
    }
  }
}

public enum RefloatBooleanControl: String, Sendable, Codable, CaseIterable {
  case package, flywheel, leds, headlights
}

public enum RefloatConfigurationLimits {
  public static let compressedBytes = 32 * 1024
  public static let fields = 512
  public static let nameBytes = 128
  public static let stringValueBytes = 4096
}

public enum RefloatConfigurationFieldType: Int, Codable, Sendable {
  case double = 1
  case integer = 2
  case string = 3
  case enumeration = 4
  case boolean = 5
  case bitfield = 6
}

public enum RefloatConfigurationWireType: Int, Codable, Sendable {
  case uint8 = 1
  case int8 = 2
  case uint16 = 3
  case int16 = 4
  case uint32 = 5
  case int32 = 6
  case double16 = 7
  case double32 = 8
  case double32Auto = 9
}

public enum RefloatConfigurationValue: Equatable, Sendable, Codable {
  case double(Double)
  case integer(Int64)
  case boolean(Bool)
  case string(String)
}

public struct RefloatConfigurationField: Equatable, Sendable, Codable {
  public let name: String
  public let longName: String
  public let type: RefloatConfigurationFieldType
  public let wireType: RefloatConfigurationWireType?
  public let defaultValue: RefloatConfigurationValue
  public let minimum: Double?
  public let maximum: Double?
  public let step: Double?
  public let precision: Int?
  public let scale: Double?
  public let suffix: String
  public let choices: [String]
  public let description: String
}

public struct RefloatConfigurationSubgroup: Equatable, Sendable, Codable {
  public let name: String
  public let fields: [String]
}

public struct RefloatConfigurationGroup: Equatable, Sendable, Codable {
  public let name: String
  public let subgroups: [RefloatConfigurationSubgroup]
}

public struct RefloatConfigurationSchema: Sendable, Codable, Equatable {
  public let fingerprint: String
  public let wireSignature: UInt32
  public let fields: [RefloatConfigurationField]
  public let groups: [RefloatConfigurationGroup]
}

public struct RefloatConfigurationSchemaIdentity: Equatable, Sendable, Codable {
  public let packageName: String
  public let packageVersion: String
  public let fingerprint: String
  public let wireSignature: UInt32
}

public struct RefloatConfigurationPage: Equatable, Sendable, Codable {
  public let identity: RefloatConfigurationSchemaIdentity
  public let group: String
  public let subgroup: String
  public let fields: [RefloatConfigurationField]
  public let nextCursor: String?
}

public struct RefloatConfigurationValues: Equatable, Sendable, Codable {
  public let revision: UInt64
  public let source: String
  public let values: [String: RefloatConfigurationValue]
}

public enum RefloatConfigurationOperation: String, Sendable, Codable {
  case schema, readCurrent, readDefault, write, applyTune
}

public enum RefloatConfigurationAvailability: String, Sendable, Codable {
  case loading, ready, unavailable
}

/// Host scheduling values passed into the shared runtime. The policy itself is Rust-owned.
public struct SamplingSettings: Equatable, Sendable {
  public var idleProbeIntervalMs: Int64
  public var idlePersistIntervalMs: Int64
  public var ridingPersistIntervalMs: Int64
  public var postRideTailMs: Int64
  public var reconnectDurationMs: Int64

  public init(
    idleProbeIntervalMs: Int64 = 500,
    idlePersistIntervalMs: Int64 = 1_000,
    ridingPersistIntervalMs: Int64 = 100,
    postRideTailMs: Int64 = 5_000,
    reconnectDurationMs: Int64 = 300_000
  ) {
    self.idleProbeIntervalMs = idleProbeIntervalMs
    self.idlePersistIntervalMs = idlePersistIntervalMs
    self.ridingPersistIntervalMs = ridingPersistIntervalMs
    self.postRideTailMs = postRideTailMs
    self.reconnectDurationMs = reconnectDurationMs
  }
}

public enum RefloatControlStatus: String, Sendable, Codable {
  case loading, ready, unavailable
}

public enum RefloatPendingAction: String, Sendable, Codable {
  case package, flywheel, leds, headlights
  case clearFatalError
}

public struct RefloatControls: Sendable, Equatable {
  public var status: RefloatControlStatus
  public var writable: Bool
  public var packageEnabled: Bool?
  public var flywheelEnabled: Bool?
  public var lightingAvailable: Bool
  public var ledsEnabled: Bool?
  public var headlightsEnabled: Bool?
  public var fatalErrorClearAvailable: Bool
  public var fatalErrorLatched: Bool?
  public var fatalErrorActive: Bool?
  public var fatalErrorClearable: Bool?
  public var pendingAction: RefloatPendingAction?

  public init(
    status: RefloatControlStatus = .loading, writable: Bool = false,
    packageEnabled: Bool? = nil, flywheelEnabled: Bool? = nil,
    lightingAvailable: Bool = false, ledsEnabled: Bool? = nil,
    headlightsEnabled: Bool? = nil, fatalErrorClearAvailable: Bool = false,
    fatalErrorLatched: Bool? = nil, fatalErrorActive: Bool? = nil,
    fatalErrorClearable: Bool? = nil, pendingAction: RefloatPendingAction? = nil
  ) {
    self.status = status
    self.writable = writable
    self.packageEnabled = packageEnabled
    self.flywheelEnabled = flywheelEnabled
    self.lightingAvailable = lightingAvailable
    self.ledsEnabled = ledsEnabled
    self.headlightsEnabled = headlightsEnabled
    self.fatalErrorClearAvailable = fatalErrorClearAvailable
    self.fatalErrorLatched = fatalErrorLatched
    self.fatalErrorActive = fatalErrorActive
    self.fatalErrorClearable = fatalErrorClearable
    self.pendingAction = pendingAction
  }
}

public enum RefloatMutationError: Error, Equatable {
  case invalidState
  case unavailable
}

public enum ControllerPendingAction: String, Sendable, Codable {
  case shutdown
}

public struct ControllerActions: Sendable, Equatable {
  public let shutdownAvailable: Bool
  public let pendingAction: ControllerPendingAction?

  public init(shutdownAvailable: Bool = false, pendingAction: ControllerPendingAction? = nil) {
    self.shutdownAvailable = shutdownAvailable
    self.pendingAction = pendingAction
  }
}

public struct RefloatConfigurationSummary: Equatable, Sendable {
  public let availability: RefloatConfigurationAvailability
  public let writable: Bool
  public let packageVersion: String?
  public let schemaFingerprint: String?
  public let wireSignature: UInt32?
  public let currentLoaded: Bool
  public let defaultLoaded: Bool
  public let pendingOperation: RefloatConfigurationOperation?
}

public enum RefloatConfigurationServiceError: String, Error, Sendable {
  case unavailable
  case invalidState
  case staleResponse
  case unauditedSchema
  case malformedResponse
  case conflict
}

public struct RefloatSchemaCacheIdentity: Hashable, Codable, Sendable {
  public let firmwareVersion: String
  public let hardwareName: String
  public let packageVersion: String
  public let customIndex: UInt8
  public let hardwareConfigurationCRC: UInt32
}

public protocol RefloatSchemaCachePort: AnyObject, Sendable {
  func cachedRefloatSchema(identity: RefloatSchemaCacheIdentity) -> Data?
  func storeRefloatSchema(_ data: Data, identity: RefloatSchemaCacheIdentity)
  func removeCachedRefloatSchema(identity: RefloatSchemaCacheIdentity)
}

public extension RefloatSchemaCachePort {
  func cachedRefloatSchema(identity _: RefloatSchemaCacheIdentity) -> Data? {
    nil
  }

  func storeRefloatSchema(_: Data, identity _: RefloatSchemaCacheIdentity) {}
  func removeCachedRefloatSchema(identity _: RefloatSchemaCacheIdentity) {}
}
