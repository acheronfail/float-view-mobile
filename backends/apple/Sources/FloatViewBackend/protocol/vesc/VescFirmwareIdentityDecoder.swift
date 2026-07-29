import Foundation

/// Length-checked owner for COMM_FW_VERSION identity and optional tail fields.
public enum VescFirmwareIdentityDecoder {
  public static func decode(_ payload: [UInt8]) throws -> FirmwareInfo {
    guard payload.first == 0, payload.count >= 4 else {
      throw VescProtocolError.unsupportedCommand(payload.first ?? 255)
    }
    let nameBytes = payload.dropFirst(3).prefix { $0 != 0 }
    var offset = min(3 + nameBytes.count + 1, payload.count)
    let controllerUUID: Data?
    if payload.count - offset >= 12 {
      controllerUUID = Data(payload[offset ..< offset + 12])
      offset += 12
    } else {
      controllerUUID = nil
    }
    let hardwareType = payload.count - offset >= 3 ? payload[offset + 2] : 0
    for _ in 0 ..< 8 where offset < payload.count {
      offset += 1
    }
    let firmwareName: String?
    if offset < payload.count {
      let bytes = payload[offset...].prefix { $0 != 0 }
      firmwareName = String(decoding: bytes, as: UTF8.self).nilIfEmpty
    } else {
      firmwareName = nil
    }
    return FirmwareInfo(
      major: payload[1],
      minor: payload[2],
      hardwareName: String(decoding: nameBytes, as: UTF8.self),
      firmwareName: firmwareName,
      hardwareType: hardwareType,
      controllerUUID: controllerUUID
    )
  }
}

private extension String {
  var nilIfEmpty: String? { isEmpty ? nil : self }
}
