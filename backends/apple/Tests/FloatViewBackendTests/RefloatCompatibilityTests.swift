@testable import FloatViewBackend
import Testing

@Test func rustCompatibilityAdapterMapsEveryRecordProfileAndRejectsPrivateCorruption() throws {
  let legacy = [UInt8](arrayLiteral: 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0)
  let dynamic = [UInt8](arrayLiteral: 1, 1, 1, 1, 1, 2, 2, 2, 1, 0, 0, 0)
  let alerts = [UInt8](arrayLiteral: 1, 1, 1, 2, 1, 3, 3, 2, 2, 1, 0, 0)

  #expect(try RefloatCompatibility.decision(from: legacy) == refloatDecision("1.0"))
  #expect(try RefloatCompatibility.decision(from: dynamic) == refloatDecision("1.1.2"))
  #expect(try RefloatCompatibility.decision(from: alerts) == refloatDecision("1.2.3"))

  var malformed = [[UInt8]]()
  malformed.append(Array(legacy.dropLast()))
  for (index, value) in [
    (0, UInt8(2)), (1, 2), (4, 2), (4, 0), (6, 0), (7, 0), (8, 0),
    (9, 2), (10, 1), (11, 1),
  ] {
    var record = legacy
    record[index] = value
    if index == 4 { record[5] = 1 }
    malformed.append(record)
  }
  for profile in UInt8(1) ... 3 {
    for family in UInt8(1) ... 2 {
      for lighting in UInt8(1) ... 2 {
        for fatal in UInt8(0) ... 1 {
          let combination = [profile, family, lighting, fatal]
          guard combination != [1, 1, 1, 0],
                combination != [2, 2, 1, 0],
                combination != [3, 2, 2, 1]
          else { continue }
          var record = alerts
          record[6 ... 9] = combination[0 ... 3]
          malformed.append(record)
        }
      }
    }
  }
  for record in malformed {
    #expect(throws: VescProtocolError.truncatedPayload) {
      try RefloatCompatibility.decision(from: record)
    }
  }
}

@Test func rustCompatibilityRejectsUnknownInfoAndIncompleteLayout() throws {
  #expect(throws: VescProtocolError.invalidFrame) {
    try RefloatCompatibility.decodeInfo(refloatInfoPayload("1.2.4"))
  }
  let incomplete = try RefloatDynamicTelemetryDecoder.decodeFields(
    [36, 101, 32, 1, 11] + Array("motor.speed".utf8) + [0]
  )
  #expect(try !RefloatCompatibility.admits(layoutRecord: incomplete.record))
}
