@testable import FloatViewBackend

let refloatRequiredFields = [
  "motor.speed", "motor.erpm", "motor.current", "motor.dir_current",
  "motor.filt_current", "motor.duty_cycle", "motor.batt_voltage",
  "motor.batt_current", "motor.mosfet_temp", "motor.motor_temp", "imu.pitch",
  "imu.balance_pitch", "imu.roll", "footpad.adc1", "footpad.adc2", "remote.input",
]

let refloatRequiredRuntimeFields = [
  "setpoint", "atr.setpoint", "brake_tilt.setpoint", "torque_tilt.setpoint",
  "turn_tilt.setpoint", "remote.setpoint", "balance_current", "atr.accel_diff",
  "atr.speed_boost", "booster.current",
]

func refloatInfoPayload(_ version: String) throws -> [UInt8] {
  if version == "1.0" { return [36, 101, 0, 10] }
  let components = version.split(separator: ".").compactMap { UInt8($0) }
  guard components.count == 3 else { throw VescProtocolError.invalidFrame }
  var payload = [UInt8](repeating: 0, count: 61)
  payload[0 ... 4] = [36, 101, 0, 2, 0]
  payload[25 ... 27] = components[0 ... 2]
  return payload
}

func refloatDecision(_ version: String) throws -> RefloatCompatibilityDecision {
  try RefloatCompatibility.decodeInfo(refloatInfoPayload(version))
}
