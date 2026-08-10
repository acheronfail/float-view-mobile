import CVescRefloatControl
import Foundation

public enum RefloatBooleanControl: String, Sendable, Codable, CaseIterable {
  case package, flywheel, leds, headlights
}

public struct RefloatLightingState: Sendable, Equatable {
  public let ledsEnabled: Bool
  public let headlightsEnabled: Bool

  public init(ledsEnabled: Bool, headlightsEnabled: Bool) {
    self.ledsEnabled = ledsEnabled
    self.headlightsEnabled = headlightsEnabled
  }
}

public enum RefloatControlCodec {
  private static let expectedABI: UInt32 = 1
  private static let maximumPayloadBytes = 9

  public static func packageRequest(enabled: Bool) throws -> [UInt8] {
    try encode(
      operation: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_PACKAGE),
      enabled: enabled
    )
  }

  public static func flywheelRequest(enabled: Bool) throws -> [UInt8] {
    try encode(
      operation: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_FLYWHEEL),
      enabled: enabled
    )
  }

  public static func lightingRequest(
    profile: RefloatControlProfile,
    control: RefloatBooleanControl?,
    enabled: Bool = false,
    current: RefloatLightingState
  ) throws -> [UInt8] {
    let target: UInt8 = switch control {
    case nil: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE)
    case .leds: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_LEDS)
    case .headlights: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_HEADLIGHTS)
    default: throw VescProtocolError.invalidFrame
    }
    return try encode(
      operation: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_LIGHTING),
      lightingProfile: bridgeProfile(profile.lighting),
      target: target,
      enabled: enabled,
      currentFlags: (current.ledsEnabled ? 1 : 0) | (current.headlightsEnabled ? 2 : 0)
    )
  }

  public static func fatalClearRequest(profile: RefloatControlProfile) throws -> [UInt8] {
    try encode(
      operation: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_FATAL_CLEAR),
      fatalClearAvailable: profile.fatalClearAvailable
    )
  }

  public static func decodeLightingResponse(
    _ payload: [UInt8], profile: RefloatControlProfile
  ) throws -> RefloatLightingState {
    try requireCompatibleABI(float_view_refloat_control_abi_version())
    var flags: UInt8 = 0
    let status = payload.withUnsafeBytes { bytes in
      float_view_refloat_control_decode_lighting(
        bytes.bindMemory(to: UInt8.self).baseAddress,
        payload.count,
        bridgeProfile(profile.lighting),
        &flags
      )
    }
    guard status == UInt32(FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK) else {
      if status != UInt32(FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INVALID_PAYLOAD) {
        report("decode bridge status \(status)")
      }
      throw VescProtocolError.invalidFrame
    }
    return try lightingState(flags: flags)
  }

  static func requireCompatibleABI(_ actual: UInt32) throws {
    guard actual == expectedABI else {
      report("ABI mismatch")
      throw VescProtocolError.invalidFrame
    }
  }

  static func lightingState(flags: UInt8) throws -> RefloatLightingState {
    guard flags & ~0x03 == 0 else { throw VescProtocolError.invalidFrame }
    return .init(ledsEnabled: flags & 1 != 0, headlightsEnabled: flags & 2 != 0)
  }

  private static func encode(
    operation: UInt8,
    lightingProfile: UInt8 = UInt8(FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE),
    fatalClearAvailable: Bool = false,
    target: UInt8 = UInt8(FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE),
    enabled: Bool = false,
    currentFlags: UInt8 = 0
  ) throws -> [UInt8] {
    try requireCompatibleABI(float_view_refloat_control_abi_version())
    var output = [UInt8](repeating: 0, count: maximumPayloadBytes)
    let capacity = output.count
    var outputLength = 0
    var unsupportedCommand: UInt8 = 0
    let status = output.withUnsafeMutableBytes { bytes in
      float_view_refloat_control_encode(
        operation,
        lightingProfile,
        fatalClearAvailable ? 1 : 0,
        target,
        enabled ? 1 : 0,
        currentFlags,
        bytes.bindMemory(to: UInt8.self).baseAddress,
        capacity,
        &outputLength,
        &unsupportedCommand
      )
    }
    switch status {
    case UInt32(FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK):
      guard outputLength > 0, outputLength <= output.count else {
        report("invalid encode output length")
        throw VescProtocolError.invalidFrame
      }
      return Array(output.prefix(outputLength))
    case UInt32(FLOAT_VIEW_REFLOAT_CONTROL_STATUS_UNSUPPORTED_COMMAND):
      throw VescProtocolError.unsupportedCommand(unsupportedCommand)
    default:
      report("encode bridge status \(status)")
      throw VescProtocolError.invalidFrame
    }
  }

  private static func bridgeProfile(_ profile: RefloatLightingProfile) -> UInt8 {
    switch profile {
    case .legacyCommand202: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_LEGACY)
    case .stableCommand20: UInt8(FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_STABLE)
    }
  }

  private static func report(_ reason: String) {
    NSLog("[FloatView] Rust Refloat control codec failed (%@)", reason)
  }
}
