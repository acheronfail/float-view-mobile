import Foundation
import Testing

@testable import FloatViewBackend

private func layoutRecord(_ name: String = "motor.speed") -> [UInt8] {
  let bytes = Array(name.utf8)
  return [1, 1, 1, 0, 0, UInt8(bytes.count), 0, 0, UInt8(bytes.count)] + bytes
}

private func bigEndianBytes(_ value: UInt64) -> [UInt8] {
  (0..<8).map { shift in UInt8(truncatingIfNeeded: value >> UInt64(56 - shift * 8)) }
}

private func sampleRecord(extensionName: String? = nil) -> [UInt8] {
  var record = [UInt8](repeating: 0, count: 32)
  record[0] = 1
  record[1] = 2
  record[2] = 4
  record[3] = 1
  record[4] = 4
  record[5] = 1
  record[6] = 1
  record[8] = 1
  record[16..<20] = [0, 0, 0, 9]
  record[20..<24] = [0, 0, 0, 1]
  record += bigEndianBytes(12.5.bitPattern)
  record += [UInt8](repeating: 0, count: 27 * 8)
  if let extensionName {
    let name = Array(extensionName.utf8)
    record[12] = 1
    record.append(UInt8(name.count))
    record += name
    record += bigEndianBytes(3.25.bitPattern)
  }
  return record
}

private func layoutPayload(_ name: String) -> [UInt8] {
  let bytes = Array(name.utf8)
  return [36, 101, 32, 1, UInt8(bytes.count)] + bytes + [0]
}

private func readyPayload(_ bits: UInt16 = 0x3c00) -> [UInt8] {
  [
    36, 101, 31, 0, 0, 0, 0, 0, 9, 2, 0, 0, 0,
    UInt8(truncatingIfNeeded: bits >> 8), UInt8(truncatingIfNeeded: bits),
  ]
}

@Test func dynamicTelemetryAdapterMapsCanonicalRecordsAndExtensions() throws {
  let layout = try RefloatDynamicTelemetryDecoder.layout(from: layoutRecord())
  #expect(layout.fields == ["motor.speed"])
  #expect(layout.runtimeFields.isEmpty)

  let value = try RefloatDynamicTelemetryDecoder.telemetry(
    from: sampleRecord(extensionName: "future.energy")
  )
  #expect(value.rideState == .riding)
  #expect(value.packageState == .running)
  #expect(value.footpadState == .both)
  #expect(value.speedKph == 12.5)
  #expect(value.extensions == ["future.energy": 3.25])
}

@Test func dynamicTelemetryAdapterRejectsMalformedCanonicalRecords() throws {
  let validLayout = layoutRecord()
  for invalid in [
    Array(validLayout.dropLast()),
    [2] + Array(validLayout.dropFirst()),
    Array(validLayout.prefix(6)) + [1, 0] + Array(validLayout.dropFirst(8)),
  ] {
    #expect(throws: VescProtocolError.self) {
      try RefloatDynamicTelemetryDecoder.layout(from: invalid)
    }
  }

  let validSample = sampleRecord()
  for (index, value) in [(0, UInt8(2)), (1, 1), (2, 0), (7, 8), (11, 8), (13, 1)] {
    var invalid = validSample
    invalid[index] = value
    #expect(throws: VescProtocolError.self) {
      try RefloatDynamicTelemetryDecoder.telemetry(from: invalid)
    }
  }
  var noncanonicalAbsence = validSample
  noncanonicalAbsence[40] = 1
  #expect(throws: VescProtocolError.self) {
    try RefloatDynamicTelemetryDecoder.telemetry(from: noncanonicalAbsence)
  }
  #expect(throws: VescProtocolError.self) {
    try RefloatDynamicTelemetryDecoder.telemetry(from: Array(validSample.dropLast()))
  }
}

@Test func dynamicTelemetryLayoutIsConnectionScopedAndReplacedAtomically() throws {
  let service = VescProtocolService()
  let firstLayout = try VescFrameCodec.encode(layoutPayload("motor.speed"))
  _ = try service.receive(Data(firstLayout))
  service.reset()
  #expect(throws: VescProtocolError.self) {
    try service.receive(Data(VescFrameCodec.encode(readyPayload())))
  }

  let replacement = try VescFrameCodec.encode(layoutPayload("future.energy"))
  _ = try service.receive(Data(replacement))
  guard
    case .refloatTelemetry(let value) = try #require(
      service.receive(Data(VescFrameCodec.encode(readyPayload(0x4200)))).first
    )
  else {
    Issue.record("Expected replacement-layout telemetry")
    return
  }
  #expect(value.speedKph == nil)
  #expect(value.extensions == ["future.energy": 3])
}

@Test func dynamicTelemetryAdapterRejectsABIMismatch() {
  #expect(throws: VescProtocolError.self) {
    try RefloatDynamicTelemetryDecoder.requireCompatibleABI(2)
  }
}
