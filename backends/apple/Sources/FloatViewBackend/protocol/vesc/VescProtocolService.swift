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

  public init(code: UInt8, kind: Kind) {
    self.code = code
    self.kind = kind
  }
}

public enum VescPayloadDecoder {
  public static func firmware(_ payload: [UInt8]) throws -> FirmwareInfo {
    try VescFirmwareIdentityDecoder.decode(payload)
  }

  public static func realtime(_ payload: [UInt8]) throws -> RealtimeValues {
    try VescControllerDataDecoder.realtime(payload)
  }

  public static func bmsValues(
    _ payload: [UInt8], expectedCanID: UInt8? = nil, observedAt: Date
  ) throws -> BmsSnapshot {
    try VescBmsValuesDecoder.decode(
      payload, expectedCanID: expectedCanID, observedAt: observedAt
    )
  }

  public static func currentLimits(_ payload: [UInt8]) throws -> ControllerCurrentLimits {
    try VescControllerDataDecoder.currentLimits(payload)
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
      VescCustomConfigurationCodec.valuesRequest(defaults: defaults), via: route
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
