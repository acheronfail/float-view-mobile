import Foundation

enum RefloatTelemetryMappings {
  static func packageState(_ code: UInt8) -> PackageState {
    [.disabled, .startup, .ready, .running][safe: Int(code)] ?? .unknown
  }

  static func packageMode(_ code: UInt8) -> PackageMode {
    [.normal, .handTest, .flywheel][safe: Int(code)] ?? .unknown
  }

  static func footpad(_ code: UInt8) -> FootpadState {
    [.none, .left, .right, .both][safe: Int(code)] ?? .unknown
  }

  static func stopCondition(_ code: UInt8) -> StopCondition {
    [.none, .pitch, .roll, .switchHalf, .switchFull, .reverseStop, .quickStop][
      safe: Int(code)
    ] ?? .unknown
  }

  static func setpointAdjustment(_ code: UInt8) -> SetpointAdjustment {
    switch code {
    case 0: .none
    case 1: .centering
    case 2: .reverseStop
    case 5: .speed
    case 6: .duty
    case 7: .error
    case 10: .highVoltage
    case 11: .lowVoltage
    case 12: .temperature
    default: .unknown
    }
  }

  static func alertReason(_ code: UInt8) -> AlertReason {
    [
      .none, .lowVoltage, .highVoltage, .controllerTemperature, .motorTemperature,
      .current, .duty, .sensors, .lowBattery, .idle, .error, .speed,
      .cellUnderTemperature, .cellOverTemperature, .cellLowVoltage, .cellHighVoltage,
      .cellImbalance, .bmsConnection, .bmsOverTemperature, .firmwareFault,
    ][safe: Int(code)] ?? .unknown
  }

  static func firmwareFault(_ code: UInt8) -> FirmwareFault? {
    guard code != 0 else { return nil }
    return [
      .unknown, .overVoltage, .underVoltage, .driver, .absoluteOverCurrent,
      .controllerOverTemperature, .motorOverTemperature, .gateDriverOverVoltage,
      .gateDriverUnderVoltage, .mcuUnderVoltage, .watchdogReset, .encoderSpi,
      .encoderSinCosBelowMinimum, .encoderSinCosAboveMaximum, .flashCorruption,
      .currentSensorOffset1, .currentSensorOffset2, .currentSensorOffset3,
      .unbalancedCurrents, .brake, .resolverLossOfTracking, .resolverDegradationOfSignal,
      .resolverLossOfSignal, .appConfigFlashCorruption, .motorConfigFlashCorruption,
      .encoderNoMagnet, .encoderMagnetTooStrong, .phaseFilter,
    ][safe: Int(code)] ?? .unknown
  }
}

private extension Array {
  subscript(safe index: Int) -> Element? {
    indices.contains(index) ? self[index] : nil
  }
}
