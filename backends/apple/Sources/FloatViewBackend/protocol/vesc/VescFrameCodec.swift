import CVescFrame
import Foundation

public enum VescFrameCodec {
  public static let maximumPayloadBytes = 4096

  private static let maximumFrameBytes = 4102
  private static let abiVersion: UInt32 = 1

  public static func encode(_ payload: [UInt8]) throws -> [UInt8] {
    guard payload.count <= maximumPayloadBytes else { throw VescProtocolError.payloadTooLarge }
    try requireCompatibleABI()

    var frame = [UInt8](repeating: 0, count: payload.count + 6)
    var frameLength = 0
    let status = payload.withUnsafeBufferPointer { payloadBuffer in
      frame.withUnsafeMutableBufferPointer { frameBuffer in
        float_view_vesc_frame_encode(
          payloadBuffer.baseAddress,
          payloadBuffer.count,
          frameBuffer.baseAddress,
          frameBuffer.count,
          &frameLength
        )
      }
    }
    try requireSuccess(status)
    guard frameLength <= frame.count, frameLength <= maximumFrameBytes else {
      reportBridgeFailure(status: status, reason: "invalid encode output length")
      throw VescProtocolError.invalidFrame
    }
    return Array(frame.prefix(frameLength))
  }

  public static func decode(_ frame: [UInt8]) throws -> [UInt8] {
    try requireCompatibleABI()

    var payload = [UInt8](repeating: 0, count: min(maximumPayloadBytes, frame.count))
    var payloadLength = 0
    let status = frame.withUnsafeBufferPointer { frameBuffer in
      payload.withUnsafeMutableBufferPointer { payloadBuffer in
        float_view_vesc_frame_decode_exact(
          frameBuffer.baseAddress,
          frameBuffer.count,
          payloadBuffer.baseAddress,
          payloadBuffer.count,
          &payloadLength
        )
      }
    }
    try requireSuccess(status)
    guard payloadLength <= payload.count, payloadLength <= maximumPayloadBytes else {
      reportBridgeFailure(status: status, reason: "invalid decode output length")
      throw VescProtocolError.invalidFrame
    }
    return Array(payload.prefix(payloadLength))
  }

  private static func requireCompatibleABI() throws {
    let actual = float_view_vesc_frame_abi_version()
    guard actual == abiVersion else {
      NSLog(
        "Float View VESC frame codec ABI mismatch: expected %u, received %u",
        abiVersion,
        actual
      )
      throw VescProtocolError.invalidFrame
    }
  }

  private static func requireSuccess(_ status: FloatViewVescFrameStatus) throws {
    switch status {
    case UInt32(FLOAT_VIEW_VESC_FRAME_STATUS_OK):
      return
    case UInt32(FLOAT_VIEW_VESC_FRAME_STATUS_INPUT_TOO_LARGE):
      throw VescProtocolError.payloadTooLarge
    case UInt32(FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_FRAME):
      throw VescProtocolError.invalidFrame
    case UInt32(FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_CRC):
      throw VescProtocolError.invalidCRC
    default:
      reportBridgeFailure(status: status, reason: "bridge status")
      throw VescProtocolError.invalidFrame
    }
  }

  private static func reportBridgeFailure(
    status: FloatViewVescFrameStatus,
    reason: StaticString
  ) {
    NSLog("Float View VESC frame codec failure (%@): %u", String(describing: reason), status)
  }
}
