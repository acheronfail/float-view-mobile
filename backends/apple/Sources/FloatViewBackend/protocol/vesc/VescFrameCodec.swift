import CVescFrame
import Foundation

enum VescFrameStreamScan: Equatable {
  case needMore(consumed: Int)
  case frame(offset: Int, length: Int, consumed: Int)
  case failure(VescProtocolError, consumed: Int)
}

public enum VescFrameCodec {
  public static let maximumPayloadBytes = 4096

  private static let maximumFrameBytes = 4102
  private static let abiVersion: UInt32 = 2

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

  static func scanStream(_ bytes: [UInt8]) throws -> VescFrameStreamScan {
    try requireCompatibleABI()

    var frameOffset = 0
    var frameLength = 0
    var consumedLength = 0
    let status = bytes.withUnsafeBufferPointer { buffer in
      float_view_vesc_frame_stream_scan_next(
        buffer.baseAddress,
        buffer.count,
        &frameOffset,
        &frameLength,
        &consumedLength
      )
    }
    guard consumedLength <= bytes.count else {
      reportBridgeFailure(status: status, reason: "invalid scan consumption")
      throw VescProtocolError.invalidFrame
    }

    switch status {
    case UInt32(FLOAT_VIEW_VESC_FRAME_STATUS_OK):
      guard frameLength > 0,
            frameOffset <= bytes.count,
            frameLength <= bytes.count - frameOffset,
            consumedLength == frameOffset + frameLength
      else {
        reportBridgeFailure(status: status, reason: "invalid scan frame region")
        throw VescProtocolError.invalidFrame
      }
      return .frame(offset: frameOffset, length: frameLength, consumed: consumedLength)
    case UInt32(FLOAT_VIEW_VESC_FRAME_STATUS_NEED_MORE_INPUT):
      guard frameOffset == 0, frameLength == 0 else {
        reportBridgeFailure(status: status, reason: "invalid incomplete scan result")
        throw VescProtocolError.invalidFrame
      }
      return .needMore(consumed: consumedLength)
    case UInt32(FLOAT_VIEW_VESC_FRAME_STATUS_INPUT_TOO_LARGE):
      guard frameOffset == 0, frameLength == 0, consumedLength > 0 else {
        reportBridgeFailure(status: status, reason: "invalid oversized scan result")
        throw VescProtocolError.invalidFrame
      }
      return .failure(.payloadTooLarge, consumed: consumedLength)
    default:
      reportBridgeFailure(status: status, reason: "stream scan bridge status")
      throw VescProtocolError.invalidFrame
    }
  }

  static func requireCompatibleABI(
    _ actual: UInt32 = float_view_vesc_frame_abi_version()
  ) throws {
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
