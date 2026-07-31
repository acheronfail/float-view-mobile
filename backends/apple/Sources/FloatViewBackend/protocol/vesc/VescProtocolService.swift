import Foundation

public struct FirmwareInfo: Equatable, Sendable {
  public let major: UInt8
  public let minor: UInt8
  public let testVersion: UInt8
  public let hardwareName: String
  public let firmwareName: String?
  public let hardwareType: UInt8
  public let controllerUUID: Data?

  public init(
    major: UInt8,
    minor: UInt8,
    testVersion: UInt8 = 0,
    hardwareName: String,
    firmwareName: String? = nil,
    hardwareType: UInt8 = 0,
    controllerUUID: Data? = nil
  ) {
    self.major = major
    self.minor = minor
    self.testVersion = testVersion
    self.hardwareName = hardwareName
    self.firmwareName = firmwareName
    self.hardwareType = hardwareType
    self.controllerUUID = controllerUUID
  }
}

public struct RealtimeValues: Equatable, Sendable {
  public let controllerTempC: Double
  public let motorTempC: Double
  public let motorCurrentA: Double
  public let batteryCurrentA: Double
  public let dutyCycle: Double
  public let erpm: Int32
  public let packVoltageV: Double
  public let faultCode: UInt8
  public let fault: VescFault
}

public struct VescFault: Equatable, Sendable {
  public enum Kind: String, Equatable, Sendable {
    case none
    case overVoltage = "over_voltage"
    case unknown
  }

  public let code: UInt8
  public let kind: Kind

  public init(code: UInt8) {
    self.code = code
    self.kind = switch code {
    case 0: .none
    case 1: .overVoltage
    default: .unknown
    }
  }
}

private struct ByteReader {
  let bytes: [UInt8]
  var offset = 0

  mutating func u8() throws -> UInt8 {
    guard offset < bytes.count else { throw VescProtocolError.truncatedPayload }
    defer { offset += 1 }
    return bytes[offset]
  }

  mutating func i16(scale: Double) throws -> Double {
    let raw = Int16(bitPattern: UInt16(try u8()) << 8 | UInt16(try u8()))
    return Double(raw) / scale
  }

  mutating func i32(scale: Double = 1) throws -> Double {
    var value: UInt32 = 0
    for _ in 0..<4 { value = value << 8 | UInt32(try u8()) }
    return Double(Int32(bitPattern: value)) / scale
  }

  mutating func float32Auto() throws -> Double {
    var raw: UInt32 = 0
    for _ in 0..<4 { raw = raw << 8 | UInt32(try u8()) }
    let exponentBits = Int((raw >> 23) & 0xff)
    let significandBits = raw & 0x7fffff
    guard exponentBits != 0 || significandBits != 0 else { return 0 }
    let significand = Double(significandBits) / 16_777_216.0 + 0.5
    let value = significand * pow(2, Double(exponentBits - 126))
    return raw & 0x80000000 == 0 ? value : -value
  }

  mutating func skip(_ count: Int) throws {
    guard offset + count <= bytes.count else { throw VescProtocolError.truncatedPayload }
    offset += count
  }
}

public enum VescPayloadDecoder {
  public static func firmware(_ payload: [UInt8]) throws -> FirmwareInfo {
    try VescFirmwareIdentityDecoder.decode(payload)
  }

  public static func realtime(_ payload: [UInt8]) throws -> RealtimeValues {
    var reader = ByteReader(bytes: payload)
    let command = try reader.u8()
    guard command == VescCommand.getRealtimeValues.rawValue else {
      throw VescProtocolError.unsupportedCommand(command)
    }
    let controllerTemp = try reader.i16(scale: 10)
    let motorTemp = try reader.i16(scale: 10)
    let motorCurrent = try reader.i32(scale: 100)
    let batteryCurrent = try reader.i32(scale: 100)
    try reader.skip(8)
    let duty = try reader.i16(scale: 1000)
    let erpm = Int32(try reader.i32())
    let packVoltage = try reader.i16(scale: 10)
    let fault = try reader.u8()
    return RealtimeValues(
      controllerTempC: controllerTemp,
      motorTempC: motorTemp,
      motorCurrentA: motorCurrent,
      batteryCurrentA: batteryCurrent,
      dutyCycle: duty,
      erpm: erpm,
      packVoltageV: packVoltage,
      faultCode: fault,
      fault: VescFault(code: fault)
    )
  }

  public static func currentLimits(_ payload: [UInt8]) throws -> ControllerCurrentLimits {
    var reader = ByteReader(bytes: payload)
    let command = try reader.u8()
    guard command == VescCommand.getMotorConfiguration.rawValue else {
      throw VescProtocolError.unsupportedCommand(command)
    }
    try reader.skip(8) // MCCONF signature and four enum fields.
    let value = ControllerCurrentLimits(
      motorCurrentMaxA: try reader.float32Auto(),
      motorCurrentMinA: try reader.float32Auto(),
      batteryCurrentMaxA: try reader.float32Auto(),
      batteryCurrentMinA: try reader.float32Auto()
    )
    let limits = [
      value.motorCurrentMaxA, value.motorCurrentMinA,
      value.batteryCurrentMaxA, value.batteryCurrentMinA,
    ]
    guard limits.allSatisfy({ $0.isFinite && abs($0) > 0 }) else {
      throw VescProtocolError.truncatedPayload
    }
    return value
  }

}

public final class VescProtocolService: VescProtocolPort, @unchecked Sendable {
  private let lock = NSLock()
  private var stream = VescFrameStreamDecoder()
  private var pendingFirmwareCanID: UInt8?
  private var realtimeFields: [String] = []
  private var realtimeRuntimeFields: [String] = []

  public init() {}

  public func reset() {
    lock.lock()
    stream = VescFrameStreamDecoder()
    pendingFirmwareCanID = nil
    realtimeFields.removeAll()
    realtimeRuntimeFields.removeAll()
    lock.unlock()
  }

  public func firmwareRequest(route: ControllerRoute) throws -> Data {
    lock.lock()
    pendingFirmwareCanID = route.canID
    lock.unlock()
    let payload = VescCommandRouter.route([VescCommand.getFirmwareVersion.rawValue], via: route)
    return Data(try VescFrameCodec.encode(payload))
  }

  public func motorConfigurationRequest(route: ControllerRoute) throws -> Data {
    Data(try VescFrameCodec.encode(
      VescCommandRouter.route([VescCommand.getMotorConfiguration.rawValue], via: route)
    ))
  }

  public func canPingRequest() throws -> Data {
    Data(try VescFrameCodec.encode([VescCommand.pingCAN.rawValue]))
  }

  public func refloatInfoRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(
      .info,
      parameters: [RefloatWire.infoProtocolVersion, RefloatWire.infoFlags]
    )
    let payload = VescCommandRouter.route(command, via: route)
    return Data(try VescFrameCodec.encode(payload))
  }

  public func refloatTelemetryIDsRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(.realtimeDataInternalIDs)
    return Data(try VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func refloatTelemetryRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(.realtimeDataInternal)
    return Data(try VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func refloatLegacyTelemetryRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(
      .getAllData,
      parameters: [RefloatWire.legacyTelemetryDetailLevel]
    )
    return Data(try VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func refloatCompactTelemetryRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(
      .getAllData,
      parameters: [RefloatWire.legacyTelemetryDetailLevel]
    )
    return Data(try VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    lock.lock()
    defer { lock.unlock() }
    return try stream.append(bytes).compactMap { payload in
      switch payload.first {
      case VescCommand.getFirmwareVersion.rawValue:
        let info = try VescPayloadDecoder.firmware(payload)
        return .firmware(
          FirmwareRecord(
            major: info.major,
            minor: info.minor,
            testVersion: info.testVersion,
            hardwareName: info.hardwareName,
            firmwareName: info.firmwareName,
            hardwareType: info.hardwareType,
            canID: pendingFirmwareCanID,
            controllerUUID: info.controllerUUID
          )
        )
      case VescCommand.getMotorConfiguration.rawValue:
        return .currentLimits(try VescPayloadDecoder.currentLimits(payload))
      case VescCommand.pingCAN.rawValue:
        return .canDevices(Array(Set(payload.dropFirst())).sorted())
      case VescCommand.customAppData.rawValue:
        guard payload.count >= 4, let command = RefloatWire.command(in: payload) else {
          return nil
        }
        if command == .info {
          return .refloatInfo(version: try RefloatInfoDecoder.decodeVersion(payload))
        }
        if command == .realtimeDataInternalIDs {
          let (fields, runtimeFields) = try RefloatDynamicTelemetryDecoder.decodeFields(payload)
          realtimeFields = fields
          realtimeRuntimeFields = runtimeFields
          return .refloatTelemetryLayout(fields: fields, runtimeFields: runtimeFields)
        }
        if command == .realtimeDataInternal {
          return .refloatTelemetry(
            try RefloatDynamicTelemetryDecoder.decodeData(
              payload,
              fields: realtimeFields,
              runtimeFields: realtimeRuntimeFields
            )
          )
        }
        if command == .getAllData {
          return .refloatTelemetry(try RefloatLegacyTelemetryDecoder.decode(payload))
        }
        return nil
      default:
        return nil
      }
    }
  }

}
