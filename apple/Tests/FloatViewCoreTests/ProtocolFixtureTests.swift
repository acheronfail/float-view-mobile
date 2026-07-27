import Foundation
import Testing
@testable import FloatViewProtocol

private struct Fixture: Decodable {
  let frameHex: String
  let chunkSizes: [Int]
}

private func fixture(_ relativePath: String) throws -> Fixture {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0..<4 { root.deleteLastPathComponent() }
  let data = try Data(contentsOf: root.appending(path: relativePath))
  return try JSONDecoder().decode(Fixture.self, from: data)
}

private func bytes(_ hex: String) -> [UInt8] {
  stride(from: 0, to: hex.count, by: 2).map {
    let start = hex.index(hex.startIndex, offsetBy: $0)
    return UInt8(hex[start..<hex.index(start, offsetBy: 2)], radix: 16)!
  }
}

@Test func firmwareFixtureDecodesAcrossChunks() throws {
  let value = try fixture("fixtures/vesc/firmware-6.06.json")
  let frame = bytes(value.frameHex)
  var stream = VescFrameStreamDecoder()
  var offset = 0
  var payloads: [[UInt8]] = []
  for count in value.chunkSizes {
    payloads += try stream.append(frame[offset..<(offset + count)])
    offset += count
  }
  let info = try VescPayloadDecoder.firmware(#require(payloads.first))
  #expect(info == FirmwareInfo(major: 6, minor: 6, hardwareName: "THOR300"))
}

@Test func realtimeAndRefloatFixturesDecode() throws {
  let realtime = try VescFrameCodec.decode(bytes(fixture("fixtures/vesc/realtime-values.json").frameHex))
  let values = try VescPayloadDecoder.realtime(realtime)
  #expect(values.controllerTempC == 25)
  #expect(values.motorTempC == 23)
  #expect(values.motorCurrentA == 12.34)
  #expect(values.batteryCurrentA == -5)
  #expect(values.dutyCycle == 0.5)
  #expect(values.erpm == 10_000)
  #expect(values.packVoltageV == 51.6)
  #expect(values.faultCode == 0)

  let refloat = try VescFrameCodec.decode(bytes(fixture("fixtures/refloat/rtdata-1.2.json").frameHex))
  #expect(
    try VescPayloadDecoder.refloatRealtime(refloat)
      == RefloatRealtime(appId: 101, packetId: 1, rideState: 0, footpadState: 0)
  )
}

@Test func corruptCRCIsRejected() {
  let corrupt = bytes("020b00060654484f52333030002ad903")
  #expect(throws: VescProtocolError.invalidCRC) {
    try VescFrameCodec.decode(corrupt)
  }
}

@Test func longFramesNoiseMultiplePacketsAndTruncation() throws {
  let longPayload = [UInt8](arrayLiteral: 4) + [UInt8](repeating: 0, count: 255)
  let longFrame = try VescFrameCodec.encode(longPayload)
  #expect(longFrame.first == 3)
  #expect(try VescFrameCodec.decode(longFrame) == longPayload)

  let shortFrame = try VescFrameCodec.encode([0, 6, 6, 0])
  var stream = VescFrameStreamDecoder()
  let combined = [0xff, 0x01] + shortFrame + shortFrame
  let packets = try stream.append(combined)
  #expect(packets == [[0, 6, 6, 0], [0, 6, 6, 0]])

  var truncated = VescFrameStreamDecoder()
  #expect(try truncated.append(shortFrame.dropLast()).isEmpty)
  #expect(try truncated.append(shortFrame.suffix(1)) == [[0, 6, 6, 0]])
}
