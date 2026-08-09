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
      while let first = buffer.first, first != 2, first != 3 {
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
