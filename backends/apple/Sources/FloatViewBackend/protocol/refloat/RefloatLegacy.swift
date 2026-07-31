import Foundation

enum RefloatLegacyTelemetryDecoder {
  static func decode(_ payload: [UInt8]) throws -> RefloatTelemetryRecord {
    guard payload.count >= 4 else { throw VescProtocolError.truncatedPayload }
    if payload[3] == 69 {
      guard payload.count == 5 else { throw VescProtocolError.invalidFrame }
      return RefloatTelemetryRecord(
        kind: .compact,
        rideState: .unknown,
        alertReason: .firmwareFault,
        firmwareFault: RefloatTelemetryMappings.firmwareFault(payload[4])
      )
    }
    let detailLevel = payload[3]
    guard (1 ... 4).contains(detailLevel) else { throw VescProtocolError.invalidFrame }
    let required = detailLevel >= 4 ? 59 : (detailLevel >= 3 ? 55 : (detailLevel >= 2 ? 42 : 35))
    guard payload.count == required else { throw VescProtocolError.truncatedPayload }
    let signed16: (Int) -> Int = { index in
      Int(Int16(bitPattern: UInt16(payload[index]) << 8 | UInt16(payload[index + 1])))
    }
    let unsigned32: (Int) -> UInt32 = { index in
      UInt32(payload[index]) << 24 | UInt32(payload[index + 1]) << 16
        | UInt32(payload[index + 2]) << 8 | UInt32(payload[index + 3])
    }
    let legacyState = payload[10] & 0x0F
    let switchBits = payload[11] & 0x07
    let adc1 = Double(payload[12]) / 50
    let adc2 = Double(payload[13]) / 50
    let footpad: FootpadState = switch switchBits {
    case 2: .both
    case 1: adc1 > adc2 ? .left : .right
    default: .none
    }
    let semantic = legacySemanticState(legacyState)
    return RefloatTelemetryRecord(
      kind: .compact,
      rideState: semantic.packageState == .running ? .riding : .idle,
      packageState: semantic.packageState,
      packageMode: payload[11] & 0x08 != 0 ? .handTest : (legacyState == 5 ? .flywheel : .normal),
      footpadState: footpad,
      stopCondition: semantic.stopCondition,
      setpointAdjustment: legacySetpointAdjustment(payload[10] >> 4),
      wheelslip: legacyState == 3,
      darkride: legacyState == 4,
      charging: legacyState == 14,
      alertReason: RefloatTelemetryMappings.alertReason(payload[11] >> 4),
      packVoltageV: Double(signed16(23)) / 10,
      dutyCycle: Double(Int(payload[33]) - 128) / 100,
      controllerTempC: detailLevel >= 2 ? Double(payload[39]) / 2 : nil,
      motorTempC: detailLevel >= 2 ? Double(payload[40]) / 2 : nil,
      motorCurrentA: Double(signed16(29)) / 10,
      batteryCurrentA: Double(signed16(31)) / 10,
      erpm: Double(signed16(25)),
      speedKph: Double(signed16(27)) / 10 * 3.6,
      pitchDeg: Double(signed16(20)) / 10,
      balancePitchDeg: Double(signed16(6)) / 10,
      rollDeg: Double(signed16(8)) / 10,
      requestedBalanceCurrentA: Double(signed16(4)) / 10,
      boosterCurrentA: Double(Int(payload[22]) - 128),
      fieldWeakeningCurrentA: Double(payload[34]) / 3,
      footpadAdc1V: adc1,
      footpadAdc2V: adc2,
      setpointDeg: Double(Int(payload[14]) - 128) / 5,
      atrSetpointDeg: Double(Int(payload[15]) - 128) / 5,
      brakeTiltSetpointDeg: Double(Int(payload[16]) - 128) / 5,
      torqueTiltSetpointDeg: Double(Int(payload[17]) - 128) / 5,
      turnTiltSetpointDeg: Double(Int(payload[18]) - 128) / 5,
      remoteSetpointDeg: Double(Int(payload[19]) - 128) / 5,
      distanceM: detailLevel >= 2 ? decodeFloat32(payload, 35) : nil,
      odometerM: detailLevel >= 3 ? Double(unsigned32(42)) : nil,
      ampHoursDrawnAh: detailLevel >= 3 ? Double(signed16(46)) / 10 : nil,
      ampHoursChargedAh: detailLevel >= 3 ? Double(signed16(48)) / 10 : nil,
      wattHoursDrawnWh: detailLevel >= 3 ? Double(signed16(50)) : nil,
      wattHoursChargedWh: detailLevel >= 3 ? Double(signed16(52)) : nil,
      batteryLevel: detailLevel >= 3 ? Double(payload[54]) / 200 : nil,
      chargingCurrentA: detailLevel >= 4 ? Double(signed16(55)) / 10 : nil,
      chargingVoltageV: detailLevel >= 4 ? Double(signed16(57)) / 10 : nil
    )
  }

  private static func decodeFloat32(_ payload: [UInt8], _ index: Int) -> Double {
    let bits =
      UInt32(payload[index]) << 24 | UInt32(payload[index + 1]) << 16
        | UInt32(payload[index + 2]) << 8 | UInt32(payload[index + 3])
    return Double(Float(bitPattern: bits))
  }

  private static func legacySetpointAdjustment(_ value: UInt8) -> SetpointAdjustment {
    switch value {
    case 0: .centering
    case 1: .reverseStop
    case 2: .none
    case 3: .duty
    case 4: .highVoltage
    case 5: .lowVoltage
    case 6: .temperature
    default: .unknown
    }
  }

  private static func legacySemanticState(
    _ value: UInt8
  ) -> (packageState: PackageState, stopCondition: StopCondition) {
    switch value {
    case 0: (.startup, .none)
    case 1 ... 5: (.running, .none)
    case 6: (.ready, .pitch)
    case 7: (.ready, .roll)
    case 8: (.ready, .switchHalf)
    case 9: (.ready, .switchFull)
    case 12: (.ready, .reverseStop)
    case 13: (.ready, .quickStop)
    case 14: (.ready, .none)
    case 15: (.disabled, .none)
    default: (.ready, .none)
    }
  }
}
