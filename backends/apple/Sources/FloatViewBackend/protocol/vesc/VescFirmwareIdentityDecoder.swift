import CVescFirmwareIdentity
import Foundation

/// Apple adapter for the Rust-owned COMM_FW_VERSION structural decoder.
public enum VescFirmwareIdentityDecoder {
  public static let motorHardwareType: UInt8 = 0
  public static let bmsHardwareType: UInt8 = 1

  private static let expectedABI: UInt32 = 1
  private static let recordHeaderBytes = 26
  private static let maximumEncodedNameBytes = 192
  private static let maximumRecordBytes = 410
  private static let firmwareNamePresent: UInt8 = 1
  private static let hardwareCRCPresent: UInt8 = 2
  private static let knownFlags = firmwareNamePresent | hardwareCRCPresent

  public static func decode(_ payload: [UInt8]) throws -> FirmwareInfo {
    try requireCompatibleABI(float_view_vesc_firmware_identity_abi_version())

    var record = [UInt8](repeating: 0, count: maximumRecordBytes)
    let recordCapacity = record.count
    var recordLength = 0
    var unsupportedCommand: UInt8 = 0
    let status = payload.withUnsafeBytes { payloadBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_vesc_firmware_identity_decode(
          payloadBytes.bindMemory(to: UInt8.self).baseAddress,
          payload.count,
          recordBytes.bindMemory(to: UInt8.self).baseAddress,
          recordCapacity,
          &recordLength,
          &unsupportedCommand
        )
      }
    }

    switch status {
    case UInt32(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_OK):
      guard recordLength <= record.count else {
        report("invalid result length")
        throw VescProtocolError.truncatedPayload
      }
      return try firmwareInfo(from: Array(record.prefix(recordLength)))
    case UInt32(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_UNSUPPORTED_COMMAND):
      throw VescProtocolError.unsupportedCommand(unsupportedCommand)
    case UInt32(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_TRUNCATED_PAYLOAD):
      throw VescProtocolError.truncatedPayload
    default:
      report("bridge status \(status)")
      throw VescProtocolError.truncatedPayload
    }
  }

  static func requireCompatibleABI(_ actual: UInt32) throws {
    guard actual == expectedABI else {
      report("ABI mismatch")
      throw VescProtocolError.truncatedPayload
    }
  }

  static func firmwareInfo(from record: [UInt8]) throws -> FirmwareInfo {
    guard record.count >= recordHeaderBytes,
          record.count <= maximumRecordBytes
    else { throw VescProtocolError.truncatedPayload }

    let flags = record[5]
    guard flags & ~knownFlags == 0 else { throw VescProtocolError.truncatedPayload }
    let hardwareNameLength = Int(record[6]) << 8 | Int(record[7])
    let firmwareNameLength = Int(record[8]) << 8 | Int(record[9])
    guard hardwareNameLength <= maximumEncodedNameBytes,
          firmwareNameLength <= maximumEncodedNameBytes,
          recordHeaderBytes + hardwareNameLength + firmwareNameLength == record.count,
          (flags & firmwareNamePresent != 0) == (firmwareNameLength > 0)
    else { throw VescProtocolError.truncatedPayload }

    let crcBytes = record[22 ..< 26]
    let hasCRC = flags & hardwareCRCPresent != 0
    guard hasCRC || crcBytes.allSatisfy({ $0 == 0 }) else {
      throw VescProtocolError.truncatedPayload
    }
    let hardwareNameEnd = recordHeaderBytes + hardwareNameLength
    let hardwareName = String(decoding: record[recordHeaderBytes ..< hardwareNameEnd], as: UTF8.self)
    let firmwareName = firmwareNameLength == 0
      ? nil
      : String(decoding: record[hardwareNameEnd ..< record.count], as: UTF8.self)
    let crc = hasCRC
      ? crcBytes.reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
      : nil

    return FirmwareInfo(
      major: record[0],
      minor: record[1],
      testVersion: record[2],
      hardwareName: hardwareName,
      firmwareName: firmwareName,
      hardwareType: record[3],
      controllerUUID: Data(record[10 ..< 22]),
      customConfigurationCount: record[4],
      hardwareConfigurationCRC: crc
    )
  }

  private static func report(_ reason: String) {
    print("[FloatView] Rust firmware identity decode failed (\(reason))")
  }
}
