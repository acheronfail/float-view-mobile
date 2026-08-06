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
  public static func packageRequest(enabled: Bool) -> [UInt8] {
    RefloatWire.request(.packageLock, parameters: [enabled ? 0 : 1])
  }

  public static func flywheelRequest(enabled: Bool) -> [UInt8] {
    RefloatWire.request(
      .flywheel,
      parameters: [enabled ? 0x81 : 0x80, 0, 0, 0, 0, 1]
    )
  }

  public static func lightingRequest(
    profile: RefloatControlProfile,
    control: RefloatBooleanControl?,
    enabled: Bool = false,
    current: RefloatLightingState
  ) throws -> [UInt8] {
    guard control == nil || control == .leds || control == .headlights else {
      throw VescProtocolError.invalidFrame
    }
    let mask: UInt8 = switch control {
    case .leds: 0x01
    case .headlights: 0x02
    case nil: 0
    default: 0
    }
    var flags: UInt8 = (current.ledsEnabled ? 0x01 : 0) | (current.headlightsEnabled ? 0x02 : 0)
    if let control {
      let bit: UInt8 = control == .leds ? 0x01 : 0x02
      flags = enabled ? flags | bit : flags & ~bit
    }
    switch profile.lighting {
    case .legacyCommand202:
      return RefloatWire.request(.legacyLights, parameters: [mask, flags])
    case .stableCommand20:
      return RefloatWire.request(.stableLights, parameters: [0, 0, 0, mask, flags])
    }
  }

  public static func fatalClearRequest(profile: RefloatControlProfile) throws -> [UInt8] {
    guard profile.fatalClearAvailable else { throw VescProtocolError.unsupportedCommand(36) }
    return RefloatWire.request(.alertsControl, parameters: [1])
  }

  public static func decodeLightingResponse(
    _ payload: [UInt8], profile: RefloatControlProfile
  ) throws -> RefloatLightingState {
    let command: RefloatCommand = profile.lighting == .legacyCommand202 ? .legacyLights : .stableLights
    guard payload.count == 4,
          payload[0] == VescCommand.customAppData.rawValue,
          payload[1] == RefloatWire.packageID,
          payload[2] == command.rawValue,
          payload[3] & ~0x03 == 0
    else { throw VescProtocolError.invalidFrame }
    return .init(
      ledsEnabled: payload[3] & 0x01 != 0,
      headlightsEnabled: payload[3] & 0x02 != 0
    )
  }
}
