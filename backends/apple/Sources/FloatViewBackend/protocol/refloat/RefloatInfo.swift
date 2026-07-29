import Foundation

enum RefloatInfoDecoder {
  static func decodeVersion(_ payload: [UInt8]) throws -> String {
    let responseVersion = payload[3]
    if responseVersion == 10 { return "1.0" }
    if responseVersion > 10 {
      return "\(responseVersion / 10).\(responseVersion % 10)"
    }
    guard responseVersion == 2, payload.count >= 61 else {
      throw VescProtocolError.truncatedPayload
    }
    return "\(payload[25]).\(payload[26]).\(payload[27])"
  }
}
