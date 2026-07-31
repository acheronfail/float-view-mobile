import Foundation

public enum VescProtocolError: Error, Equatable {
  case invalidFrame
  case invalidCRC
  case payloadTooLarge
  case unsupportedCommand(UInt8)
  case truncatedPayload
}

public enum VescCRC {
  public static func checksum(_ bytes: some Sequence<UInt8>) -> UInt16 {
    var crc: UInt16 = 0
    for byte in bytes {
      crc ^= UInt16(byte) << 8
      for _ in 0 ..< 8 {
        crc = (crc & 0x8000) != 0 ? (crc << 1) ^ 0x1021 : crc << 1
      }
    }
    return crc
  }
}

public enum VescFrameCodec {
  public static let maximumPayloadBytes = 4096

  public static func encode(_ payload: [UInt8]) throws -> [UInt8] {
    guard payload.count <= maximumPayloadBytes else { throw VescProtocolError.payloadTooLarge }
    var frame: [UInt8]
    if payload.count <= 255 {
      frame = [2, UInt8(payload.count)]
    } else {
      frame = [3, UInt8(payload.count >> 8), UInt8(payload.count & 0xFF)]
    }
    frame.append(contentsOf: payload)
    let crc = VescCRC.checksum(payload)
    frame.append(UInt8(crc >> 8))
    frame.append(UInt8(crc & 0xFF))
    frame.append(3)
    return frame
  }

  public static func decode(_ frame: [UInt8]) throws -> [UInt8] {
    guard let first = frame.first else { throw VescProtocolError.invalidFrame }
    let headerCount: Int
    let payloadCount: Int
    switch first {
    case 2:
      guard frame.count >= 5 else { throw VescProtocolError.invalidFrame }
      headerCount = 2
      payloadCount = Int(frame[1])
    case 3:
      guard frame.count >= 6 else { throw VescProtocolError.invalidFrame }
      headerCount = 3
      payloadCount = Int(frame[1]) << 8 | Int(frame[2])
    default:
      throw VescProtocolError.invalidFrame
    }
    guard payloadCount <= maximumPayloadBytes else { throw VescProtocolError.payloadTooLarge }
    let expectedCount = headerCount + payloadCount + 3
    guard frame.count == expectedCount, frame.last == 3 else {
      throw VescProtocolError.invalidFrame
    }
    let payload = Array(frame[headerCount ..< (headerCount + payloadCount)])
    let received = UInt16(frame[headerCount + payloadCount]) << 8
      | UInt16(frame[headerCount + payloadCount + 1])
    guard VescCRC.checksum(payload) == received else { throw VescProtocolError.invalidCRC }
    return payload
  }
}

public struct VescFrameStreamDecoder: Sendable {
  private var buffer: [UInt8] = []
  public init() {}

  public mutating func append(_ bytes: some Sequence<UInt8>) throws -> [[UInt8]] {
    buffer.append(contentsOf: bytes)
    guard buffer.count <= VescFrameCodec.maximumPayloadBytes * 2 else {
      buffer.removeAll(keepingCapacity: true)
      throw VescProtocolError.payloadTooLarge
    }
    var payloads: [[UInt8]] = []
    while true {
      while let first = buffer.first, first != 2 && first != 3 {
        buffer.removeFirst()
      }
      guard let first = buffer.first else { return payloads }
      let headerCount = first == 2 ? 2 : 3
      guard buffer.count >= headerCount else { return payloads }
      let count = first == 2 ? Int(buffer[1]) : Int(buffer[1]) << 8 | Int(buffer[2])
      guard count <= VescFrameCodec.maximumPayloadBytes else {
        buffer.removeFirst()
        throw VescProtocolError.payloadTooLarge
      }
      let frameCount = headerCount + count + 3
      guard buffer.count >= frameCount else { return payloads }
      let frame = Array(buffer.prefix(frameCount))
      buffer.removeFirst(frameCount)
      try payloads.append(VescFrameCodec.decode(frame))
    }
  }
}
