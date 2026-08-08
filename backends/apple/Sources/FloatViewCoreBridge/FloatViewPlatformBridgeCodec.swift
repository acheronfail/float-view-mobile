import Foundation

public let floatViewBridgeCodecVersion: UInt8 = 1
public let floatViewPlatformEventKind: UInt32 = 1
public let floatViewMaximumBluetoothBytes = 4096
public let floatViewMaximumDiagnosticBytes = 64

public enum FloatViewBridgeCodecError: Error, Equatable {
  case invalidVersion
  case invalidTag
  case invalidValue
  case invalidUTF8
  case truncated
  case payloadTooLarge
  case commandEnvelopeMismatch
}

public enum FloatViewBluetoothRadio: UInt8, Sendable {
  case unknown = 1
  case poweredOff = 2
  case poweredOn = 3
}

public enum FloatViewBluetoothAuthorization: UInt8, Sendable {
  case unknown = 1
  case denied = 2
  case granted = 3
}

public enum FloatViewBluetoothConnectionState: UInt8, Sendable {
  case connecting = 1
  case ready = 2
  case disconnected = 3
  case failed = 4
}

public enum FloatViewLocationAuthorization: UInt8, Sendable {
  case unknown = 1
  case denied = 2
  case foreground = 3
  case background = 4
}

public enum FloatViewPermissionKind: UInt8, Sendable {
  case bluetooth = 1
  case location = 2
  case backgroundLocation = 3
  case notifications = 4
}

public enum FloatViewThemePreference: UInt8, Sendable {
  case system = 1
  case light = 2
  case dark = 3
}

public enum FloatViewPlatformCommand: Sendable, Equatable {
  case bluetoothStartScan
  case bluetoothStopScan
  case bluetoothConnect(connectionGeneration: UInt64, deviceToken: Data)
  case bluetoothDisconnect(connectionGeneration: UInt64)
  case bluetoothWrite(connectionGeneration: UInt64, bytes: Data)
  case locationStart(background: Bool)
  case locationStop
  case setBackgroundLogging(enabled: Bool)
  case presentPermission(permission: FloatViewPermissionKind)
  case presentSettings
  case runtimeReady(endpoint: String, launchToken: String)

  public var bridgeKind: UInt32 {
    switch self {
    case .bluetoothStartScan: 1
    case .bluetoothStopScan: 2
    case .bluetoothConnect: 3
    case .bluetoothDisconnect: 4
    case .bluetoothWrite: 5
    case .locationStart: 6
    case .locationStop: 7
    case .setBackgroundLogging: 8
    case .presentPermission: 9
    case .presentSettings: 10
    case .runtimeReady: 11
    }
  }
}

public struct FloatViewPlatformCommandEnvelope: Sendable, Equatable {
  public let runtimeGeneration: UInt64
  public let operationID: UInt64
  public let command: FloatViewPlatformCommand

  public init(
    runtimeGeneration: UInt64,
    operationID: UInt64,
    command: FloatViewPlatformCommand
  ) {
    self.runtimeGeneration = runtimeGeneration
    self.operationID = operationID
    self.command = command
  }

  public static func decode(
    _ hostCommand: FloatViewHostCommand
  ) throws -> FloatViewPlatformCommandEnvelope {
    var reader = try BridgeReader(hostCommand.payload)
    let command: FloatViewPlatformCommand = switch hostCommand.kind {
    case 1: .bluetoothStartScan
    case 2: .bluetoothStopScan
    case 3:
      try .bluetoothConnect(
        connectionGeneration: reader.u64(),
        deviceToken: reader.bytes(maximum: 256)
      )
    case 4: try .bluetoothDisconnect(connectionGeneration: reader.u64())
    case 5:
      try .bluetoothWrite(
        connectionGeneration: reader.u64(),
        bytes: reader.bytes(maximum: FloatViewCoreRuntime.maximumPayloadBytes)
      )
    case 6: try .locationStart(background: reader.boolean())
    case 7: .locationStop
    case 8: try .setBackgroundLogging(enabled: reader.boolean())
    case 9: try .presentPermission(permission: tagged(reader.u8()))
    case 10: .presentSettings
    case 11:
      try .runtimeReady(
        endpoint: reader.string(maximum: 256),
        launchToken: reader.string(maximum: 128)
      )
    default: throw FloatViewBridgeCodecError.invalidTag
    }
    try reader.finish()
    return FloatViewPlatformCommandEnvelope(
      runtimeGeneration: hostCommand.runtimeGeneration,
      operationID: hostCommand.operationID,
      command: command
    )
  }
}

public enum FloatViewPlatformEvent: Sendable, Equatable {
  case bluetoothState(
    radio: FloatViewBluetoothRadio,
    authorization: FloatViewBluetoothAuthorization
  )
  case bluetoothDevice(deviceToken: Data, name: String?, rssi: Int16)
  case bluetoothConnection(
    connectionGeneration: UInt64,
    state: FloatViewBluetoothConnectionState
  )
  case locationAuthorization(FloatViewLocationAuthorization)
  case location(latitude: Double, longitude: Double, accuracyM: Double?)
  case lifecycle(foreground: Bool, backgroundEligible: Bool)
  case loggingService(active: Bool)
  case theme(FloatViewThemePreference)
}

private func tagged<T: RawRepresentable>(_ value: UInt8) throws -> T where T.RawValue == UInt8 {
  guard let decoded = T(rawValue: value) else { throw FloatViewBridgeCodecError.invalidTag }
  return decoded
}

private struct BridgeReader {
  let bytes: [UInt8]
  var offset = 0

  init(_ data: Data) throws {
    guard data.count <= FloatViewCoreRuntime.maximumPayloadBytes else {
      throw FloatViewBridgeCodecError.payloadTooLarge
    }
    bytes = Array(data)
  }

  mutating func u8() throws -> UInt8 {
    guard offset < bytes.count else { throw FloatViewBridgeCodecError.truncated }
    defer { offset += 1 }
    return bytes[offset]
  }

  mutating func u32() throws -> UInt32 {
    var value: UInt32 = 0
    for _ in 0 ..< 4 {
      value = try (value << 8) | UInt32(u8())
    }
    return value
  }

  mutating func u64() throws -> UInt64 {
    var value: UInt64 = 0
    for _ in 0 ..< 8 {
      value = try (value << 8) | UInt64(u8())
    }
    return value
  }

  mutating func boolean() throws -> Bool {
    switch try u8() {
    case 0: false
    case 1: true
    default: throw FloatViewBridgeCodecError.invalidValue
    }
  }

  mutating func i16() throws -> Int16 {
    try Int16(bitPattern: UInt16(u16()))
  }

  mutating func u16() throws -> UInt16 {
    var value: UInt16 = 0
    for _ in 0 ..< 2 {
      value = try (value << 8) | UInt16(u8())
    }
    return value
  }

  mutating func f64() throws -> Double {
    try Double(bitPattern: u64())
  }

  mutating func bytes(maximum: Int) throws -> Data {
    let count = try Int(u32())
    guard count > 0, count <= maximum, offset <= bytes.count - count else {
      throw count == 0 ? FloatViewBridgeCodecError.invalidValue
        : FloatViewBridgeCodecError.payloadTooLarge
    }
    defer { offset += count }
    return Data(bytes[offset ..< offset + count])
  }

  mutating func optionalString(maximum: Int) throws -> String? {
    guard try boolean() else { return nil }
    let value = try bytes(maximum: maximum)
    guard let decoded = String(data: value, encoding: .utf8) else {
      throw FloatViewBridgeCodecError.invalidUTF8
    }
    return decoded
  }

  mutating func string(maximum: Int) throws -> String {
    let value = try bytes(maximum: maximum)
    guard let decoded = String(data: value, encoding: .utf8) else {
      throw FloatViewBridgeCodecError.invalidUTF8
    }
    return decoded
  }

  mutating func optionalF64() throws -> Double? {
    try boolean() ? f64() : nil
  }

  mutating func finish() throws {
    guard offset == bytes.count else { throw FloatViewBridgeCodecError.invalidValue }
  }
}
