import Foundation
import Testing
@testable import FloatViewBackend

private struct Fixture: Decodable {
  let frameHex: String
  let chunkSizes: [Int]
}

private struct MultiCaseFixture: Decodable {
  struct Case: Decodable {
    let name: String
    let frameHex: String
    let chunkSizes: [Int]
  }

  let cases: [Case]
}

private func fixture(_ relativePath: String) throws -> Fixture {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0..<4 { root.deleteLastPathComponent() }
  let data = try Data(contentsOf: root.appending(path: relativePath))
  return try JSONDecoder().decode(Fixture.self, from: data)
}

private func multiCaseFixture(_ relativePath: String) throws -> MultiCaseFixture {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0..<4 { root.deleteLastPathComponent() }
  let data = try Data(contentsOf: root.appending(path: relativePath))
  return try JSONDecoder().decode(MultiCaseFixture.self, from: data)
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

@Test func composedThorFirmwareVariantsDecode() throws {
  for (version, minor) in [("6.02", 2), ("6.05", 5), ("6.06", 6)] {
    let value = try fixture("fixtures/vesc/firmware-thor300-\(version).json")
    let payload = try VescFrameCodec.decode(bytes(value.frameHex))
    let info = try VescPayloadDecoder.firmware(payload)
    #expect(
      info == FirmwareInfo(
        major: 6,
        minor: UInt8(minor),
        hardwareName: "THOR300",
        hardwareType: 0
      )
    )
  }
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
  #expect(values.fault == VescFault(code: 0))

  let faultFrame = try VescFrameCodec.decode(
    bytes(fixture("fixtures/vesc/realtime-values-over-voltage.json").frameHex)
  )
  let faultValues = try VescPayloadDecoder.realtime(faultFrame)
  #expect(faultValues.faultCode == 1)
  #expect(faultValues.fault == VescFault(code: 1))
  #expect(faultValues.fault.kind.rawValue == "over_voltage")
  #expect(VescFault(code: 255).kind == .unknown)
  #expect(VescFault(code: 255).code == 255)

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

@Test func canDiscoveryAndForwardedFirmwarePreserveCanIdentity() throws {
  let service = VescProtocolService()
  #expect(try VescFrameCodec.decode(Array(service.canPingRequest())) == [62])
  #expect(
    try service.receive(Data(VescFrameCodec.encode([62, 42, 7, 42])))
      == [.canDevices([7, 42])]
  )

  #expect(
    try VescFrameCodec.decode(Array(service.firmwareRequest(canID: 42))) == [34, 42, 0]
  )
  let firmwareFrame = bytes(try fixture("fixtures/vesc/firmware-6.06.json").frameHex)
  let messages = try service.receive(Data(firmwareFrame))
  let firmware = try #require(messages.first)
  #expect(
    firmware ==
      .firmware(
        FirmwareRecord(
          major: 6,
          minor: 6,
          hardwareName: "THOR300",
          firmwareName: nil,
          hardwareType: 0,
          canID: 42
        )
      )
  )
  #expect(
    try VescFrameCodec.decode(Array(service.refloatInfoRequest(canID: 42)))
      == [34, 42, 36, 101, 0, 2, 0]
  )
  #expect(
    try service.receive(Data(VescFrameCodec.encode([36, 101, 0, 12])))
      == [.refloatInfo(version: "1.2")]
  )
}

@Test func refloatRealtimeIDsAndDataNormalizeCoreTelemetry() throws {
  let service = VescProtocolService()
  #expect(
    try VescFrameCodec.decode(Array(service.refloatTelemetryIDsRequest(canID: 42)))
      == [34, 42, 36, 101, 32]
  )
  #expect(
    try VescFrameCodec.decode(Array(service.refloatTelemetryRequest(canID: 42)))
      == [34, 42, 36, 101, 31]
  )
  let fields = [
    "motor.speed", "motor.erpm", "motor.current", "motor.duty_cycle",
    "motor.batt_voltage", "motor.batt_current", "motor.mosfet_temp",
    "motor.motor_temp",
  ]
  var idsPayload: [UInt8] = [36, 101, 32, UInt8(fields.count)]
  for field in fields {
    let bytes = Array(field.utf8)
    idsPayload.append(UInt8(bytes.count))
    idsPayload += bytes
  }
  idsPayload.append(0)
  #expect(
    try service.receive(Data(VescFrameCodec.encode(idsPayload)))
      == [.refloatTelemetryLayout(fields: fields, runtimeFields: [])]
  )

  let payload: [UInt8] = [
    36, 101, 31,
    0, 0, 0, 0, 0, 1,
    3, 0x40, 0, 0,
    0x49, 0x00,
    0x64, 0x00,
    0x48, 0x00,
    0x38, 0x00,
    0x54, 0x00,
    0xc4, 0x00,
    0x50, 0x00,
    0x4c, 0x00,
  ]

  let message = try #require(
    service.receive(Data(VescFrameCodec.encode(payload))).first
  )
  guard case .refloatTelemetry(let value) = message else {
    Issue.record("Expected Refloat telemetry")
    return
  }
  #expect(value.rideState == .riding)
  #expect(value.footpadState == 1)
  #expect(value.packVoltageV == 64)
  #expect(value.dutyCycle == 0.5)
  #expect(value.controllerTempC == 32)
  #expect(value.motorTempC == 16)
  #expect(value.motorCurrentA == 8)
  #expect(value.batteryCurrentA == -4)
  #expect(value.erpm == 1024)
  #expect(value.speedKph == 36)
}

@Test func androidHardwareRefloatCaptureDecodesAcrossNotificationChunks() throws {
  let capture = try multiCaseFixture(
    "fixtures/refloat/rtdata-1.2.3-android-hardware.json"
  )
  let service = VescProtocolService()
  var messages: [VescProtocolMessage] = []

  for item in capture.cases {
    let frame = bytes(item.frameHex)
    var offset = 0
    for count in item.chunkSizes {
      messages += try service.receive(Data(frame[offset..<(offset + count)]))
      offset += count
    }
  }

  #expect(messages.count == 3)
  #expect(messages[0] == .refloatInfo(version: "1.2.3"))
  guard case .refloatTelemetryLayout(let fields, let runtimeFields) = messages[1] else {
    Issue.record("Expected captured Refloat field layout")
    return
  }
  #expect(fields.count == 16)
  #expect(runtimeFields.count == 10)
  #expect(fields.prefix(3) == ["motor.speed", "motor.erpm", "motor.current"])

  guard case .refloatTelemetry(let value) = messages[2] else {
    Issue.record("Expected captured Refloat telemetry")
    return
  }
  #expect(value.rideState == .idle)
  #expect(value.footpadState == 0)
  #expect(value.motorCurrentA == 0)
  #expect(value.batteryCurrentA == 0)
  #expect(value.erpm == 0)
}
