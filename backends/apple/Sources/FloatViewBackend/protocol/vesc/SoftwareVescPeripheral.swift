import Foundation

/// Minimal deterministic direct VESC used only by software-host conformance.
public struct SoftwareVescPeripheral: Sendable {
  public init() {}

  public func response(to frame: Data) -> Data? {
    guard let request = try? VescFrameCodec.decode(Array(frame)) else { return nil }
    let payload: [UInt8]?
    switch request {
    case [0]:
      payload = Self.firmware
    case [36, 101, 0, 2, 0]:
      payload = [36, 101, 0, 10]
    case [36, 101, 10]:
      payload = Self.telemetry
    default:
      payload = nil
    }
    guard let payload else { return nil }
    return try? Data(VescFrameCodec.encode(payload))
  }

  private static let firmware: [UInt8] =
    [0, 6, 6] + Array("THOR300".utf8) + [0] + [UInt8](repeating: 0, count: 20)

  private static let telemetry: [UInt8] = {
    var payload = [UInt8](repeating: 0, count: 55)
    payload[0...3] = [36, 101, 10, 3]
    payload[23...24] = [2, 4]
    payload[33] = 128
    payload[39] = 60
    payload[40] = 56
    return payload
  }()
}
