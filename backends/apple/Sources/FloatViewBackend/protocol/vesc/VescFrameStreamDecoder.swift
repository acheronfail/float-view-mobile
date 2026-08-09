public struct VescFrameStreamDecoder: Sendable {
  private var buffer: [UInt8] = []

  public init() {}

  public mutating func append(_ bytes: some Sequence<UInt8>) throws -> [[UInt8]] {
    buffer.append(contentsOf: bytes)
    var payloads: [[UInt8]] = []
    while true {
      switch try VescFrameCodec.scanStream(buffer) {
      case let .needMore(consumed):
        buffer.removeFirst(consumed)
        return payloads
      case let .frame(offset, length, consumed):
        let frame = Array(buffer[offset ..< offset + length])
        buffer.removeFirst(consumed)
        try payloads.append(VescFrameCodec.decode(frame))
      case let .failure(error, consumed):
        if consumed == buffer.count {
          buffer.removeAll(keepingCapacity: true)
        } else {
          buffer.removeFirst(consumed)
        }
        throw error
      }
    }
  }
}
