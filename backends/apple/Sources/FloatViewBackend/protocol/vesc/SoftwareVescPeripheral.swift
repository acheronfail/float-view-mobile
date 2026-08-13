import Foundation
#if canImport(RefloatConfigurationFixtures)
  import RefloatConfigurationFixtures
#endif

/// Minimal deterministic direct VESC used only by software-host conformance.
public final class SoftwareVescPeripheral: @unchecked Sendable {
  private let lock = NSLock()
  private var currentConfiguration: [UInt8]

  public init() {
    currentConfiguration = Self.fixture("values/1.0.0-current.bin")
  }

  public func response(to frame: Data) -> Data? {
    guard let request = try? VescFrameCodec.decode(Array(frame)) else { return nil }
    let payload: [UInt8]?
    switch request {
    case [VescCommand.getFirmwareVersion.rawValue]:
      payload = Self.firmware
    case [VescCommand.pingCAN.rawValue]:
      payload = [VescCommand.pingCAN.rawValue, Self.bmsCanID]
    case [VescCommand.forwardCAN.rawValue, Self.bmsCanID, VescCommand.getFirmwareVersion.rawValue]:
      payload = Self.bmsFirmware
    case [VescCommand.forwardCAN.rawValue, Self.bmsCanID, VescCommand.bmsGetValues.rawValue]:
      payload = Self.bmsValues
    case RefloatWire.request(
      .info,
      parameters: [RefloatWire.infoProtocolVersion, RefloatWire.infoFlags]
    ):
      payload = RefloatWire.request(.info, parameters: [10])
    case RefloatWire.request(
      .getAllData,
      parameters: [RefloatWire.legacyTelemetryDetailLevel]
    ):
      payload = Self.telemetry
    default:
      payload = configurationResponse(request)
    }
    guard let payload else { return nil }
    return try? Data(VescFrameCodec.encode(payload))
  }

  private func configurationResponse(_ request: [UInt8]) -> [UInt8]? {
    guard let command = request.first else { return nil }
    lock.lock()
    defer { lock.unlock() }
    switch command {
    case VescCommand.getCustomConfigurationXML.rawValue:
      guard request.count == 10, request[1] == 0,
            let length = Self.i32(request, 2), length > 0, length <= 400,
            let offset = Self.i32(request, 6), offset >= 0,
            offset < Self.configurationSchema.count
      else { return nil }
      let end = min(Self.configurationSchema.count, offset + length)
      return [command, 0] + Self.i32Bytes(Self.configurationSchema.count) + Self.i32Bytes(offset)
        + Array(Self.configurationSchema[offset ..< end])
    case VescCommand.getCustomConfiguration.rawValue:
      guard request == [command, 0] else { return nil }
      return [command, 0] + currentConfiguration
    case VescCommand.getCustomConfigurationDefault.rawValue:
      guard request == [command, 0] else { return nil }
      return [command, 0] + Self.defaultConfiguration
    case VescCommand.setCustomConfiguration.rawValue:
      guard request.count > 2, request[1] == 0,
            Array(request.dropFirst(2)) == Self.acceptedConfigurationWrite
      else { return nil }
      currentConfiguration = Self.expectedConfigurationReadback
      return [command]
    default:
      return nil
    }
  }

  private static let firmware: [UInt8] = {
    var bytes: [UInt8] = [VescCommand.getFirmwareVersion.rawValue, 6, 6]
    bytes += Array("THOR300".utf8)
    bytes.append(0)
    bytes += [UInt8](repeating: 0, count: 12)
    bytes += [0, 0, 0, 1, 0, 0, 0, 0]
    bytes += Array("VESC".utf8)
    bytes.append(0)
    return bytes
  }()

  private static let bmsCanID: UInt8 = 42
  private static let configurationSchema = fixture("schemas/1.0.0.qcompress")
  private static let defaultConfiguration = fixture("values/1.0.0-default.bin")
  private static let acceptedConfigurationWrite = fixture("values/1.0.0-write.bin")
  private static let expectedConfigurationReadback = fixture("values/1.0.0-readback.bin")
  private static let bmsFirmware = hex(
    "00060056424d530000112233445566778899aabb00000101"
  )
  private static let bmsValues = hex(
    "6003010b0003412140ff43a158ff445120000004e20000f424031068105e10540100010209f60a410abe09ab14820b6d036b03b62a428000004480000042000000440000002794014f4b00"
  )

  private static let telemetry: [UInt8] = {
    var payload = [UInt8](repeating: 0, count: 55)
    payload[0 ... 3] = [
      VescCommand.customAppData.rawValue,
      RefloatWire.packageID,
      RefloatCommand.getAllData.rawValue,
      3,
    ]
    payload[23 ... 24] = [2, 4]
    // The software host advertises legacy Refloat 1.0, whose decoder reads
    // the ride state from the low nibble at offset 10.
    payload[10] = 1
    payload[33] = 128
    payload[39] = 60
    payload[40] = 56
    return payload
  }()

  private static func hex(_ value: String) -> [UInt8] {
    stride(from: 0, to: value.count, by: 2).map { offset in
      let start = value.index(value.startIndex, offsetBy: offset)
      let end = value.index(start, offsetBy: 2)
      return UInt8(value[start ..< end], radix: 16)!
    }
  }

  private static func fixture(_ path: String) -> [UInt8] {
    #if canImport(RefloatConfigurationFixtures)
      let data = RefloatConfigurationFixtureResources.data(path)
    #else
      let data = Bundle.main.resourceURL.flatMap {
        try? Data(
          contentsOf: $0.appending(path: path),
          options: [.mappedIfSafe]
        )
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
      UInt8(raw >> 24),
      UInt8(truncatingIfNeeded: raw >> 16),
      UInt8(truncatingIfNeeded: raw >> 8),
      UInt8(truncatingIfNeeded: raw),
    ]
  }
}
