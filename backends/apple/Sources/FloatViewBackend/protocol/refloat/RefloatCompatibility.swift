import CVescRefloatCompatibility
import Foundation

public enum RefloatTelemetryFamily: Sendable, Equatable {
  case legacyFixed
  case dynamic
}

public enum RefloatWireProfile: Sendable, Equatable {
  case legacyFixed
  case dynamic
  case dynamicAlerts
}

public enum RefloatLightingProfile: Sendable, Equatable {
  case legacyCommand202
  case stableCommand20
}

public struct RefloatControlProfile: Sendable, Equatable {
  public let lighting: RefloatLightingProfile
  public let fatalClearAvailable: Bool

  public init(lighting: RefloatLightingProfile, fatalClearAvailable: Bool) {
    self.lighting = lighting
    self.fatalClearAvailable = fatalClearAvailable
  }
}

/// One complete Rust-accepted INFO compatibility decision.
public struct RefloatCompatibilityDecision: Sendable, Equatable {
  public let wireVersion: String
  public let profile: RefloatWireProfile
  public let family: RefloatTelemetryFamily
  public let controlProfile: RefloatControlProfile

  init(
    wireVersion: String,
    profile: RefloatWireProfile,
    family: RefloatTelemetryFamily,
    controlProfile: RefloatControlProfile
  ) {
    self.wireVersion = wireVersion
    self.profile = profile
    self.family = family
    self.controlProfile = controlProfile
  }
}

/// Strict Apple adapter for Rust-owned Refloat INFO compatibility policy.
public enum RefloatCompatibility {
  private static let expectedABI: UInt32 = 1
  private static let recordVersion: UInt8 = 1
  private static let recordKind: UInt8 = 1

  public static func infoRequest() throws -> [UInt8] {
    try requireCompatibleABI()
    var request = [UInt8](
      repeating: 0,
      count: Int(FLOAT_VIEW_REFLOAT_COMPATIBILITY_INFO_REQUEST_BYTES)
    )
    let capacity = request.count
    var length = 0
    let status = request.withUnsafeMutableBytes { bytes in
      float_view_refloat_compatibility_info_request(
        bytes.bindMemory(to: UInt8.self).baseAddress,
        capacity,
        &length
      )
    }
    try requireSuccess(status)
    guard length == request.count else { throw VescProtocolError.truncatedPayload }
    return request
  }

  public static func decodeInfo(_ payload: [UInt8]) throws -> RefloatCompatibilityDecision {
    try requireCompatibleABI()
    var record = [UInt8](
      repeating: 0,
      count: Int(FLOAT_VIEW_REFLOAT_COMPATIBILITY_RECORD_BYTES)
    )
    let capacity = record.count
    var length = 0
    let status = payload.withUnsafeBytes { payloadBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_refloat_compatibility_decode_info(
          payloadBytes.bindMemory(to: UInt8.self).baseAddress,
          payload.count,
          recordBytes.bindMemory(to: UInt8.self).baseAddress,
          capacity,
          &length
        )
      }
    }
    try requireSuccess(status)
    guard length == record.count else { throw VescProtocolError.truncatedPayload }
    return try decision(from: record)
  }

  static func admits(layoutRecord: [UInt8]) throws -> Bool {
    try requireCompatibleABI()
    let status = layoutRecord.withUnsafeBytes { bytes in
      float_view_refloat_compatibility_admit_layout(
        bytes.bindMemory(to: UInt8.self).baseAddress,
        layoutRecord.count
      )
    }
    switch status {
    case UInt32(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK):
      return true
    case UInt32(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_LAYOUT):
      return false
    default:
      try requireSuccess(status)
      return false
    }
  }

  static func decision(from record: [UInt8]) throws -> RefloatCompatibilityDecision {
    guard record.count == Int(FLOAT_VIEW_REFLOAT_COMPATIBILITY_RECORD_BYTES),
          record[0] == recordVersion,
          record[1] == recordKind,
          record[4] <= 1,
          record[9] <= 1,
          record[10] == 0,
          record[11] == 0,
          record[4] == 1 || record[5] == 0
    else { throw VescProtocolError.truncatedPayload }

    let wireVersion = record[4] == 1
      ? "\(record[2]).\(record[3]).\(record[5])"
      : "\(record[2]).\(record[3])"
    let decision: (
      RefloatWireProfile, RefloatTelemetryFamily, RefloatControlProfile
    ) = switch (record[6], record[7], record[8], record[9]) {
    case (1, 1, 1, 0):
      (.legacyFixed, .legacyFixed, .init(
        lighting: .legacyCommand202, fatalClearAvailable: false
      ))
    case (2, 2, 1, 0):
      (.dynamic, .dynamic, .init(
        lighting: .legacyCommand202, fatalClearAvailable: false
      ))
    case (3, 2, 2, 1):
      (.dynamicAlerts, .dynamic, .init(
        lighting: .stableCommand20, fatalClearAvailable: true
      ))
    default:
      throw VescProtocolError.truncatedPayload
    }
    return RefloatCompatibilityDecision(
      wireVersion: wireVersion,
      profile: decision.0,
      family: decision.1,
      controlProfile: decision.2
    )
  }

  private static func requireCompatibleABI() throws {
    guard float_view_refloat_compatibility_abi_version() == expectedABI else {
      throw VescProtocolError.truncatedPayload
    }
  }

  private static func requireSuccess(_ status: UInt32) throws {
    switch status {
    case UInt32(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK):
      return
    case UInt32(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_ENVELOPE),
         UInt32(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_PROTOCOL),
         UInt32(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_RELEASE),
         UInt32(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_LAYOUT):
      throw VescProtocolError.invalidFrame
    default:
      throw VescProtocolError.truncatedPayload
    }
  }
}
