import Foundation

/// Strict owner for the bounded common COMM_FW_VERSION prefix and hardware-specific tails.
public enum VescFirmwareIdentityDecoder {
  public static let motorHardwareType: UInt8 = 0
  public static let bmsHardwareType: UInt8 = 1

  public static func decode(_ payload: [UInt8]) throws -> FirmwareInfo {
    guard let command = payload.first else { throw VescProtocolError.truncatedPayload }
    guard command == VescCommand.getFirmwareVersion.rawValue else {
      throw VescProtocolError.unsupportedCommand(command)
    }
    guard payload.count >= 3 else { throw VescProtocolError.truncatedPayload }

    let hardwareNameBytes = try terminatedBytes(in: payload, from: 3, maximum: 64)
    let uuidStart = hardwareNameBytes.endIndex + 1
    let commonEnd = uuidStart + 16 // UUID, pairing marker, test version, hardware type, config count.
    guard commonEnd <= payload.count else { throw VescProtocolError.truncatedPayload }

    let hardwareType = payload[uuidStart + 14]
    let firmwareName: String?
    switch hardwareType {
    case bmsHardwareType:
      guard payload[1] == 6, payload[2] == 0, payload.count == commonEnd else {
        throw VescProtocolError.truncatedPayload
      }
      firmwareName = nil
    case motorHardwareType:
      firmwareName = try extendedFirmwareName(in: payload, from: commonEnd)
    default:
      // Custom modules use the same four capability bytes, name, and optional CRC tail.
      firmwareName = try extendedFirmwareName(in: payload, from: commonEnd)
    }

    return FirmwareInfo(
      major: payload[1],
      minor: payload[2],
      testVersion: payload[uuidStart + 13],
      hardwareName: String(decoding: hardwareNameBytes, as: UTF8.self),
      firmwareName: firmwareName,
      hardwareType: hardwareType,
      controllerUUID: Data(payload[uuidStart ..< uuidStart + 12])
    )
  }

  private static func extendedFirmwareName(in payload: [UInt8], from commonEnd: Int) throws -> String? {
    let firmwareNameStart = commonEnd + 4
    guard firmwareNameStart < payload.count else { throw VescProtocolError.truncatedPayload }
    let bytes = try terminatedBytes(in: payload, from: firmwareNameStart, maximum: 64)
    let tailLength = payload.count - (bytes.endIndex + 1)
    guard tailLength == 0 || tailLength == 4 else { throw VescProtocolError.truncatedPayload }
    return String(decoding: bytes, as: UTF8.self).nilIfEmpty
  }

  private static func terminatedBytes(
    in payload: [UInt8], from start: Int, maximum: Int
  ) throws -> ArraySlice<UInt8> {
    guard start < payload.count,
          let end = payload[start...].firstIndex(of: 0),
          end - start <= maximum
    else { throw VescProtocolError.truncatedPayload }
    return payload[start ..< end]
  }
}

private extension String {
  var nilIfEmpty: String? {
    isEmpty ? nil : self
  }
}
