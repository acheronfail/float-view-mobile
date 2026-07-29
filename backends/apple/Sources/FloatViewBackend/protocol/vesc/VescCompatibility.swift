import Foundation

public struct VescFirmwareVersion: Hashable, Sendable {
  public let major: UInt8
  public let minor: UInt8

  public init(_ major: UInt8, _ minor: UInt8) {
    self.major = major
    self.minor = minor
  }
}

/// Exact controller firmware versions covered by immutable protocol fixtures.
public enum VescCompatibility {
  public static let supported: Set<VescFirmwareVersion> = [
    .init(6, 2),
    .init(6, 5),
    .init(6, 6),
  ]

  public static func supports(_ firmware: FirmwareRecord) -> Bool {
    supported.contains(.init(firmware.major, firmware.minor))
  }
}
