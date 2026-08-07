import Foundation

public let floatViewBridgeCodecVersion: UInt8 = 1
public let floatViewPlatformCommandKind: UInt32 = 1
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

public enum FloatViewResourceNamespace: UInt8, Sendable, Hashable {
  case frontend = 1
  case sql = 2
  case softwareFixture = 3
}

public enum FloatViewStorageRootKind: UInt8, Sendable, Hashable {
  case durable = 1
  case cache = 2
  case temporary = 3
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
  case bluetoothConnect(deviceToken: Data)
  case bluetoothDisconnect(connectionGeneration: UInt64)
  case bluetoothWrite(connectionGeneration: UInt64, bytes: Data)
  case locationStart(background: Bool)
  case locationStop
  case setBackgroundLogging(enabled: Bool)
  case readResource(
    namespace: FloatViewResourceNamespace,
    logicalName: String,
    maximumBytes: UInt64
  )
  case stageDocument(destination: String, maximumBytes: UInt64)
  case resolveStorageRoot(kind: FloatViewStorageRootKind)
  case availableSpace(root: String)
  case monotonicClock
  case wallClock
  case secureRandom(count: UInt16)
  case diagnostic(stage: String, code: String)
  case presentPermission(permission: FloatViewPermissionKind)
  case presentSettings
  case presentTheme(theme: FloatViewThemePreference)
  case presentServiceState(active: Bool)
  case presentExport(stagedPath: String)
  case runtimeReady(endpoint: String, launchToken: String)
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

  public func encode() throws -> Data {
    var writer = BridgeWriter()
    try writer.u8(floatViewBridgeCodecVersion)
    try writer.u64(runtimeGeneration)
    try writer.u64(operationID)
    switch command {
    case .bluetoothStartScan:
      try writer.u8(1)
    case .bluetoothStopScan:
      try writer.u8(2)
    case let .bluetoothConnect(deviceToken):
      guard !deviceToken.isEmpty, deviceToken.count <= 256 else {
        throw FloatViewBridgeCodecError.payloadTooLarge
      }
      try writer.u8(3)
      try writer.bytes(deviceToken)
    case let .bluetoothDisconnect(connectionGeneration):
      try writer.u8(4)
      try writer.u64(connectionGeneration)
    case let .bluetoothWrite(connectionGeneration, bytes):
      guard !bytes.isEmpty, bytes.count <= FloatViewCoreRuntime.maximumPayloadBytes else {
        throw FloatViewBridgeCodecError.payloadTooLarge
      }
      try writer.u8(5)
      try writer.u64(connectionGeneration)
      try writer.bytes(bytes)
    case let .locationStart(background):
      try writer.u8(6)
      try writer.boolean(background)
    case .locationStop:
      try writer.u8(7)
    case let .setBackgroundLogging(enabled):
      try writer.u8(8)
      try writer.boolean(enabled)
    case let .readResource(namespace, logicalName, maximumBytes):
      try writer.u8(9)
      try writer.u8(namespace.rawValue)
      try writer.string(logicalName, maximum: 128)
      try writer.u64(maximumBytes)
    case let .stageDocument(destination, maximumBytes):
      try writer.u8(10)
      try writer.string(destination, maximum: 1024)
      try writer.u64(maximumBytes)
    case let .resolveStorageRoot(kind):
      try writer.u8(11)
      try writer.u8(kind.rawValue)
    case let .availableSpace(root):
      try writer.u8(12)
      try writer.string(root, maximum: 1024)
    case .monotonicClock:
      try writer.u8(13)
    case .wallClock:
      try writer.u8(14)
    case let .secureRandom(count):
      try writer.u8(15)
      try writer.u16(count)
    case let .diagnostic(stage, code):
      try writer.u8(16)
      try writer.string(stage, maximum: floatViewMaximumDiagnosticBytes)
      try writer.string(code, maximum: floatViewMaximumDiagnosticBytes)
    case let .presentPermission(permission):
      try writer.u8(17)
      try writer.u8(permission.rawValue)
    case .presentSettings:
      try writer.u8(18)
    case let .presentTheme(theme):
      try writer.u8(19)
      try writer.u8(theme.rawValue)
    case let .presentServiceState(active):
      try writer.u8(20)
      try writer.boolean(active)
    case let .presentExport(stagedPath):
      try writer.u8(21)
      try writer.string(stagedPath, maximum: 1024)
    case let .runtimeReady(endpoint, launchToken):
      try writer.u8(22)
      try writer.string(endpoint, maximum: 256)
      try writer.string(launchToken, maximum: 128)
    }
    return writer.data
  }

  public static func decode(
    _ hostCommand: FloatViewHostCommand
  ) throws -> FloatViewPlatformCommandEnvelope {
    guard hostCommand.kind == floatViewPlatformCommandKind else {
      throw FloatViewBridgeCodecError.invalidTag
    }
    var reader = try BridgeReader(hostCommand.payload)
    guard try reader.u8() == floatViewBridgeCodecVersion else {
      throw FloatViewBridgeCodecError.invalidVersion
    }
    let generation = try reader.u64()
    let operation = try reader.u64()
    let command: FloatViewPlatformCommand = switch try reader.u8() {
    case 1: .bluetoothStartScan
    case 2: .bluetoothStopScan
    case 3: try .bluetoothConnect(deviceToken: reader.bytes(maximum: 256))
    case 4: try .bluetoothDisconnect(connectionGeneration: reader.u64())
    case 5:
      try .bluetoothWrite(
        connectionGeneration: reader.u64(),
        bytes: reader.bytes(maximum: FloatViewCoreRuntime.maximumPayloadBytes)
      )
    case 6: try .locationStart(background: reader.boolean())
    case 7: .locationStop
    case 8: try .setBackgroundLogging(enabled: reader.boolean())
    case 9:
      try .readResource(
        namespace: tagged(reader.u8()),
        logicalName: reader.string(maximum: 128),
        maximumBytes: reader.u64()
      )
    case 10:
      try .stageDocument(
        destination: reader.string(maximum: 1024),
        maximumBytes: reader.u64()
      )
    case 11: try .resolveStorageRoot(kind: tagged(reader.u8()))
    case 12: try .availableSpace(root: reader.string(maximum: 1024))
    case 13: .monotonicClock
    case 14: .wallClock
    case 15: try .secureRandom(count: reader.u16())
    case 16:
      try .diagnostic(
        stage: reader.string(maximum: floatViewMaximumDiagnosticBytes),
        code: reader.string(maximum: floatViewMaximumDiagnosticBytes)
      )
    case 17: try .presentPermission(permission: tagged(reader.u8()))
    case 18: .presentSettings
    case 19: try .presentTheme(theme: tagged(reader.u8()))
    case 20: try .presentServiceState(active: reader.boolean())
    case 21: try .presentExport(stagedPath: reader.string(maximum: 1024))
    case 22:
      try .runtimeReady(
        endpoint: reader.string(maximum: 256),
        launchToken: reader.string(maximum: 128)
      )
    default: throw FloatViewBridgeCodecError.invalidTag
    }
    try reader.finish()
    guard generation == hostCommand.runtimeGeneration,
          operation == hostCommand.operationID
    else {
      throw FloatViewBridgeCodecError.commandEnvelopeMismatch
    }
    return FloatViewPlatformCommandEnvelope(
      runtimeGeneration: generation,
      operationID: operation,
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

public struct FloatViewPlatformEventEnvelope: Sendable, Equatable {
  public let runtimeGeneration: UInt64
  public let connectionGeneration: UInt64
  public let adapterOrder: UInt64
  public let event: FloatViewPlatformEvent

  public init(
    runtimeGeneration: UInt64,
    connectionGeneration: UInt64,
    adapterOrder: UInt64,
    event: FloatViewPlatformEvent
  ) {
    self.runtimeGeneration = runtimeGeneration
    self.connectionGeneration = connectionGeneration
    self.adapterOrder = adapterOrder
    self.event = event
  }

  public func encode() throws -> Data {
    var writer = BridgeWriter()
    try writer.u8(floatViewBridgeCodecVersion)
    try writer.u64(runtimeGeneration)
    try writer.u64(connectionGeneration)
    try writer.u64(adapterOrder)
    switch event {
    case let .bluetoothState(radio, authorization):
      try writer.u8(1)
      try writer.u8(radio.rawValue)
      try writer.u8(authorization.rawValue)
    case let .bluetoothDevice(deviceToken, name, rssi):
      guard !deviceToken.isEmpty, deviceToken.count <= 256,
            name?.utf8.count ?? 0 <= 128
      else { throw FloatViewBridgeCodecError.payloadTooLarge }
      try writer.u8(2)
      try writer.bytes(deviceToken)
      try writer.optionalString(name)
      try writer.i16(rssi)
    case let .bluetoothConnection(eventGeneration, state):
      try writer.u8(3)
      try writer.u64(eventGeneration)
      try writer.u8(state.rawValue)
    case let .locationAuthorization(authorization):
      try writer.u8(4)
      try writer.u8(authorization.rawValue)
    case let .location(latitude, longitude, accuracyM):
      guard latitude.isFinite, longitude.isFinite,
            (-90 ... 90).contains(latitude), (-180 ... 180).contains(longitude),
            accuracyM.map({ $0.isFinite && $0 >= 0 }) ?? true
      else { throw FloatViewBridgeCodecError.invalidValue }
      try writer.u8(5)
      try writer.f64(latitude)
      try writer.f64(longitude)
      try writer.optionalF64(accuracyM)
    case let .lifecycle(foreground, backgroundEligible):
      try writer.u8(6)
      try writer.boolean(foreground)
      try writer.boolean(backgroundEligible)
    case let .loggingService(active):
      try writer.u8(7)
      try writer.boolean(active)
    case let .theme(theme):
      try writer.u8(8)
      try writer.u8(theme.rawValue)
    }
    return writer.data
  }

  public static func decode(_ data: Data) throws -> FloatViewPlatformEventEnvelope {
    var reader = try BridgeReader(data)
    guard try reader.u8() == floatViewBridgeCodecVersion else {
      throw FloatViewBridgeCodecError.invalidVersion
    }
    let runtimeGeneration = try reader.u64()
    let connectionGeneration = try reader.u64()
    let adapterOrder = try reader.u64()
    let event: FloatViewPlatformEvent = switch try reader.u8() {
    case 1:
      try .bluetoothState(
        radio: tagged(reader.u8()),
        authorization: tagged(reader.u8())
      )
    case 2:
      try .bluetoothDevice(
        deviceToken: reader.bytes(maximum: 256),
        name: reader.optionalString(maximum: 128),
        rssi: reader.i16()
      )
    case 3:
      try .bluetoothConnection(
        connectionGeneration: reader.u64(),
        state: tagged(reader.u8())
      )
    case 4: try .locationAuthorization(tagged(reader.u8()))
    case 5:
      try .location(
        latitude: reader.f64(),
        longitude: reader.f64(),
        accuracyM: reader.optionalF64()
      )
    case 6:
      try .lifecycle(
        foreground: reader.boolean(),
        backgroundEligible: reader.boolean()
      )
    case 7: try .loggingService(active: reader.boolean())
    case 8: try .theme(tagged(reader.u8()))
    default: throw FloatViewBridgeCodecError.invalidTag
    }
    try reader.finish()
    return FloatViewPlatformEventEnvelope(
      runtimeGeneration: runtimeGeneration,
      connectionGeneration: connectionGeneration,
      adapterOrder: adapterOrder,
      event: event
    )
  }
}

private func tagged<T: RawRepresentable>(_ value: UInt8) throws -> T where T.RawValue == UInt8 {
  guard let decoded = T(rawValue: value) else { throw FloatViewBridgeCodecError.invalidTag }
  return decoded
}

private struct BridgeWriter {
  var data = Data()

  mutating func reserve(_ count: Int) throws {
    guard count >= 0,
          data.count <= FloatViewCoreRuntime.maximumPayloadBytes - count
    else { throw FloatViewBridgeCodecError.payloadTooLarge }
  }

  mutating func u8(_ value: UInt8) throws {
    try reserve(1)
    data.append(value)
  }

  mutating func u32(_ value: UInt32) throws {
    try raw(value.bigEndian)
  }

  mutating func u16(_ value: UInt16) throws {
    try raw(value.bigEndian)
  }

  mutating func u64(_ value: UInt64) throws {
    try raw(value.bigEndian)
  }

  mutating func i16(_ value: Int16) throws {
    try raw(UInt16(bitPattern: value).bigEndian)
  }

  mutating func f64(_ value: Double) throws {
    try u64(value.bitPattern)
  }

  mutating func boolean(_ value: Bool) throws {
    try u8(value ? 1 : 0)
  }

  mutating func bytes(_ value: Data) throws {
    guard let count = UInt32(exactly: value.count) else {
      throw FloatViewBridgeCodecError.payloadTooLarge
    }
    try u32(count)
    try reserve(value.count)
    data.append(value)
  }

  mutating func string(_ value: String, maximum: Int) throws {
    let encoded = Data(value.utf8)
    guard !encoded.isEmpty, encoded.count <= maximum else {
      throw FloatViewBridgeCodecError.payloadTooLarge
    }
    try bytes(encoded)
  }

  mutating func optionalString(_ value: String?) throws {
    try boolean(value != nil)
    if let value {
      try bytes(Data(value.utf8))
    }
  }

  mutating func optionalF64(_ value: Double?) throws {
    try boolean(value != nil)
    if let value { try f64(value) }
  }

  mutating func raw<T>(_ value: T) throws {
    try withUnsafeBytes(of: value) { bytes in
      try reserve(bytes.count)
      data.append(contentsOf: bytes)
    }
  }
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
