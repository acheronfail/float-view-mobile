@testable import FloatViewBackend
import Foundation
import Testing

private struct LegacySourceFixture: Decodable {
  struct Case: Decodable {
    let name: String
    let payloadHex: String
  }

  let cases: [Case]
}

private func legacyPayload(_ name: String) throws -> [UInt8] {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let data = try Data(
    contentsOf: root.appending(path: "fixtures/refloat/detailed-source-provenance.json")
  )
  let fixture = try JSONDecoder().decode(LegacySourceFixture.self, from: data)
  let hex = try #require(fixture.cases.first { $0.name == name }).payloadHex
  return stride(from: 0, to: hex.count, by: 2).map { offset in
    let start = hex.index(hex.startIndex, offsetBy: offset)
    let end = hex.index(start, offsetBy: 2)
    return UInt8(hex[start ..< end], radix: 16)!
  }
}

@Test func rustLegacyTelemetryBridgePreservesAllDetailOptionsAndSentinel() throws {
  for detail in 1 ... 4 {
    let decoded = try RefloatLegacyTelemetryDecoder.decode(
      legacyPayload("1.0.0-command-10-detail-\(detail)")
    )
    #expect(decoded.kind == .compact)
    #expect((decoded.controllerTempC != nil) == (detail >= 2))
    #expect((decoded.distanceM != nil) == (detail >= 2))
    #expect((decoded.odometerM != nil) == (detail >= 3))
    #expect((decoded.batteryLevel != nil) == (detail >= 3))
    #expect((decoded.chargingCurrentA != nil) == (detail >= 4))
    #expect((decoded.chargingVoltageV != nil) == (detail >= 4))
  }
  var sentinel = try legacyPayload("1.0.0-command-10-detail-2")
  sentinel[34] = 222
  #expect(try RefloatLegacyTelemetryDecoder.decode(sentinel).fieldWeakeningCurrentA == nil)
}

@Test func rustLegacyTelemetryBridgeMapsKnownAndUnknownActiveFaults() throws {
  let known = try RefloatLegacyTelemetryDecoder.decode(
    legacyPayload("1.2.3-command-10-active-firmware-fault")
  )
  #expect(known.alertReason == .firmwareFault)
  #expect(known.firmwareFault == .overVoltage)
  var unknown = try legacyPayload("1.2.3-command-10-active-firmware-fault")
  unknown[4] = 250
  #expect(try RefloatLegacyTelemetryDecoder.decode(unknown).firmwareFault == .unknown)
}

@Test func rustLegacyTelemetryAdapterRejectsEveryPrivateRecordClass() throws {
  #expect(throws: VescProtocolError.self) {
    try RefloatLegacyTelemetryDecoder.requireCompatibleABI(2)
  }
  var record = canonicalLegacySampleRecord()
  _ = try RefloatLegacyTelemetryDecoder.telemetry(from: record)
  for index in [0, 1, 2, 3, 4, 5, 6, 7, 9] {
    var invalid = record
    invalid[index] = 255
    #expect(throws: VescProtocolError.self) {
      try RefloatLegacyTelemetryDecoder.telemetry(from: invalid)
    }
  }
  for index in [10, 11, 14, 15] {
    var reserved = record
    reserved[index] = 1
    #expect(throws: VescProtocolError.self) {
      try RefloatLegacyTelemetryDecoder.telemetry(from: reserved)
    }
  }
  var unknownBehavior = record
  unknownBehavior[8] = 8
  #expect(throws: VescProtocolError.self) {
    try RefloatLegacyTelemetryDecoder.telemetry(from: unknownBehavior)
  }
  var unknownOption = record
  unknownOption[12] = 0x80
  #expect(throws: VescProtocolError.self) {
    try RefloatLegacyTelemetryDecoder.telemetry(from: unknownOption)
  }
  var absentNonzero = record
  absentNonzero.replaceSubrange(16 + 2 * 8 ..< 16 + 3 * 8, with: 1.0.bitPattern.bigEndianBytes)
  #expect(throws: VescProtocolError.self) {
    try RefloatLegacyTelemetryDecoder.telemetry(from: absentNonzero)
  }
  var nonfinite = record
  nonfinite.replaceSubrange(16 ..< 24, with: Double.infinity.bitPattern.bigEndianBytes)
  #expect(throws: VescProtocolError.self) {
    try RefloatLegacyTelemetryDecoder.telemetry(from: nonfinite)
  }
  record.removeLast()
  #expect(throws: VescProtocolError.self) {
    try RefloatLegacyTelemetryDecoder.telemetry(from: record)
  }
}

private func canonicalLegacySampleRecord() -> [UInt8] {
  var record = [UInt8](repeating: 0, count: 264)
  record[0 ... 9] = [1, 1, 1, 2, 1, 4, 1, 1, 0, 1]
  return record
}

private extension UInt64 {
  var bigEndianBytes: [UInt8] {
    (0 ..< 8).map { UInt8(truncatingIfNeeded: self >> UInt64((7 - $0) * 8)) }
  }
}
