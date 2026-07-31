import Foundation

/// Strict owner for the source-defined COMM_FW_VERSION identity and optional CRC tail.
public enum VescFirmwareIdentityDecoder {
  public static func decode(_ payload: [UInt8]) throws -> FirmwareInfo {
    guard let command = payload.first else {
      throw VescProtocolError.truncatedPayload
    }
    guard command == VescCommand.getFirmwareVersion.rawValue else {
      throw VescProtocolError.unsupportedCommand(payload.first ?? 255)
    }
    guard payload.count >= 3 else { throw VescProtocolError.truncatedPayload }

    func terminatedBytes(from start: Int, maximum: Int) throws -> ArraySlice<UInt8> {
      guard start < payload.count,
            let end = payload[start...].firstIndex(of: 0),
            end - start <= maximum
      else { throw VescProtocolError.truncatedPayload }
      return payload[start..<end]
    }

    let nameBytes = try terminatedBytes(from: 3, maximum: 64)
    let uuidStart = nameBytes.endIndex + 1
    let flagsStart = uuidStart + 12
    let firmwareNameStart = flagsStart + 8
    guard firmwareNameStart < payload.count else {
      throw VescProtocolError.truncatedPayload
    }
    let controllerUUID = Data(payload[uuidStart..<flagsStart])
    let firmwareNameBytes = try terminatedBytes(from: firmwareNameStart, maximum: 64)
    let tailLength = payload.count - (firmwareNameBytes.endIndex + 1)
    guard tailLength == 0 || tailLength == 4 else {
      throw VescProtocolError.truncatedPayload
    }

    return FirmwareInfo(
      major: payload[1],
      minor: payload[2],
      testVersion: payload[uuidStart + 13],
      hardwareName: String(decoding: nameBytes, as: UTF8.self),
      firmwareName: String(decoding: firmwareNameBytes, as: UTF8.self).nilIfEmpty,
      hardwareType: payload[uuidStart + 14],
      controllerUUID: controllerUUID
    )
  }
}

private extension String {
  var nilIfEmpty: String? { isEmpty ? nil : self }
}
