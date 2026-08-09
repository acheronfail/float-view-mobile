import CVescFrame
@testable import FloatViewBackend
import Testing

@Test func rustFrameCodecMatchesImmutableBoundaries() throws {
  #expect(float_view_vesc_frame_abi_version() == 1)

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

@Test func swiftStreamBoundariesAlwaysFinishThroughRustExactDecode() throws {
  let shortPayload: [UInt8] = [0, 6, 6, 0]
  let shortFrame = try VescFrameCodec.encode(shortPayload)
  let longPayload = [UInt8](repeating: 0xA5, count: 256)
  let longFrame = try VescFrameCodec.encode(longPayload)

  var fragmented = VescFrameStreamDecoder()
  for byte in shortFrame.dropLast() {
    #expect(try fragmented.append([byte]).isEmpty)
  }
  #expect(try fragmented.append(shortFrame.suffix(1)) == [shortPayload])

  var combined = VescFrameStreamDecoder()
  #expect(
    try combined.append([0xFF, 0, 1] + shortFrame + longFrame + shortFrame)
      == [shortPayload, longPayload, shortPayload]
  )

  var invalidCRC = shortFrame
  invalidCRC[invalidCRC.count - 3] ^= 1
  var failed = VescFrameStreamDecoder()
  #expect(throws: VescProtocolError.invalidCRC) { try failed.append(invalidCRC) }

  var overflow = VescFrameStreamDecoder()
  #expect(try overflow.append([3, 0]).isEmpty)
  #expect(throws: VescProtocolError.payloadTooLarge) {
    try overflow.append([UInt8](repeating: 0, count: 8192))
  }
  #expect(try overflow.append(shortFrame) == [shortPayload])
}
