import Foundation

public enum RefloatTelemetryFamily: Sendable, Equatable {
  case legacyFixed
  case dynamic
}

public enum RefloatWireProfile: Sendable, Equatable {
  case legacyFixed
  case dynamic
  case dynamicAlerts

  public var family: RefloatTelemetryFamily {
    self == .legacyFixed ? .legacyFixed : .dynamic
  }
}

public struct RefloatCompatibilityEntry: Sendable, Equatable {
  public let sourceVersion: String
  public let wireVersion: String
  public let profile: RefloatWireProfile

  public var family: RefloatTelemetryFamily {
    profile.family
  }
}

public enum RefloatCompatibility {
  public static let entries: [RefloatCompatibilityEntry] = [
    .init(sourceVersion: "1.0.0", wireVersion: "1.0", profile: .legacyFixed),
    .init(sourceVersion: "1.0.1", wireVersion: "1.0", profile: .legacyFixed),
    .init(sourceVersion: "1.1.0", wireVersion: "1.1.0", profile: .dynamic),
    .init(sourceVersion: "1.1.1", wireVersion: "1.1.1", profile: .dynamic),
    .init(sourceVersion: "1.1.2", wireVersion: "1.1.2", profile: .dynamic),
    .init(sourceVersion: "1.2.0", wireVersion: "1.2.0", profile: .dynamicAlerts),
    .init(sourceVersion: "1.2.1", wireVersion: "1.2.1", profile: .dynamicAlerts),
    .init(sourceVersion: "1.2.2", wireVersion: "1.2.2", profile: .dynamicAlerts),
    .init(sourceVersion: "1.2.3", wireVersion: "1.2.3", profile: .dynamicAlerts),
  ]

  public static func entry(wireVersion: String) -> RefloatCompatibilityEntry? {
    entries.first { $0.wireVersion == wireVersion }
  }

  public static let requiredFields = [
    "motor.speed", "motor.erpm", "motor.current", "motor.dir_current",
    "motor.filt_current", "motor.duty_cycle", "motor.batt_voltage",
    "motor.batt_current", "motor.mosfet_temp", "motor.motor_temp", "imu.pitch",
    "imu.balance_pitch", "imu.roll", "footpad.adc1", "footpad.adc2", "remote.input",
  ]

  public static let requiredRuntimeFields = [
    "setpoint", "atr.setpoint", "brake_tilt.setpoint", "torque_tilt.setpoint",
    "turn_tilt.setpoint", "remote.setpoint", "balance_current", "atr.accel_diff",
    "atr.speed_boost", "booster.current",
  ]

  public static func supportsDynamicLayout(
    fields: [String],
    runtimeFields: [String]
  ) -> Bool {
    Set(requiredFields).isSubset(of: Set(fields))
      && Set(requiredRuntimeFields).isSubset(of: Set(runtimeFields))
  }
}
