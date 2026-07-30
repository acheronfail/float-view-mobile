import Foundation

enum RefloatDynamicTelemetryDecoder {
  private static let maxFields = 64
  private static let maxFieldBytes = 64
  private static let maxAggregateFieldBytes = 4_096
  private static let maxExtensions = 32

  static func decodeFields(
    _ payload: [UInt8]
  ) throws -> (fields: [String], runtimeFields: [String]) {
    guard payload.count >= 5 else { throw VescProtocolError.truncatedPayload }
    var index = 3
    var aggregateBytes = 0
    func readList() throws -> [String] {
      guard index < payload.count else { throw VescProtocolError.truncatedPayload }
      let count = Int(payload[index])
      index += 1
      return try (0..<count).map { _ in
        guard index < payload.count else { throw VescProtocolError.truncatedPayload }
        let length = Int(payload[index])
        index += 1
        guard length > 0, length <= maxFieldBytes, index + length <= payload.count else {
          throw VescProtocolError.invalidFrame
        }
        guard let value = String(bytes: payload[index..<(index + length)], encoding: .utf8) else {
          throw VescProtocolError.invalidFrame
        }
        index += length
        aggregateBytes += length
        guard aggregateBytes <= maxAggregateFieldBytes else {
          throw VescProtocolError.invalidFrame
        }
        return value
      }
    }
    let fields = try readList()
    let runtimeFields = try readList()
    let all = fields + runtimeFields
    guard index == payload.count, all.count <= maxFields, Set(all).count == all.count else {
      throw VescProtocolError.invalidFrame
    }
    return (fields, runtimeFields)
  }

  static func decodeData(
    _ payload: [UInt8],
    fields: [String],
    runtimeFields: [String]
  ) throws -> RefloatTelemetryRecord {
    guard payload.count >= 13, !fields.isEmpty else { throw VescProtocolError.invalidFrame }
    let mask = payload[3]
    let includedRuntimeFields = mask & 0x1 != 0 ? runtimeFields : []
    var expectedCount = 13 + (fields.count + includedRuntimeFields.count) * 2
    if mask & 0x2 != 0 { expectedCount += 4 }
    if mask & 0x4 != 0 { expectedCount += 9 }
    guard payload.count == expectedCount else { throw VescProtocolError.truncatedPayload }

    var index = 13
    var values: [String: Double] = [:]
    for field in fields + includedRuntimeFields {
      values[field] = try decodeFloat16(payload[index], payload[index + 1])
      index += 2
    }
    var chargingCurrent: Double?
    var chargingVoltage: Double?
    if mask & 0x2 != 0 {
      chargingCurrent = try decodeFloat16(payload[index], payload[index + 1])
      chargingVoltage = try decodeFloat16(payload[index + 2], payload[index + 3])
      index += 4
    }
    var firmwareFault: FirmwareFault?
    if mask & 0x4 != 0 {
      let activeAlerts =
        UInt32(payload[index]) << 24 | UInt32(payload[index + 1]) << 16
        | UInt32(payload[index + 2]) << 8 | UInt32(payload[index + 3])
      firmwareFault = activeAlerts & 0x1 != 0
        ? RefloatTelemetryMappings.firmwareFault(payload[index + 8]) : nil
    }

    let known = Set([
      "motor.speed", "motor.erpm", "motor.current", "motor.dir_current",
      "motor.filt_current", "motor.duty_cycle", "motor.batt_voltage",
      "motor.batt_current", "motor.mosfet_temp", "motor.motor_temp", "imu.pitch",
      "imu.balance_pitch", "imu.roll", "footpad.adc1", "footpad.adc2", "remote.input",
      "setpoint", "atr.setpoint", "brake_tilt.setpoint", "torque_tilt.setpoint",
      "turn_tilt.setpoint", "remote.setpoint", "balance_current", "atr.accel_diff",
      "atr.speed_boost", "booster.current",
    ])
    let extensions = values.filter { !known.contains($0.key) }
    guard extensions.count <= maxExtensions else { throw VescProtocolError.invalidFrame }

    let stateCode = payload[9] & 0x03
    let packageState = RefloatTelemetryMappings.packageState(stateCode)
    let flags = payload[10]
    return RefloatTelemetryRecord(
      kind: .realtime,
      rideState: packageState == .running ? .riding : .idle,
      packageState: packageState,
      packageMode: RefloatTelemetryMappings.packageMode(payload[9] >> 4 & 0x03),
      footpadState: RefloatTelemetryMappings.footpad(flags >> 6 & 0x03),
      stopCondition: RefloatTelemetryMappings.stopCondition(payload[11] & 0x0f),
      setpointAdjustment: RefloatTelemetryMappings.setpointAdjustment(payload[11] >> 4),
      wheelslip: flags & 0x01 != 0,
      darkride: flags & 0x02 != 0,
      charging: flags & 0x20 != 0,
      alertReason: RefloatTelemetryMappings.alertReason(payload[12]),
      firmwareFault: firmwareFault,
      packVoltageV: values["motor.batt_voltage"],
      dutyCycle: values["motor.duty_cycle"],
      controllerTempC: values["motor.mosfet_temp"],
      motorTempC: values["motor.motor_temp"],
      motorCurrentA: values["motor.current"],
      batteryCurrentA: values["motor.batt_current"],
      erpm: values["motor.erpm"],
      speedKph: values["motor.speed"],
      pitchDeg: values["imu.pitch"],
      balancePitchDeg: values["imu.balance_pitch"],
      rollDeg: values["imu.roll"],
      directionalMotorCurrentA: values["motor.dir_current"],
      filteredMotorCurrentA: values["motor.filt_current"],
      requestedBalanceCurrentA: values["balance_current"],
      boosterCurrentA: values["booster.current"],
      footpadAdc1V: values["footpad.adc1"],
      footpadAdc2V: values["footpad.adc2"],
      remoteInput: values["remote.input"],
      setpointDeg: values["setpoint"],
      atrSetpointDeg: values["atr.setpoint"],
      brakeTiltSetpointDeg: values["brake_tilt.setpoint"],
      torqueTiltSetpointDeg: values["torque_tilt.setpoint"],
      turnTiltSetpointDeg: values["turn_tilt.setpoint"],
      remoteSetpointDeg: values["remote.setpoint"],
      atrAccelerationDifferenceErpmPerSecond: values["atr.accel_diff"],
      atrSpeedBoost: values["atr.speed_boost"],
      chargingCurrentA: chargingCurrent,
      chargingVoltageV: chargingVoltage,
      extensions: extensions
    )
  }

  private static func decodeFloat16(_ high: UInt8, _ low: UInt8) throws -> Double {
    let bits = UInt16(high) << 8 | UInt16(low)
    let sign = bits & 0x8000 == 0 ? 1.0 : -1.0
    let exponent = Int(bits >> 10 & 0x1f)
    let fraction = Int(bits & 0x03ff)
    guard exponent != 0x1f else { throw VescProtocolError.invalidFrame }
    let value = exponent == 0
      ? sign * Double(fraction) * pow(2, -24)
      : sign * (1 + Double(fraction) / 1024) * pow(2, Double(exponent - 15))
    guard value.isFinite else { throw VescProtocolError.invalidFrame }
    return value
  }
}
