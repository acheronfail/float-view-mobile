import CVescFrame
@testable import FloatViewBackend
import Testing

@Test func rustFrameCodecMatchesImmutableBoundaries() throws {
  #expect(float_view_vesc_frame_abi_version() == 2)
  #expect(throws: VescProtocolError.invalidFrame) {
    try VescFrameCodec.requireCompatibleABI(1)
  }

  let shortPayload: [UInt8] = [0, 6, 6, 84, 72, 79, 82, 51, 48, 48, 0]
  let shortFrame: [UInt8] = [2, 11, 0, 6, 6, 84, 72, 79, 82, 51, 48, 48, 0, 42, 216, 3]
  #expect(try VescFrameCodec.encode(shortPayload) == shortFrame)
  #expect(try VescFrameCodec.decode(shortFrame) == shortPayload)

  let longPayload = [UInt8](arrayLiteral: 4) + [UInt8](repeating: 0, count: 255)
  let longFrame = try VescFrameCodec.encode(longPayload)
  #expect(longFrame.count == 262)
  #expect(Array(longFrame.prefix(3)) == [3, 1, 0])
  #expect(Array(longFrame.suffix(3)) == [0xAA, 0xE9, 3])
  #expect(try VescFrameCodec.decode(longFrame) == longPayload)

  let maximumPayload = [UInt8](repeating: 0x33, count: VescFrameCodec.maximumPayloadBytes)
  let maximumFrame = try VescFrameCodec.encode(maximumPayload)
  #expect(maximumFrame.count == 4102)
  #expect(try VescFrameCodec.decode(maximumFrame) == maximumPayload)

  #expect(throws: VescProtocolError.payloadTooLarge) {
    try VescFrameCodec.encode(maximumPayload + [0])
  }
}

@Test func rustExactDecodeRejectsEveryMalformedClass() throws {
  let valid = try VescFrameCodec.encode([1, 2, 3])
  #expect(throws: VescProtocolError.invalidFrame) { try VescFrameCodec.decode([]) }
  #expect(throws: VescProtocolError.invalidFrame) { try VescFrameCodec.decode([4, 0, 0, 0, 3]) }
  #expect(throws: VescProtocolError.invalidFrame) { try VescFrameCodec.decode([2, 0, 0, 0]) }
  #expect(throws: VescProtocolError.invalidFrame) { try VescFrameCodec.decode(valid + [0]) }

  var invalidFooter = valid
  invalidFooter[invalidFooter.count - 1] = 4
  #expect(throws: VescProtocolError.invalidFrame) { try VescFrameCodec.decode(invalidFooter) }

  var invalidCRC = valid
  invalidCRC[invalidCRC.count - 3] ^= 1
  #expect(throws: VescProtocolError.invalidCRC) { try VescFrameCodec.decode(invalidCRC) }
  #expect(throws: VescProtocolError.payloadTooLarge) {
    try VescFrameCodec.decode([3, 0x10, 0x01, 0, 0, 3])
  }
  #expect(throws: VescProtocolError.payloadTooLarge) {
    try VescFrameCodec.decode([UInt8](repeating: 0, count: 4103))
  }
}

@Test func rustStreamScanAndExactDecodeOwnAppleFraming() throws {
  let shortPayload: [UInt8] = [0, 6, 6, 0]
  let shortFrame = try VescFrameCodec.encode(shortPayload)
  let longPayload = [UInt8](repeating: 0xA5, count: 256)
  let longFrame = try VescFrameCodec.encode(longPayload)

  var fragmented = VescFrameStreamDecoder()
  for byte in shortFrame.dropLast() {
    #expect(try fragmented.append([byte]).isEmpty)
  }
  #expect(try fragmented.append(shortFrame.suffix(1)) == [shortPayload])

  var splitLongHeader = VescFrameStreamDecoder()
  #expect(try splitLongHeader.append(longFrame.prefix(1)).isEmpty)
  #expect(try splitLongHeader.append(longFrame[1 ..< 2]).isEmpty)
  #expect(try splitLongHeader.append(longFrame.dropFirst(2)) == [longPayload])

  var fixtureChunks = VescFrameStreamDecoder()
  var chunkOffset = 0
  for chunkCount in [1, 2, 17, 100, 142] {
    let nextOffset = chunkOffset + chunkCount
    let payloads = try fixtureChunks.append(longFrame[chunkOffset ..< nextOffset])
    #expect(payloads == (nextOffset == longFrame.count ? [longPayload] : []))
    chunkOffset = nextOffset
  }

  var combined = VescFrameStreamDecoder()
  #expect(
    try combined.append([0xFF, 0, 1] + shortFrame + longFrame + shortFrame)
      == [shortPayload, longPayload, shortPayload]
  )

  var invalidCRC = shortFrame
  invalidCRC[invalidCRC.count - 3] ^= 1
  var failed = VescFrameStreamDecoder()
  #expect(throws: VescProtocolError.invalidCRC) {
    try failed.append(invalidCRC + shortFrame)
  }
  #expect(try failed.append([]) == [shortPayload])

  var invalidFooter = shortFrame
  invalidFooter[invalidFooter.count - 1] = 4
  var badFooter = VescFrameStreamDecoder()
  #expect(throws: VescProtocolError.invalidFrame) {
    try badFooter.append(invalidFooter + shortFrame)
  }
  #expect(try badFooter.append([]) == [shortPayload])

  var oversizedDeclaration = VescFrameStreamDecoder()
  #expect(throws: VescProtocolError.payloadTooLarge) {
    try oversizedDeclaration.append([0xFF, 3, 0x10, 0x01])
  }
  #expect(try oversizedDeclaration.append(shortFrame) == [shortPayload])

  var overflow = VescFrameStreamDecoder()
  #expect(try overflow.append([3, 0]).isEmpty)
  #expect(throws: VescProtocolError.payloadTooLarge) {
    try overflow.append([UInt8](repeating: 0, count: 8192))
  }
  #expect(try overflow.append(shortFrame) == [shortPayload])
}

@Test func checkpointSwiftStreamReferenceMatchesRustProductionScanner() throws {
  let firstPayload: [UInt8] = [1, 2, 3]
  let secondPayload = [UInt8](repeating: 0xA5, count: 256)
  let firstFrame = try VescFrameCodec.encode(firstPayload)
  let secondFrame = try VescFrameCodec.encode(secondPayload)
  let chunks = [
    [UInt8](arrayLiteral: 0xFF, 0),
    Array(firstFrame.prefix(1)),
    Array(firstFrame.dropFirst()) + Array(secondFrame.prefix(2)),
    Array(secondFrame.dropFirst(2)),
  ]

  var production = VescFrameStreamDecoder()
  var reference = CheckpointSwiftVescStreamReference()
  for chunk in chunks {
    let productionPayloads = try production.append(chunk)
    let referencePayloads = try reference.append(chunk)
    #expect(productionPayloads == referencePayloads)
  }
}

/// Temporary test-only checkpoint parser. It is never linked into either host
/// and is deleted after the representative physical iOS cutover succeeds.
private struct CheckpointSwiftVescStreamReference {
  private var buffer: [UInt8] = []

  mutating func append(_ bytes: some Sequence<UInt8>) throws -> [[UInt8]] {
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
      let headerBytes = first == 2 ? 2 : 3
      guard buffer.count >= headerBytes else { return payloads }
      let payloadBytes = first == 2 ? Int(buffer[1]) : Int(buffer[1]) << 8 | Int(buffer[2])
      guard payloadBytes <= VescFrameCodec.maximumPayloadBytes else {
        buffer.removeFirst()
        throw VescProtocolError.payloadTooLarge
      }
      let frameBytes = headerBytes + payloadBytes + 3
      guard buffer.count >= frameBytes else { return payloads }
      let frame = Array(buffer.prefix(frameBytes))
      buffer.removeFirst(frameBytes)
      try payloads.append(VescFrameCodec.decode(frame))
    }
  }
}
