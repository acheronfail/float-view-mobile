import Foundation
#if canImport(RefloatConfigurationFixtures)
  import RefloatConfigurationFixtures
#endif

/// Bounded simulator-only byte fixture; production protocol behavior lives in Rust.
public final class SoftwareVescPeripheral: @unchecked Sendable {
  private let lock = NSLock()
  private var currentConfiguration: [UInt8]

  public init() {
    currentConfiguration = Self.fixture("values/1.0.0-current.bin")
  }

  public func response(to frame: Data) -> Data? {
    guard let request = SoftwareVescFrame.decode(Array(frame)) else { return nil }
    let payload: [UInt8]?
    switch request {
    case [Self.getFirmwareVersion]: payload = Self.firmware
    case [Self.pingCAN]: payload = [Self.pingCAN, Self.bmsCanID]
    case [Self.forwardCAN, Self.bmsCanID, Self.getFirmwareVersion]: payload = Self.bmsFirmware
    case [Self.forwardCAN, Self.bmsCanID, Self.bmsGetValues]: payload = Self.bmsValues
    case Self.refloatRequest(Self.info, Self.infoProtocolVersion, Self.infoFlags):
      payload = Self.refloatRequest(Self.info, 10)
    case Self.refloatRequest(Self.getAllData, Self.legacyTelemetryDetailLevel):
      payload = Self.telemetry
    default: payload = configurationResponse(request)
    }
    guard let payload, let frame = SoftwareVescFrame.encode(payload) else { return nil }
    return Data(frame)
  }

  private func configurationResponse(_ request: [UInt8]) -> [UInt8]? {
    guard let command = request.first else { return nil }
    lock.lock()
    defer { lock.unlock() }
    switch command {
    case Self.getCustomConfigurationXML:
      guard request.count == 10, request[1] == 0,
            let length = Self.i32(request, 2), length > 0, length <= 400,
            let offset = Self.i32(request, 6), offset >= 0,
            offset < Self.configurationSchema.count
      else { return nil }
      let end = min(Self.configurationSchema.count, offset + length)
      return [command, 0] + Self.i32Bytes(Self.configurationSchema.count) + Self.i32Bytes(offset)
        + Array(Self.configurationSchema[offset ..< end])
    case Self.getCustomConfiguration:
      guard request == [command, 0] else { return nil }
      return [command, 0] + currentConfiguration
    case Self.getCustomConfigurationDefault:
      guard request == [command, 0] else { return nil }
      return [command, 0] + Self.defaultConfiguration
    case Self.setCustomConfiguration:
      guard request.count > 2, request[1] == 0,
            Array(request.dropFirst(2)) == Self.acceptedConfigurationWrite
      else { return nil }
      currentConfiguration = Self.expectedConfigurationReadback
      return [command]
    default: return nil
    }
  }

  private static let getFirmwareVersion: UInt8 = 0
  private static let forwardCAN: UInt8 = 34
  private static let customAppData: UInt8 = 36
  private static let pingCAN: UInt8 = 62
  private static let getCustomConfigurationXML: UInt8 = 92
  private static let getCustomConfiguration: UInt8 = 93
  private static let getCustomConfigurationDefault: UInt8 = 94
  private static let setCustomConfiguration: UInt8 = 95
  private static let bmsGetValues: UInt8 = 96
  private static let bmsCanID: UInt8 = 42
  private static let packageID: UInt8 = 101
  private static let info: UInt8 = 0
  private static let getAllData: UInt8 = 10
  private static let infoProtocolVersion: UInt8 = 2
  private static let infoFlags: UInt8 = 0
  private static let legacyTelemetryDetailLevel: UInt8 = 4

  private static func refloatRequest(_ command: UInt8, _ parameters: UInt8...) -> [UInt8] {
    [customAppData, packageID, command] + parameters
  }

  private static let firmware: [UInt8] = {
    var bytes: [UInt8] = [getFirmwareVersion, 6, 6]
    bytes += Array("THOR300".utf8) + [0] + [UInt8](repeating: 0, count: 12)
    bytes += [0, 0, 0, 1, 0, 0, 0, 0] + Array("VESC".utf8) + [0]
    return bytes
  }()

  private static let configurationSchema = fixture("schemas/1.0.0.qcompress")
  private static let defaultConfiguration = fixture("values/1.0.0-default.bin")
  private static let acceptedConfigurationWrite = fixture("values/1.0.0-write.bin")
  private static let expectedConfigurationReadback = fixture("values/1.0.0-readback.bin")
  private static let bmsFirmware = hex("00060056424d530000112233445566778899aabb00000101")
  private static let bmsValues = hex(
    "6003010b0003412140ff43a158ff445120000004e20000f424031068105e10540100010209f60a410abe09ab14820b6d036b03b62a428000004480000042000000440000002794014f4b00"
  )
  private static let telemetry: [UInt8] = {
    var payload = [UInt8](repeating: 0, count: 55)
    payload[0 ... 3] = [customAppData, packageID, getAllData, 3]
    payload[23 ... 24] = [2, 4]
    payload[10] = 1
    payload[33] = 128
    payload[39] = 60
    payload[40] = 56
    return payload
  }()

  private static func hex(_ value: String) -> [UInt8] {
    stride(from: 0, to: value.count, by: 2).map { offset in
      let start = value.index(value.startIndex, offsetBy: offset)
      return UInt8(value[start ..< value.index(start, offsetBy: 2)], radix: 16)!
    }
  }

  private static func fixture(_ path: String) -> [UInt8] {
    #if canImport(RefloatConfigurationFixtures)
      let data = RefloatConfigurationFixtureResources.data(path)
    #else
      let data = Bundle.main.resourceURL.flatMap {
        try? Data(contentsOf: $0.appending(path: path), options: [.mappedIfSafe])
      }
    #endif
    guard let data, !data.isEmpty, data.count <= 32_768 else {
      preconditionFailure("Missing bounded software Refloat configuration fixture")
    }
    return Array(data)
  }

  private static func i32(_ bytes: [UInt8], _ offset: Int) -> Int? {
    guard offset + 4 <= bytes.count else { return nil }
    let value = bytes[offset ..< offset + 4].reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
    return value <= UInt32(Int32.max) ? Int(value) : nil
  }

  private static func i32Bytes(_ value: Int) -> [UInt8] {
    let raw = UInt32(value)
    return [
      UInt8(raw >> 24), UInt8(truncatingIfNeeded: raw >> 16),
      UInt8(truncatingIfNeeded: raw >> 8), UInt8(truncatingIfNeeded: raw),
    ]
  }
}

private enum SoftwareVescFrame {
  static let maximumPayloadBytes = 4_096

  static func encode(_ payload: [UInt8]) -> [UInt8]? {
    guard payload.count <= maximumPayloadBytes else { return nil }
    let header: [UInt8] = payload.count <= 255
      ? [2, UInt8(payload.count)]
      : [3, UInt8(payload.count >> 8), UInt8(payload.count & 0xFF)]
    let crc = checksum(payload)
    return header + payload + [UInt8(crc >> 8), UInt8(crc & 0xFF), 3]
  }

  static func decode(_ frame: [UInt8]) -> [UInt8]? {
    guard let start = frame.first else { return nil }
    let headerSize: Int
    let payloadSize: Int
    switch start {
    case 2:
      guard frame.count >= 5 else { return nil }
      headerSize = 2
      payloadSize = Int(frame[1])
    case 3:
      guard frame.count >= 6 else { return nil }
      headerSize = 3
      payloadSize = Int(frame[1]) << 8 | Int(frame[2])
    default: return nil
    }
    guard payloadSize <= maximumPayloadBytes,
          frame.count == headerSize + payloadSize + 3,
          frame.last == 3
    else { return nil }
    let payload = Array(frame[headerSize ..< headerSize + payloadSize])
    let received = UInt16(frame[headerSize + payloadSize]) << 8
      | UInt16(frame[headerSize + payloadSize + 1])
    return checksum(payload) == received ? payload : nil
  }

  private static func checksum(_ bytes: [UInt8]) -> UInt16 {
    var crc: UInt16 = 0
    for byte in bytes {
      crc ^= UInt16(byte) << 8
      for _ in 0 ..< 8 {
        crc = crc & 0x8000 != 0 ? (crc << 1) ^ 0x1021 : crc << 1
      }
    }
    return crc
  }
}
