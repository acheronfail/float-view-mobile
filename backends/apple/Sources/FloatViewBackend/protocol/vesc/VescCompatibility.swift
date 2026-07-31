import Foundation

public struct VescFirmwareVersion: Hashable, Sendable {
  public let major: UInt8
  public let minor: UInt8

  public init(_ major: UInt8, _ minor: UInt8) {
    self.major = major
    self.minor = minor
  }
}

public enum VescAccessMode: String, Sendable {
  case readOnly
  case readWrite
}

public enum VescOperation: Sendable {
  case read
  case write
}

/// Exact controller firmware versions covered by immutable protocol fixtures.
public enum VescCompatibility {
  public static let supported: Set<VescFirmwareVersion> = [
    .init(6, 2),
    .init(6, 5),
    .init(6, 6),
    .init(7, 0),
  ]

  public static func accessMode(_ firmware: FirmwareRecord) -> VescAccessMode? {
    guard firmware.hardwareType == 0,
          supported.contains(.init(firmware.major, firmware.minor))
    else { return nil }
    return firmware.testVersion == 0 ? .readWrite : .readOnly
  }

  public static func supports(_ firmware: FirmwareRecord) -> Bool {
    accessMode(firmware) != nil
  }

  public static func permits(_ operation: VescOperation, for firmware: FirmwareRecord) -> Bool {
    guard let mode = accessMode(firmware) else { return false }
    return operation == .read || mode == .readWrite
  }
}
