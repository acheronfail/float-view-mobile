import Foundation

/// Minimal deterministic direct VESC used only by software-host conformance.
public struct SoftwareVescPeripheral: Sendable {
  public init() {}

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
      payload = nil
    }
    guard let payload else { return nil }
    return try? Data(VescFrameCodec.encode(payload))
  }

  private static let firmware: [UInt8] = {
    var bytes: [UInt8] = [VescCommand.getFirmwareVersion.rawValue, 6, 6]
    bytes += Array("THOR300".utf8)
    bytes.append(0)
    bytes += [UInt8](repeating: 0, count: 12)
    bytes += [0, 0, 0, 0, 0, 0, 0, 0]
    bytes += Array("VESC".utf8)
    bytes.append(0)
    return bytes
  }()

  private static let bmsCanID: UInt8 = 42
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
}
