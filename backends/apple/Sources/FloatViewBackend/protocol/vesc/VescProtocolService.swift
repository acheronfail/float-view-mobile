import Foundation

public struct FirmwareInfo: Equatable, Sendable {
  public let major: UInt8
  public let minor: UInt8
  public let testVersion: UInt8
  public let hardwareName: String
  public let firmwareName: String?
  public let hardwareType: UInt8
  public let controllerUUID: Data?
  public let customConfigurationCount: UInt8
  public let hardwareConfigurationCRC: UInt32?

  public init(
    major: UInt8,
    minor: UInt8,
    testVersion: UInt8 = 0,
    hardwareName: String,
    firmwareName: String? = nil,
    hardwareType: UInt8 = 0,
    controllerUUID: Data? = nil,
    customConfigurationCount: UInt8 = 0,
    hardwareConfigurationCRC: UInt32? = nil
  ) {
    self.major = major
    self.minor = minor
    self.testVersion = testVersion
    self.hardwareName = hardwareName
    self.firmwareName = firmwareName
    self.hardwareType = hardwareType
    self.controllerUUID = controllerUUID
    self.customConfigurationCount = customConfigurationCount
    self.hardwareConfigurationCRC = hardwareConfigurationCRC
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
    kind = switch code {
    case 0: .none
    case 1: .overVoltage
    default: .unknown
    }
  }
}

private struct ByteReader {
  let bytes: [UInt8]
  var offset = 0

  var remaining: Int {
    bytes.count - offset
  }

  mutating func u8() throws -> UInt8 {
    guard offset < bytes.count else { throw VescProtocolError.truncatedPayload }
    defer { offset += 1 }
    return bytes[offset]
  }

  mutating func i16(scale: Double) throws -> Double {
    let raw = try Int16(bitPattern: UInt16(u8()) << 8 | UInt16(u8()))
    return Double(raw) / scale
  }

  mutating func i32(scale: Double = 1) throws -> Double {
    var value: UInt32 = 0
    for _ in 0 ..< 4 {
      value = try value << 8 | UInt32(u8())
    }
    return Double(Int32(bitPattern: value)) / scale
  }

  mutating func float32Auto() throws -> Double {
    var raw: UInt32 = 0
    for _ in 0 ..< 4 {
      raw = try raw << 8 | UInt32(u8())
    }
    let exponentBits = Int((raw >> 23) & 0xFF)
    let significandBits = raw & 0x7FFFFF
    guard exponentBits != 0 || significandBits != 0 else { return 0 }
    let significand = Double(significandBits) / 16_777_216.0 + 0.5
    let value = significand * pow(2, Double(exponentBits - 126))
    let signedValue = raw & 0x80000000 == 0 ? value : -value
    guard signedValue.isFinite, abs(signedValue) <= Double(Float.greatestFiniteMagnitude) else {
      throw VescProtocolError.truncatedPayload
    }
    return signedValue
  }

  mutating func skip(_ count: Int) throws {
    guard offset + count <= bytes.count else { throw VescProtocolError.truncatedPayload }
    offset += count
  }

  mutating func terminatedUTF8(maxContentBytes: Int) throws -> String {
    let start = offset
    let searchEnd = min(bytes.count, start + maxContentBytes + 1)
    guard let terminator = bytes[start ..< searchEnd].firstIndex(of: 0) else {
      throw VescProtocolError.truncatedPayload
    }
    guard let value = String(bytes: bytes[start ..< terminator], encoding: .utf8) else {
      throw VescProtocolError.truncatedPayload
    }
    offset = terminator + 1
    return value
  }
}

public enum VescPayloadDecoder {
  private static let batterySeriesCellOffsets: [UInt32: Int] = [
    0x2E43A161: 462, // 6.02
    0x3F829CF7: 452, // 6.05
    0x2EFD0142: 458, // 6.06
    0x57AD8F53: 463, // 7.00
    0x9307410D: 463, // 7.01
  ]

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
    let erpm = try Int32(reader.i32())
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

  public static func bmsValues(
    _ payload: [UInt8], expectedCanID: UInt8? = nil, observedAt: Date
  ) throws -> BmsSnapshot {
    var reader = ByteReader(bytes: payload)
    let command = try reader.u8()
    guard command == VescCommand.bmsGetValues.rawValue else {
      throw VescProtocolError.unsupportedCommand(command)
    }

    let totalVoltage = try reader.i32(scale: 1_000_000)
    let chargeVoltage = try reader.i32(scale: 1_000_000)
    let inputCurrent = try reader.i32(scale: 1_000_000)
    let inputICCurrent = try reader.i32(scale: 1_000_000)
    let sessionAmpHours = try reader.i32(scale: 1_000)
    let sessionWattHours = try reader.i32(scale: 1_000)

    let cellCount = try Int(reader.u8())
    guard cellCount <= 50 else { throw VescProtocolError.truncatedPayload }
    var cellVoltages: [Double] = []
    cellVoltages.reserveCapacity(cellCount)
    for _ in 0 ..< cellCount {
      try cellVoltages.append(reader.i16(scale: 1_000))
    }

    var balancing: [Bool] = []
    balancing.reserveCapacity(cellCount)
    for _ in 0 ..< cellCount {
      let flag = try reader.u8()
      guard flag <= 1 else { throw VescProtocolError.truncatedPayload }
      balancing.append(flag == 1)
    }

    let temperatureCount = try Int(reader.u8())
    guard temperatureCount <= 50 else { throw VescProtocolError.truncatedPayload }
    var temperatures: [Double] = []
    temperatures.reserveCapacity(temperatureCount)
    for _ in 0 ..< temperatureCount {
      try temperatures.append(reader.i16(scale: 100))
    }

    let icTemperature = try reader.i16(scale: 100)
    let humiditySensorTemperature = try reader.i16(scale: 100)
    let humidity = try reader.i16(scale: 100)
    let highestCellTemperature = try reader.i16(scale: 100)
    let stateOfCharge = try reader.i16(scale: 1_000)
    let stateOfHealth = try reader.i16(scale: 1_000)
    let reportedCanID = try reader.u8()
    guard reportedCanID <= 253, expectedCanID == nil || reportedCanID == expectedCanID else {
      throw VescProtocolError.truncatedPayload
    }
    let lifetimeChargeAmpHours = try reader.float32Auto()
    let lifetimeChargeWattHours = try reader.float32Auto()
    let lifetimeDischargeAmpHours = try reader.float32Auto()
    let lifetimeDischargeWattHours = try reader.float32Auto()
    let pressure = try reader.i16(scale: 0.1)

    let dataVersion: UInt8?
    let status: String?
    if reader.remaining == 0 {
      dataVersion = nil
      status = nil
    } else {
      dataVersion = try reader.u8()
      status = try reader.terminatedUTF8(maxContentBytes: 40)
      guard reader.remaining == 0 else { throw VescProtocolError.truncatedPayload }
    }

    let numericValues = [
      totalVoltage, chargeVoltage, inputCurrent, inputICCurrent,
      sessionAmpHours, sessionWattHours, icTemperature,
      humiditySensorTemperature, humidity, highestCellTemperature,
      stateOfCharge, stateOfHealth, lifetimeChargeAmpHours,
      lifetimeChargeWattHours, lifetimeDischargeAmpHours,
      lifetimeDischargeWattHours, pressure,
    ] + cellVoltages + temperatures
    guard numericValues.allSatisfy(\.isFinite) else {
      throw VescProtocolError.truncatedPayload
    }

    return BmsSnapshot(
      observedAt: observedAt, totalVoltageV: totalVoltage, chargeVoltageV: chargeVoltage,
      inputCurrentA: inputCurrent, inputICCurrentA: inputICCurrent,
      sessionAmpHoursAh: sessionAmpHours, sessionWattHoursWh: sessionWattHours,
      cellVoltagesV: cellVoltages, balancing: balancing, temperaturesC: temperatures,
      icTemperatureC: icTemperature, humiditySensorTemperatureC: humiditySensorTemperature,
      humidityPercent: humidity, highestCellTemperatureC: highestCellTemperature,
      stateOfCharge: stateOfCharge, stateOfHealth: stateOfHealth,
      reportedCanID: reportedCanID, lifetimeChargeAmpHoursAh: lifetimeChargeAmpHours,
      lifetimeChargeWattHoursWh: lifetimeChargeWattHours,
      lifetimeDischargeAmpHoursAh: lifetimeDischargeAmpHours,
      lifetimeDischargeWattHoursWh: lifetimeDischargeWattHours, pressurePa: pressure,
      dataVersion: dataVersion, status: status
    )
  }

  public static func currentLimits(_ payload: [UInt8]) throws -> ControllerCurrentLimits {
    var reader = ByteReader(bytes: payload)
    let command = try reader.u8()
    guard command == VescCommand.getMotorConfiguration.rawValue else {
      throw VescProtocolError.unsupportedCommand(command)
    }
    var signature: UInt32 = 0
    for _ in 0 ..< 4 {
      signature = try signature << 8 | UInt32(reader.u8())
    }
    try reader.skip(4) // Four enum fields.
    let value = try ControllerCurrentLimits(
      motorCurrentMaxA: reader.float32Auto(),
      motorCurrentMinA: reader.float32Auto(),
      batteryCurrentMaxA: reader.float32Auto(),
      batteryCurrentMinA: reader.float32Auto(),
      batterySeriesCells: batterySeriesCellOffsets[signature].flatMap { offset in
        guard payload.indices.contains(offset), payload[offset] > 0 else { return nil }
        return payload[offset]
      }
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
  public let supportsBmsTelemetry = true
  private let lock = NSLock()
  private var stream = VescFrameStreamDecoder()
  private var realtimeFields: [String] = []
  private var realtimeRuntimeFields: [String] = []

  public init() {}

  public func reset() {
    lock.lock()
    stream = VescFrameStreamDecoder()
    realtimeFields.removeAll()
    realtimeRuntimeFields.removeAll()
    lock.unlock()
  }

  public func firmwareRequest(route: ControllerRoute) throws -> Data {
    let payload = VescCommandRouter.route([VescCommand.getFirmwareVersion.rawValue], via: route)
    return try Data(VescFrameCodec.encode(payload))
  }

  public func motorConfigurationRequest(route: ControllerRoute) throws -> Data {
    try Data(VescFrameCodec.encode(
      VescCommandRouter.route([VescCommand.getMotorConfiguration.rawValue], via: route)
    ))
  }

  public func canPingRequest() throws -> Data {
    try Data(VescFrameCodec.encode([VescCommand.pingCAN.rawValue]))
  }

  public func controllerShutdownRequest(route: ControllerRoute) throws -> Data {
    try Data(VescFrameCodec.encode(
      VescCommandRouter.route([VescCommand.shutdown.rawValue, 0, 0], via: route)
    ))
  }

  public func refloatInfoRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(
      .info,
      parameters: [RefloatWire.infoProtocolVersion, RefloatWire.infoFlags]
    )
    let payload = VescCommandRouter.route(command, via: route)
    return try Data(VescFrameCodec.encode(payload))
  }

  public func refloatTelemetryIDsRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(.realtimeDataInternalIDs)
    return try Data(VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func refloatTelemetryRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(.realtimeDataInternal)
    return try Data(VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func refloatLegacyTelemetryRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(
      .getAllData,
      parameters: [RefloatWire.legacyTelemetryDetailLevel]
    )
    return try Data(VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func refloatCompactTelemetryRequest(route: ControllerRoute) throws -> Data {
    let command = RefloatWire.request(
      .getAllData,
      parameters: [RefloatWire.legacyTelemetryDetailLevel]
    )
    return try Data(VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func customConfigurationSchemaRequest(
    route: ControllerRoute, length: Int, offset: Int
  ) throws -> Data {
    try Data(VescFrameCodec.encode(VescCommandRouter.route(
      VescCustomConfigurationCodec.schemaRequest(length: length, offset: offset), via: route
    )))
  }

  public func customConfigurationValuesRequest(route: ControllerRoute, defaults: Bool) throws -> Data {
    try Data(VescFrameCodec.encode(VescCommandRouter.route(
      try VescCustomConfigurationCodec.valuesRequest(defaults: defaults), via: route
    )))
  }

  public func customConfigurationWriteRequest(route: ControllerRoute, values: Data) throws -> Data {
    try Data(VescFrameCodec.encode(VescCommandRouter.route(
      VescCustomConfigurationCodec.writeRequest(values: values), via: route
    )))
  }

  public func refloatPackageControlRequest(route: ControllerRoute, enabled: Bool) throws -> Data {
    try routedRefloatControl(RefloatControlCodec.packageRequest(enabled: enabled), route: route)
  }

  public func refloatFlywheelControlRequest(route: ControllerRoute, enabled: Bool) throws -> Data {
    try routedRefloatControl(RefloatControlCodec.flywheelRequest(enabled: enabled), route: route)
  }

  public func refloatLightingControlRequest(
    route: ControllerRoute,
    profile: RefloatControlProfile,
    control: RefloatBooleanControl?,
    enabled: Bool = false,
    current: RefloatLightingState
  ) throws -> Data {
    try routedRefloatControl(
      RefloatControlCodec.lightingRequest(
        profile: profile, control: control, enabled: enabled, current: current
      ),
      route: route
    )
  }

  public func refloatFatalClearRequest(
    route: ControllerRoute, profile: RefloatControlProfile
  ) throws -> Data {
    try routedRefloatControl(RefloatControlCodec.fatalClearRequest(profile: profile), route: route)
  }

  private func routedRefloatControl(_ command: [UInt8], route: ControllerRoute) throws -> Data {
    try Data(VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
  }

  public func bmsValuesRequest(connection: BmsConnection) throws -> Data {
    let route: ControllerRoute = switch connection {
    case .direct: .direct
    case let .can(canID): .can(canID)
    }
    return try Data(VescFrameCodec.encode(
      VescCommandRouter.route([VescCommand.bmsGetValues.rawValue], via: route)
    ))
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
            canID: nil,
            controllerUUID: info.controllerUUID,
            customConfigurationCount: info.customConfigurationCount,
            hardwareConfigurationCRC: info.hardwareConfigurationCRC
          )
        )
      case VescCommand.getMotorConfiguration.rawValue:
        return try .currentLimits(VescPayloadDecoder.currentLimits(payload))
      case VescCommand.pingCAN.rawValue:
        return .canDevices(Array(Set(payload.dropFirst())).sorted())
      case VescCommand.bmsGetValues.rawValue:
        return .bmsValuesPayload(Data(payload))
      case VescCommand.getCustomConfigurationXML.rawValue,
           VescCommand.getCustomConfiguration.rawValue,
           VescCommand.getCustomConfigurationDefault.rawValue,
           VescCommand.setCustomConfiguration.rawValue:
        return try .customConfiguration(VescCustomConfigurationCodec.decode(payload))
      case VescCommand.customAppData.rawValue:
        guard payload.count >= 4, let command = RefloatWire.command(in: payload) else {
          return nil
        }
        if command == .info {
          return try .refloatInfo(version: RefloatInfoDecoder.decodeVersion(payload))
        }
        if command == .realtimeDataInternalIDs {
          let (fields, runtimeFields) = try RefloatDynamicTelemetryDecoder.decodeFields(payload)
          realtimeFields = fields
          realtimeRuntimeFields = runtimeFields
          return .refloatTelemetryLayout(fields: fields, runtimeFields: runtimeFields)
        }
        if command == .realtimeDataInternal {
          return try .refloatTelemetry(
            RefloatDynamicTelemetryDecoder.decodeData(
              payload,
              fields: realtimeFields,
              runtimeFields: realtimeRuntimeFields
            )
          )
        }
        if command == .getAllData {
          return try .refloatTelemetry(RefloatLegacyTelemetryDecoder.decode(payload))
        }
        if command == .legacyLights {
          let profile = RefloatControlProfile(lighting: .legacyCommand202, fatalClearAvailable: false)
          return try .refloatLighting(
            command: .legacyLights,
            state: RefloatControlCodec.decodeLightingResponse(payload, profile: profile)
          )
        }
        if command == .stableLights {
          let profile = RefloatControlProfile(lighting: .stableCommand20, fatalClearAvailable: true)
          return try .refloatLighting(
            command: .stableLights,
            state: RefloatControlCodec.decodeLightingResponse(payload, profile: profile)
          )
        }
        return nil
      default:
        return nil
      }
    }
  }
}
