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
    case RefloatWire.request(
      .info,
      parameters: [RefloatWire.infoProtocolVersion, RefloatWire.infoFlags]
    ):
      payload = RefloatWire.request(.info, parameters: [10])
    case RefloatWire.request(.getAllData):
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

  private static let telemetry: [UInt8] = {
    var payload = [UInt8](repeating: 0, count: 55)
    payload[0...3] = [
      VescCommand.customAppData.rawValue,
      RefloatWire.packageID,
      RefloatCommand.getAllData.rawValue,
      3,
    ]
    payload[23...24] = [2, 4]
    payload[33] = 128
    payload[39] = 60
    payload[40] = 56
    return payload
  }()
}
