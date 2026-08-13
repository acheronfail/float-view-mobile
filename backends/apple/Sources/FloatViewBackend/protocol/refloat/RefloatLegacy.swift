import CVescRefloatLegacyTelemetry
import Foundation

/// Shared Apple adapter for the Rust-owned Refloat command-10 decoder.
enum RefloatLegacyTelemetryDecoder {
  private static let expectedABI: UInt32 = 1
  private static let recordVersion: UInt8 = 1
  private static let headerBytes = 16
  private static let sampleBytes = 264
  private static let activeFaultBytes = 16
  private static let valueCount = 31
  private static let knownBehaviorFlags: UInt8 = 0b111
  private static let knownOptionFlags: UInt16 = 0x0FFF

  static func decode(_ payload: [UInt8]) throws -> RefloatTelemetryRecord {
    try requireCompatibleABI(float_view_refloat_legacy_telemetry_abi_version())
    var record = [UInt8](repeating: 0, count: sampleBytes)
    let capacity = record.count
    var recordLength = 0
    var unsupportedCommand: UInt8 = 0
    let status = payload.withUnsafeBytes { payloadBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_refloat_legacy_telemetry_decode(
          payloadBytes.bindMemory(to: UInt8.self).baseAddress,
          payload.count,
          recordBytes.bindMemory(to: UInt8.self).baseAddress,
          capacity,
          &recordLength,
          &unsupportedCommand
        )
      }
    }
    switch status {
    case UInt32(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_OK):
      guard recordLength <= record.count else {
        report("invalid result length")
        throw VescProtocolError.truncatedPayload
      }
      return try telemetry(from: Array(record.prefix(recordLength)))
    case UInt32(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_UNSUPPORTED_ENVELOPE):
      throw VescProtocolError.unsupportedCommand(unsupportedCommand)
    case UInt32(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_TRUNCATED_PAYLOAD):
      throw VescProtocolError.truncatedPayload
    case UInt32(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_INVALID_PAYLOAD):
      throw VescProtocolError.invalidFrame
    default:
      report("bridge status \(status)")
      throw VescProtocolError.truncatedPayload
    }
  }

  static func requireCompatibleABI(_ actual: UInt32) throws {
    guard actual == expectedABI else {
      report("ABI mismatch")
      throw VescProtocolError.truncatedPayload
    }
  }

  static func telemetry(from record: [UInt8]) throws -> RefloatTelemetryRecord {
    guard record.count >= headerBytes,
          record[0] == recordVersion,
          record[14] == 0,
          record[15] == 0
    else { throw VescProtocolError.truncatedPayload }
    switch record[1] {
    case UInt8(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_RESULT_SAMPLE):
      return try sample(from: record)
    case UInt8(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_RESULT_ACTIVE_FIRMWARE_FAULT):
      return try activeFault(from: record)
    default:
      throw VescProtocolError.truncatedPayload
    }
  }

  private static func sample(from record: [UInt8]) throws -> RefloatTelemetryRecord {
    guard record.count == sampleBytes,
          (1 ... 4).contains(record[2]),
          record[8] & ~knownBehaviorFlags == 0,
          record[10] == 0,
          record[11] == 0
    else { throw VescProtocolError.truncatedPayload }
    let options = UInt16(record[12]) << 8 | UInt16(record[13])
    guard options & ~knownOptionFlags == 0 else { throw VescProtocolError.truncatedPayload }

    let packageState: PackageState = try closed(
      record[3],
      [
        1: .startup, 2: .running, 3: .ready, 4: .disabled,
      ]
    )
    let packageMode: PackageMode = try closed(
      record[4],
      [
        1: .normal, 2: .handTest, 3: .flywheel,
      ]
    )
    let footpadState: FootpadState = try closed(
      record[5],
      [
        1: .none, 2: .left, 3: .right, 4: .both,
      ]
    )
    let stopCondition: StopCondition = try closed(
      record[6],
      [
        1: .none, 2: .pitch, 3: .roll, 4: .switchHalf, 5: .switchFull,
        6: .reverseStop, 7: .quickStop,
      ]
    )
    let adjustment: SetpointAdjustment = try closed(
      record[7],
      [
        1: .centering, 2: .reverseStop, 3: .none, 4: .duty,
        5: .highVoltage, 6: .lowVoltage, 7: .temperature, 8: .unknown,
      ]
    )
    let alert: AlertReason = try closed(
      record[9],
      [
        1: .none, 2: .lowVoltage, 3: .highVoltage, 4: .controllerTemperature,
        5: .motorTemperature, 6: .current, 7: .duty, 8: .sensors,
        9: .lowBattery, 10: .idle, 11: .error, 12: .speed,
        13: .cellUnderTemperature, 14: .cellOverTemperature,
        15: .cellLowVoltage, 16: .cellHighVoltage, 17: .unknown,
      ]
    )

    var cursor = headerBytes
    var values: [Double] = []
    values.reserveCapacity(valueCount)
    for _ in 0 ..< valueCount {
      try values.append(double(from: record, cursor: &cursor))
    }
    guard cursor == record.count else { throw VescProtocolError.truncatedPayload }

    let optionFlags: [UInt16] = [
      0x001, 0x002, 0x004, 0x008, 0x010, 0x020,
      0x040, 0x080, 0x100, 0x200, 0x400, 0x800,
    ]
    let optionIndexes = [2, 3, 13, 22, 23, 24, 25, 26, 27, 28, 29, 30]
    for (flag, index) in zip(optionFlags, optionIndexes) {
      if options & flag == 0, values[index].bitPattern != 0 {
        throw VescProtocolError.truncatedPayload
      }
    }
    let detail = record[2]
    let required: UInt16 = detail == 1 ? 0 : detail == 2 ? 0x00B : detail == 3 ? 0x3FB : 0xFFB
    guard options & required == required,
          detail >= 2 || options & 0xFFB == 0,
          detail >= 3 || options & 0xFF0 == 0,
          detail >= 4 || options & 0xC00 == 0
    else { throw VescProtocolError.truncatedPayload }
    func optional(_ index: Int, _ flag: UInt16) -> Double? {
      options & flag == 0 ? nil : values[index]
    }

    return RefloatTelemetryRecord(
      kind: .compact,
      rideState: packageState == .running ? .riding : .idle,
      packageState: packageState,
      packageMode: packageMode,
      footpadState: footpadState,
      stopCondition: stopCondition,
      setpointAdjustment: adjustment,
      wheelslip: record[8] & 0x01 != 0,
      darkride: record[8] & 0x02 != 0,
      charging: record[8] & 0x04 != 0,
      alertReason: alert,
      packVoltageV: values[0], dutyCycle: values[1],
      controllerTempC: optional(2, 0x001), motorTempC: optional(3, 0x002),
      motorCurrentA: values[4], batteryCurrentA: values[5], erpm: values[6],
      speedKph: values[7], pitchDeg: values[8], balancePitchDeg: values[9],
      rollDeg: values[10], requestedBalanceCurrentA: values[11],
      boosterCurrentA: values[12], fieldWeakeningCurrentA: optional(13, 0x004),
      footpadAdc1V: values[14], footpadAdc2V: values[15],
      setpointDeg: values[16], atrSetpointDeg: values[17],
      brakeTiltSetpointDeg: values[18], torqueTiltSetpointDeg: values[19],
      turnTiltSetpointDeg: values[20], remoteSetpointDeg: values[21],
      distanceM: optional(22, 0x008), odometerM: optional(23, 0x010),
      ampHoursDrawnAh: optional(24, 0x020), ampHoursChargedAh: optional(25, 0x040),
      wattHoursDrawnWh: optional(26, 0x080), wattHoursChargedWh: optional(27, 0x100),
      batteryLevel: optional(28, 0x200), chargingCurrentA: optional(29, 0x400),
      chargingVoltageV: optional(30, 0x800)
    )
  }

  private static func activeFault(from record: [UInt8]) throws -> RefloatTelemetryRecord {
    guard record.count == activeFaultBytes,
          record[2 ..< 10].allSatisfy({ $0 == 0 }),
          record[11] != 0,
          record[12] == 0,
          record[13] == 0
    else { throw VescProtocolError.truncatedPayload }
    let faults: [UInt8: FirmwareFault] = [
      1: .overVoltage, 2: .underVoltage, 3: .driver, 4: .absoluteOverCurrent,
      5: .controllerOverTemperature, 6: .motorOverTemperature,
      7: .gateDriverOverVoltage, 8: .gateDriverUnderVoltage, 9: .mcuUnderVoltage,
      10: .watchdogReset, 11: .encoderSpi, 12: .encoderSinCosBelowMinimum,
      13: .encoderSinCosAboveMaximum, 14: .flashCorruption, 15: .currentSensorOffset1,
      16: .currentSensorOffset2, 17: .currentSensorOffset3, 18: .unbalancedCurrents,
      19: .brake, 20: .resolverLossOfTracking, 21: .resolverDegradationOfSignal,
      22: .resolverLossOfSignal, 23: .appConfigFlashCorruption,
      24: .motorConfigFlashCorruption, 25: .encoderNoMagnet,
      26: .encoderMagnetTooStrong, 27: .phaseFilter, 28: .unknown,
    ]
    guard let fault = faults[record[10]],
          (record[10] == 28) == (record[11] > 27)
    else { throw VescProtocolError.truncatedPayload }
    return RefloatTelemetryRecord(
      kind: .compact, rideState: .unknown, alertReason: .firmwareFault,
      firmwareFault: fault
    )
  }

  private static func closed<T>(_ code: UInt8, _ values: [UInt8: T]) throws -> T {
    guard let value = values[code] else { throw VescProtocolError.truncatedPayload }
    return value
  }

  private static func double(from record: [UInt8], cursor: inout Int) throws -> Double {
    guard cursor <= record.count - MemoryLayout<UInt64>.size else {
      throw VescProtocolError.truncatedPayload
    }
    var bits: UInt64 = 0
    for byte in record[cursor ..< cursor + MemoryLayout<UInt64>.size] {
      bits = bits << 8 | UInt64(byte)
    }
    cursor += MemoryLayout<UInt64>.size
    let value = Double(bitPattern: bits)
    guard value.isFinite else { throw VescProtocolError.truncatedPayload }
    return value
  }

  private static func report(_ reason: String) {
    print("[FloatView] Rust Refloat legacy telemetry decode failed (\(reason))")
  }
}
