@testable import FloatViewBackend
import Foundation
import Testing

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

private struct RefloatSourceFixture: Decodable {
  struct Case: Decodable {
    let name: String
    let payloadHex: String
  }

  let cases: [Case]
}

private struct BmsSourceFixture: Decodable {
  struct Case: Decodable {
    let name: String
    let payloadHex: String
  }

  let cases: [Case]
}

private func fixture(_ relativePath: String) throws -> Fixture {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let data = try Data(contentsOf: root.appending(path: relativePath))
  return try JSONDecoder().decode(Fixture.self, from: data)
}

private func multiCaseFixture(_ relativePath: String) throws -> MultiCaseFixture {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let data = try Data(contentsOf: root.appending(path: relativePath))
  return try JSONDecoder().decode(MultiCaseFixture.self, from: data)
}

private func rawFixture(_ relativePath: String) throws -> Data {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  return try Data(contentsOf: root.appending(path: relativePath))
}

private func refloatSourcePayload(_ name: String) throws -> [UInt8] {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let data = try Data(
    contentsOf: root.appending(path: "fixtures/refloat/detailed-source-provenance.json")
  )
  let fixture = try JSONDecoder().decode(RefloatSourceFixture.self, from: data)
  return try bytes(#require(fixture.cases.first { $0.name == name }).payloadHex)
}

private func bmsSourcePayload(_ name: String) throws -> [UInt8] {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let data = try Data(contentsOf: root.appending(path: "fixtures/vesc-bms/source-derived.json"))
  let fixture = try JSONDecoder().decode(BmsSourceFixture.self, from: data)
  return try bytes(#require(fixture.cases.first { $0.name == name }).payloadHex)
}

private func boundedBmsPayload(
  cells: Int, temperatures: Int, dataVersion: UInt8? = nil, status: [UInt8] = []
) -> [UInt8] {
  var payload = [UInt8(96)] + Array(repeating: 0, count: 24)
  payload.append(UInt8(cells))
  payload += Array(repeating: 0, count: cells * 2)
  payload += Array(repeating: 0, count: cells)
  payload.append(UInt8(temperatures))
  payload += Array(repeating: 0, count: temperatures * 2)
  payload += Array(repeating: 0, count: 12)
  payload.append(42)
  payload += Array(repeating: 0, count: 18)
  if let dataVersion {
    payload.append(dataVersion)
    payload += status
    payload.append(0)
  }
  return payload
}

private func bytes(_ hex: String) -> [UInt8] {
  stride(from: 0, to: hex.count, by: 2).map {
    let start = hex.index(hex.startIndex, offsetBy: $0)
    return UInt8(hex[start ..< hex.index(start, offsetBy: 2)], radix: 16)!
  }
}

@Test func firmwareFixtureDecodesAcrossChunks() throws {
  let value = try fixture("fixtures/vesc/firmware-6.06.json")
  let frame = bytes(value.frameHex)
  var stream = VescFrameStreamDecoder()
  var offset = 0
  var payloads: [[UInt8]] = []
  for count in value.chunkSizes {
    payloads += try stream.append(frame[offset ..< (offset + count)])
    offset += count
  }
  let info = try VescPayloadDecoder.firmware(#require(payloads.first))
  #expect(info == FirmwareInfo(
    major: 6,
    minor: 6,
    hardwareName: "THOR300",
    controllerUUID: Data(repeating: 0, count: 12),
    hardwareConfigurationCRC: 0
  ))
}

@Test func expressFirmwareFixtureDecodesAcrossChunks() throws {
  let value = try fixture("fixtures/vesc/firmware-express-6.05.json")
  let frame = bytes(value.frameHex)
  var stream = VescFrameStreamDecoder()
  var offset = 0
  var payloads: [[UInt8]] = []
  for count in value.chunkSizes {
    payloads += try stream.append(frame[offset ..< (offset + count)])
    offset += count
  }
  let info = try VescPayloadDecoder.firmware(#require(payloads.first))
  #expect(info == FirmwareInfo(
    major: 6,
    minor: 5,
    hardwareName: "VESC Express T",
    hardwareType: 2,
    controllerUUID: Data(repeating: 0, count: 12)
  ))
}

@Test func composedThorFirmwareVariantsDecode() throws {
  for (version, major, minor) in [("6.02", 6, 2), ("6.05", 6, 5), ("6.06", 6, 6), ("7.00", 7, 0)] {
    let value = try fixture("fixtures/vesc/firmware-thor300-\(version).json")
    let payload = try VescFrameCodec.decode(bytes(value.frameHex))
    let info = try VescPayloadDecoder.firmware(payload)
    #expect(
      info == FirmwareInfo(
        major: UInt8(major),
        minor: UInt8(minor),
        hardwareName: "THOR300",
        hardwareType: 0,
        controllerUUID: Data(repeating: 0, count: 12),
        hardwareConfigurationCRC: version == "6.02" ? nil : 0
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
}

@Test func motorConfigurationCurrentLimitsDecodeAndRoute() throws {
  var payload = Array(repeating: UInt8(0), count: 489)
  payload.replaceSubrange(
    0 ..< 25,
    with: bytes("0e57ad8f530000000042c80000c2c8000042480000c1a00000")
  )
  payload[463] = 18
  let limits = try VescPayloadDecoder.currentLimits(payload)
  #expect(limits == ControllerCurrentLimits(
    motorCurrentMaxA: 100,
    motorCurrentMinA: -100,
    batteryCurrentMaxA: 50,
    batteryCurrentMinA: -20,
    batterySeriesCells: 18
  ))

  let service = VescProtocolService()
  #expect(
    try VescFrameCodec.decode(Array(service.motorConfigurationRequest(route: .direct))) == [14]
  )
  #expect(
    try VescFrameCodec.decode(Array(service.motorConfigurationRequest(route: .can(42))))
      == [34, 42, 14]
  )
  #expect(
    try service.receive(Data(VescFrameCodec.encode(payload))) == [.currentLimits(limits)]
  )
}

@Test func detailedSourceFixturesDecodeEveryKnownLegacyAndDynamicField() throws {
  let legacy = try RefloatLegacyTelemetryDecoder.decode(
    refloatSourcePayload("1.0.0-command-10-detail-4")
  )
  #expect(legacy.kind == .compact)
  #expect(legacy.packageState == .running)
  #expect(legacy.packageMode == .normal)
  #expect(legacy.footpadState == .both)
  #expect(legacy.packVoltageV != nil)
  #expect(legacy.dutyCycle != nil)
  #expect(legacy.motorCurrentA != nil)
  #expect(legacy.batteryCurrentA != nil)
  #expect(legacy.erpm != nil)
  #expect(legacy.speedKph != nil)
  #expect(legacy.pitchDeg != nil)
  #expect(legacy.balancePitchDeg != nil)
  #expect(legacy.rollDeg != nil)
  #expect(legacy.requestedBalanceCurrentA != nil)
  #expect(legacy.boosterCurrentA != nil)
  #expect(legacy.fieldWeakeningCurrentA != nil)
  #expect(legacy.footpadAdc1V != nil)
  #expect(legacy.footpadAdc2V != nil)
  #expect(legacy.setpointDeg != nil)
  #expect(legacy.atrSetpointDeg != nil)
  #expect(legacy.brakeTiltSetpointDeg != nil)
  #expect(legacy.torqueTiltSetpointDeg != nil)
  #expect(legacy.turnTiltSetpointDeg != nil)
  #expect(legacy.remoteSetpointDeg != nil)
  #expect(legacy.distanceM != nil)
  #expect(legacy.odometerM != nil)
  #expect(legacy.ampHoursDrawnAh != nil)
  #expect(legacy.ampHoursChargedAh != nil)
  #expect(legacy.wattHoursDrawnWh != nil)
  #expect(legacy.wattHoursChargedWh != nil)
  #expect(legacy.batteryLevel != nil)
  #expect(legacy.chargingCurrentA != nil)
  #expect(legacy.chargingVoltageV != nil)

  let (fields, runtimeFields) = try RefloatDynamicTelemetryDecoder.decodeFields(
    refloatSourcePayload("1.1.0-and-1.2.3-command-32-layout")
  )
  #expect(fields.count == 16)
  #expect(runtimeFields.count == 10)
  let dynamic = try RefloatDynamicTelemetryDecoder.decodeData(
    refloatSourcePayload("1.2.3-command-31-running-alert-tail"),
    fields: fields,
    runtimeFields: runtimeFields
  )
  #expect(dynamic.kind == .realtime)
  #expect(dynamic.packageState == .running)
  #expect(dynamic.packageMode == .normal)
  #expect(dynamic.footpadState == .both)
  #expect(dynamic.directionalMotorCurrentA != nil)
  #expect(dynamic.filteredMotorCurrentA != nil)
  #expect(dynamic.remoteInput != nil)
  #expect(dynamic.atrAccelerationDifferenceErpmPerSecond != nil)
  #expect(dynamic.atrSpeedBoost != nil)
  #expect(dynamic.extensions.isEmpty)
  #expect(try abs(#require(dynamic.speedKph) - (try #require(legacy.speedKph))) < 0.3)

  let fault = try RefloatDynamicTelemetryDecoder.decodeData(
    refloatSourcePayload("1.2.3-command-31-active-firmware-fault"),
    fields: fields,
    runtimeFields: runtimeFields
  )
  #expect(fault.alertReason == .firmwareFault)
  #expect(fault.firmwareFault == .overVoltage)
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
  let combined = [0xFF, 0x01] + shortFrame + shortFrame
  let packets = try stream.append(combined)
  #expect(packets == [[0, 6, 6, 0], [0, 6, 6, 0]])

  var truncated = VescFrameStreamDecoder()
  #expect(try truncated.append(shortFrame.dropLast()).isEmpty)
  #expect(try truncated.append(shortFrame.suffix(1)) == [[0, 6, 6, 0]])
}

@Test func protocolDecodingLeavesFirmwareRouteAttributionToScheduler() throws {
  let service = VescProtocolService()
  #expect(try VescFrameCodec.decode(Array(service.canPingRequest())) == [62])
  #expect(
    try service.receive(Data(VescFrameCodec.encode([62, 42, 7, 42])))
      == [.canDevices([7, 42])]
  )

  #expect(
    try VescFrameCodec.decode(Array(service.firmwareRequest(route: .can(42)))) == [34, 42, 0]
  )
  let firmwareFrame = try bytes(fixture("fixtures/vesc/firmware-6.06.json").frameHex)
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
          canID: nil,
          controllerUUID: Data(repeating: 0, count: 12),
          hardwareConfigurationCRC: 0
        )
      )
  )
  #expect(
    try VescFrameCodec.decode(Array(service.refloatInfoRequest(route: .can(42))))
      == [34, 42, 36, 101, 0, 2, 0]
  )
  #expect(
    try service.receive(Data(VescFrameCodec.encode([36, 101, 0, 12])))
      == [.refloatInfo(version: "1.2")]
  )
}

@Test func bmsValuesRequestsUseExactDirectAndCANRoutes() throws {
  let service = VescProtocolService()
  #expect(try VescFrameCodec.decode(Array(service.bmsValuesRequest(connection: .direct))) == [96])
  #expect(try VescFrameCodec.decode(Array(service.bmsValuesRequest(connection: .can(42)))) == [34, 42, 96])
}

@Test func firmwareIdentityUsesStrictHardwareSpecificTails() throws {
  let vbms = [UInt8(0), 6, 0] + Array("VBMS".utf8) + [0] +
    Array(repeating: UInt8(7), count: 12) + [0, 0, 1, 1]
  let decoded = try VescPayloadDecoder.firmware(vbms)
  #expect(decoded.hardwareType == 1)
  #expect(decoded.firmwareName == nil)
  #expect(decoded.controllerUUID == Data(repeating: 7, count: 12))
  #expect(decoded.customConfigurationCount == 1)
  #expect(decoded.hardwareConfigurationCRC == nil)
  #expect(throws: VescProtocolError.self) { try VescPayloadDecoder.firmware(vbms + [0]) }
  var wrongVersion = vbms
  wrongVersion[2] = 1
  #expect(throws: VescProtocolError.self) { try VescPayloadDecoder.firmware(wrongVersion) }
}

@Test func sourceDerivedVBMSFirmwareAndValuesDecodeExactly() throws {
  let firmware = try VescPayloadDecoder.firmware(bmsSourcePayload("vbms-firmware-6.00"))
  #expect(firmware.major == 6)
  #expect(firmware.minor == 0)
  #expect(firmware.hardwareName == "VBMS")
  #expect(firmware.hardwareType == 1)

  let vbms32 = try VescPayloadDecoder.firmware(bmsSourcePayload("vbms32-firmware-6.06"))
  #expect(vbms32.major == 6)
  #expect(vbms32.minor == 6)
  #expect(vbms32.hardwareName == "VBMS32")
  #expect(vbms32.hardwareType == 2)
  #expect(vbms32.firmwareName == nil)

  let observedAt = Date(timeIntervalSince1970: 1234)
  let legacy = try VescPayloadDecoder.bmsValues(
    bmsSourcePayload("vbms-values-legacy-typical"), expectedCanID: 42,
    observedAt: observedAt
  )
  #expect(legacy.observedAt == observedAt)
  #expect(legacy.totalVoltageV == 50.4)
  #expect(legacy.chargeVoltageV == 54.6)
  #expect(legacy.inputCurrentA == -12.345)
  #expect(legacy.inputICCurrentA == -12.3)
  #expect(legacy.sessionAmpHoursAh == 1.25)
  #expect(legacy.sessionWattHoursWh == 62.5)
  #expect(legacy.cellVoltagesV == [4.2, 4.19, 4.18])
  #expect(legacy.balancing == [true, false, true])
  #expect(legacy.temperaturesC == [25.5, 26.25])
  #expect(legacy.reportedCanID == 42)
  #expect(abs(legacy.pressurePa - 101_320) < 0.001)
  #expect(legacy.dataVersion == nil)
  #expect(legacy.status == nil)

  let extended = try VescPayloadDecoder.bmsValues(
    bmsSourcePayload("vbms-values-extended-typical"), expectedCanID: 42,
    observedAt: observedAt
  )
  #expect(extended.dataVersion == 1)
  #expect(extended.status == "OK")
}

@Test func bmsValuesAcceptExactMinimumAndMaximumBounds() throws {
  let observedAt = Date(timeIntervalSince1970: 0)
  let minimum = try VescPayloadDecoder.bmsValues(
    boundedBmsPayload(cells: 0, temperatures: 0), expectedCanID: 42,
    observedAt: observedAt
  )
  #expect(minimum.cellVoltagesV.isEmpty)
  #expect(minimum.temperaturesC.isEmpty)

  let maximumStatus = Array(repeating: UInt8(ascii: "A"), count: 40)
  let maximum = try VescPayloadDecoder.bmsValues(
    boundedBmsPayload(
      cells: 50, temperatures: 50, dataVersion: 255, status: maximumStatus
    ),
    expectedCanID: 42, observedAt: observedAt
  )
  #expect(maximum.cellVoltagesV.count == 50)
  #expect(maximum.balancing.count == 50)
  #expect(maximum.temperaturesC.count == 50)
  #expect(maximum.status?.utf8.count == 40)
}

@Test func malformedBmsValuesNeverPublishPartialSnapshots() throws {
  let observedAt = Date(timeIntervalSince1970: 0)
  let legacy = try bmsSourcePayload("vbms-values-legacy-typical")
  for length in 0 ..< legacy.count {
    #expect(throws: VescProtocolError.self) {
      try VescPayloadDecoder.bmsValues(
        Array(legacy.prefix(length)), expectedCanID: 42, observedAt: observedAt
      )
    }
  }

  var excessiveCells = legacy
  excessiveCells[25] = 51
  var excessiveTemperatures = legacy
  excessiveTemperatures[35] = 51
  var invalidBalancing = legacy
  invalidBalancing[32] = 2
  var nonFiniteAutoFloat = legacy
  nonFiniteAutoFloat.replaceSubrange(53 ... 56, with: [0x7F, 0xFF, 0xFF, 0xFF])
  let missingStatusTerminator: [UInt8] = legacy + [1]
  let oversizedStatus: [UInt8] =
    legacy + [1] + Array(repeating: UInt8(ascii: "A"), count: 41) + [0]
  let invalidUTF8: [UInt8] = legacy + [1, 0xFF, 0]
  let unexpectedRemainder: [UInt8] = legacy + [1, UInt8(ascii: "O"), 0, 0]
  let malformed: [[UInt8]] = [
    excessiveCells, excessiveTemperatures, invalidBalancing, nonFiniteAutoFloat,
    missingStatusTerminator, oversizedStatus, invalidUTF8, unexpectedRemainder,
  ]
  for payload in malformed {
    #expect(throws: VescProtocolError.self) {
      try VescPayloadDecoder.bmsValues(payload, expectedCanID: 42, observedAt: observedAt)
    }
  }
  #expect(throws: VescProtocolError.self) {
    try VescPayloadDecoder.bmsValues(legacy, expectedCanID: 41, observedAt: observedAt)
  }
}

@Test func exactCompatibilityCatalogsAndLegacyWireIdentityAreStable() throws {
  #expect(VescCompatibility.supported == [
    .init(6, 2), .init(6, 5), .init(6, 6), .init(7, 0),
  ])
  #expect(RefloatCompatibility.entries.map(\.sourceVersion) == [
    "1.0.0", "1.0.1",
    "1.1.0", "1.1.1", "1.1.2",
    "1.2.0", "1.2.1", "1.2.2", "1.2.3",
  ])
  #expect(RefloatCompatibility.entry(wireVersion: "1.0")?.family == .legacyFixed)
  #expect(RefloatCompatibility.entry(wireVersion: "1.2.0")?.profile == .dynamicAlerts)
  #expect(RefloatCompatibility.entry(wireVersion: "1.3.0") == nil)
  #expect(RefloatCompatibility.supportsDynamicLayout(
    fields: RefloatCompatibility.requiredFields + ["future.energy"],
    runtimeFields: RefloatCompatibility.requiredRuntimeFields
  ))
  #expect(!RefloatCompatibility.supportsDynamicLayout(
    fields: Array(RefloatCompatibility.requiredFields.dropFirst()),
    runtimeFields: RefloatCompatibility.requiredRuntimeFields
  ))

  let service = VescProtocolService()
  let legacy = try VescFrameCodec.encode([36, 101, 0, 10])
  #expect(try service.receive(Data(legacy)) == [.refloatInfo(version: "1.0")])
  #expect(
    try VescFrameCodec.decode(Array(service.refloatLegacyTelemetryRequest(route: .direct)))
      == [36, 101, 10, 4]
  )
  #expect(
    try VescFrameCodec.decode(Array(service.refloatTelemetryRequest(route: .direct)))
      == [36, 101, 31]
  )
}

@Test func strictFirmwareIdentityAllowsReadOnlyTestBuilds() throws {
  let payload = try VescFrameCodec.decode(bytes(fixture("fixtures/vesc/firmware-thor300-7.00.json").frameHex))
  let identity = try VescPayloadDecoder.firmware(payload)
  #expect(identity.testVersion == 0)
  #expect(identity.customConfigurationCount == 0)
  #expect(identity.hardwareConfigurationCRC == 0)
  #expect(throws: VescProtocolError.truncatedPayload) {
    try VescPayloadDecoder.firmware(Array(payload.dropLast()))
  }
  let testFirmware = FirmwareRecord(
    major: 7,
    minor: 0,
    testVersion: 1,
    hardwareName: "THOR300",
    firmwareName: nil
  )
  #expect(VescCompatibility.supports(testFirmware))
  #expect(VescCompatibility.accessMode(testFirmware) == .readOnly)
  #expect(VescCompatibility.permits(.read, for: testFirmware))
  #expect(!VescCompatibility.permits(.write, for: testFirmware))
  #expect(!VescCompatibility.supports(FirmwareRecord(
    major: 7,
    minor: 1,
    hardwareName: "THOR300",
    firmwareName: nil
  )))
}

@Test func customConfigurationCommandsUseExactPayloadsAndStrictResponses() throws {
  let service = VescProtocolService()
  #expect(
    try VescFrameCodec.decode(Array(service.customConfigurationSchemaRequest(
      route: .direct, length: 400, offset: 0
    ))) == [92, 0, 0, 0, 1, 144, 0, 0, 0, 0]
  )
  #expect(
    try VescFrameCodec.decode(Array(service.customConfigurationSchemaRequest(
      route: .can(42), length: 16, offset: 400
    ))) == [34, 42, 92, 0, 0, 0, 0, 16, 0, 0, 1, 144]
  )
  #expect(try VescFrameCodec.decode(Array(service.customConfigurationValuesRequest(
    route: .direct, defaults: false
  ))) == [93, 0])
  #expect(try VescFrameCodec.decode(Array(service.customConfigurationValuesRequest(
    route: .can(42), defaults: true
  ))) == [34, 42, 94, 0])
  #expect(try VescFrameCodec.decode(Array(service.customConfigurationWriteRequest(
    route: .can(42), values: Data([1, 2, 3, 4])
  ))) == [34, 42, 95, 0, 1, 2, 3, 4])

  #expect(
    try VescCustomConfigurationCodec.decode([92, 0, 0, 0, 0, 4, 0, 0, 0, 0, 1, 2, 3, 4])
      == .schemaChunk(index: 0, totalLength: 4, offset: 0, bytes: Data([1, 2, 3, 4]))
  )
  #expect(
    try VescCustomConfigurationCodec.decode([93, 0, 0, 0, 0, 1])
      == .values(command: .getCustomConfiguration, index: 0, bytes: Data([0, 0, 0, 1]))
  )
  #expect(try VescCustomConfigurationCodec.decode([95]) == .writeAcknowledgement)
  #expect(throws: VescProtocolError.self) {
    try VescCustomConfigurationCodec.decode([92, 0, 0, 0, 0, 4, 0, 0, 0, 3, 1, 2])
  }
  #expect(throws: VescProtocolError.self) {
    try VescCustomConfigurationCodec.decode([95, 0])
  }
  #expect(throws: VescProtocolError.self) {
    try VescCustomConfigurationCodec.decode([36])
  }
}

@Test func rustCustomConfigurationAdapterRejectsInvalidABIAndPrivateRecords() throws {
  #expect(throws: VescProtocolError.self) {
    try VescCustomConfigurationCodec.requireCompatibleABI(2)
  }
  var record = [UInt8](repeating: 0, count: 28)
  record[0] = 1
  record[1] = 2
  record[2] = 93
  record[20 ..< 24] = [0, 0, 0, 2]
  record[24 ..< 28] = [0, 0, 0, 4]
  #expect(
    try VescCustomConfigurationCodec.response(from: record, payload: [93, 9, 0, 0, 0, 1])
      == .values(command: .getCustomConfiguration, index: 0, bytes: Data([0, 0, 0, 1]))
  )
  record[0] = 2
  #expect(throws: VescProtocolError.self) {
    try VescCustomConfigurationCodec.response(from: record, payload: [93, 9, 0, 0, 0, 1])
  }
  record[0] = 1
  record[24 ..< 28] = [0, 0, 0, 5]
  #expect(throws: VescProtocolError.self) {
    try VescCustomConfigurationCodec.response(from: record, payload: [93, 9, 0, 0, 0, 1])
  }
}

@Test func refloatRealtimeIDsAndDataNormalizeCoreTelemetry() throws {
  let service = VescProtocolService()
  #expect(
    try VescFrameCodec.decode(Array(service.refloatTelemetryIDsRequest(route: .can(42))))
      == [34, 42, 36, 101, 32]
  )
  #expect(
    try VescFrameCodec.decode(Array(service.refloatTelemetryRequest(route: .can(42))))
      == [34, 42, 36, 101, 31]
  )
  let fields = [
    "motor.speed", "motor.erpm", "motor.current", "motor.duty_cycle",
    "motor.batt_voltage", "motor.batt_current", "motor.mosfet_temp",
    "motor.motor_temp", "future.energy",
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
    0x50, 0x80,
    0x64, 0x00,
    0x48, 0x00,
    0x38, 0x00,
    0x54, 0x00,
    0xC4, 0x00,
    0x50, 0x00,
    0x4C, 0x00,
    0x45, 0x00,
  ]

  let message = try #require(
    service.receive(Data(VescFrameCodec.encode(payload))).first
  )
  guard case let .refloatTelemetry(value) = message else {
    Issue.record("Expected Refloat telemetry")
    return
  }
  #expect(value.rideState == .riding)
  #expect(value.footpadState == .left)
  #expect(value.packVoltageV == 64)
  #expect(value.dutyCycle == 0.5)
  #expect(value.controllerTempC == 32)
  #expect(value.motorTempC == 16)
  #expect(value.motorCurrentA == 8)
  #expect(value.batteryCurrentA == -4)
  #expect(value.erpm == 1024)
  #expect(value.speedKph == 36)
  #expect(value.extensions == ["future.energy": 5])
}

@Test func softwareVescPeripheralProducesARidingLegacySample() throws {
  let peripheral = SoftwareVescPeripheral()
  let service = VescProtocolService()
  let request = try service.refloatLegacyTelemetryRequest(route: .direct)
  let response = try #require(peripheral.response(to: request))

  guard case let .refloatTelemetry(value) = try #require(service.receive(response).first) else {
    Issue.record("Expected software peripheral Refloat telemetry")
    return
  }
  #expect(value.rideState == .riding)

  let ping = try #require(peripheral.response(to: service.canPingRequest()))
  #expect(try service.receive(ping) == [.canDevices([42])])

  let identityResponse = try #require(peripheral.response(to: service.firmwareRequest(route: .can(42))))
  guard case let .firmware(identity) = try #require(service.receive(identityResponse).first) else {
    Issue.record("Expected software peripheral VBMS identity")
    return
  }
  #expect(identity.hardwareType == VescFirmwareIdentityDecoder.bmsHardwareType)

  let valuesResponse = try #require(peripheral.response(to: service.bmsValuesRequest(connection: .can(42))))
  guard case let .bmsValuesPayload(payload) = try #require(service.receive(valuesResponse).first) else {
    Issue.record("Expected software peripheral VBMS values")
    return
  }
  let snapshot = try VescPayloadDecoder.bmsValues(Array(payload), expectedCanID: 42, observedAt: .distantPast)
  #expect(snapshot.reportedCanID == 42)
  #expect(snapshot.status == "OK")

  let directIdentityResponse = try #require(peripheral.response(to: service.firmwareRequest(route: .direct)))
  guard case let .firmware(directIdentity) = try #require(service.receive(directIdentityResponse).first) else {
    Issue.record("Expected software peripheral controller identity")
    return
  }
  #expect(directIdentity.customConfigurationCount == 1)

  let schemaResponse = try #require(peripheral.response(to: service.customConfigurationSchemaRequest(
    route: .direct, length: 32, offset: 0
  )))
  guard case let .customConfiguration(.schemaChunk(index, total, offset, chunk)) =
    try #require(service.receive(schemaResponse).first)
  else {
    Issue.record("Expected software peripheral schema chunk")
    return
  }
  let schema = try rawFixture("fixtures/refloat-configuration/schemas/1.0.0.qcompress")
  #expect(index == 0)
  #expect(total == schema.count)
  #expect(offset == 0)
  #expect(chunk == schema.prefix(32))

  let write = try rawFixture("fixtures/refloat-configuration/values/1.0.0-write.bin")
  let writeResponse = try #require(peripheral.response(to: service.customConfigurationWriteRequest(
    route: .direct, values: write
  )))
  #expect(try service.receive(writeResponse) == [.customConfiguration(.writeAcknowledgement)])
  let currentResponse = try #require(peripheral.response(to: service.customConfigurationValuesRequest(
    route: .direct, defaults: false
  )))
  guard case let .customConfiguration(.values(_, _, current)) = try #require(service.receive(currentResponse).first)
  else {
    Issue.record("Expected software peripheral current configuration")
    return
  }
  let expectedReadback = try rawFixture("fixtures/refloat-configuration/values/1.0.0-readback.bin")
  #expect(current == expectedReadback)
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
      messages += try service.receive(Data(frame[offset ..< (offset + count)]))
      offset += count
    }
  }

  #expect(messages.count == 3)
  #expect(messages[0] == .refloatInfo(version: "1.2.3"))
  guard case let .refloatTelemetryLayout(fields, runtimeFields) = messages[1] else {
    Issue.record("Expected captured Refloat field layout")
    return
  }
  #expect(fields.count == 16)
  #expect(runtimeFields.count == 10)
  #expect(fields.prefix(3) == ["motor.speed", "motor.erpm", "motor.current"])

  guard case let .refloatTelemetry(value) = messages[2] else {
    Issue.record("Expected captured Refloat telemetry")
    return
  }
  #expect(value.rideState == .idle)
  #expect(value.footpadState == FootpadState.none)
  #expect(value.motorCurrentA == 0)
  #expect(value.batteryCurrentA == 0)
  #expect(abs((value.erpm ?? 0) - -0.07818603515625) < 0.000_001)
}
