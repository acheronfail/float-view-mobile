import CVescRefloatDynamicTelemetry
import Foundation

struct RefloatDynamicTelemetryLayout: Equatable {
  let fields: [String]
  let runtimeFields: [String]
  let record: [UInt8]
}

/// Shared Apple adapter for Rust-owned Refloat command-31/32 decoding.
enum RefloatDynamicTelemetryDecoder {
  private static let expectedABI: UInt32 = 1
  private static let recordVersion: UInt8 = 1
  private static let maxFields = 64
  private static let maxFieldBytes = 64
  private static let maxExtensions = 32
  private static let fixedValueCount = 28
  private static let knownBehaviorFlags: UInt8 = 0x07
  private static let knownOptionFlags: UInt8 = 0x07

  static func decodeFields(_ payload: [UInt8]) throws -> RefloatDynamicTelemetryLayout {
    try requireCompatibleABI(float_view_refloat_dynamic_telemetry_abi_version())
    var record = [UInt8](
      repeating: 0,
      count: Int(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_LAYOUT_RECORD_BYTES)
    )
    let capacity = record.count
    var recordLength = 0
    let status = payload.withUnsafeBytes { payloadBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_refloat_dynamic_telemetry_decode_layout(
          payloadBytes.bindMemory(to: UInt8.self).baseAddress,
          payload.count,
          recordBytes.bindMemory(to: UInt8.self).baseAddress,
          capacity,
          &recordLength
        )
      }
    }
    try requireSuccess(status, recordLength: recordLength, capacity: record.count)
    return try layout(from: Array(record.prefix(recordLength)))
  }

  static func decodeData(
    _ payload: [UInt8],
    layout: RefloatDynamicTelemetryLayout
  ) throws -> RefloatTelemetryRecord {
    try requireCompatibleABI(float_view_refloat_dynamic_telemetry_abi_version())
    var record = [UInt8](
      repeating: 0,
      count: Int(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_SAMPLE_RECORD_BYTES)
    )
    let capacity = record.count
    var recordLength = 0
    let status = layout.record.withUnsafeBytes { layoutBytes in
      payload.withUnsafeBytes { payloadBytes in
        record.withUnsafeMutableBytes { recordBytes in
          float_view_refloat_dynamic_telemetry_decode_realtime(
            layoutBytes.bindMemory(to: UInt8.self).baseAddress,
            layout.record.count,
            payloadBytes.bindMemory(to: UInt8.self).baseAddress,
            payload.count,
            recordBytes.bindMemory(to: UInt8.self).baseAddress,
            capacity,
            &recordLength
          )
        }
      }
    }
    try requireSuccess(status, recordLength: recordLength, capacity: record.count)
    return try telemetry(from: Array(record.prefix(recordLength)))
  }

  static func requireCompatibleABI(_ actual: UInt32) throws {
    guard actual == expectedABI else {
      report("ABI mismatch")
      throw VescProtocolError.truncatedPayload
    }
  }

  static func layout(from record: [UInt8]) throws -> RefloatDynamicTelemetryLayout {
    guard record.count >= Int(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_LAYOUT_HEADER_BYTES),
          record.count <= Int(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_LAYOUT_RECORD_BYTES),
          record[0] == recordVersion,
          record[1] == UInt8(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_LAYOUT_RECORD_KIND),
          record[6] == 0,
          record[7] == 0
    else { throw VescProtocolError.truncatedPayload }
    let fieldsCount = Int(record[2])
    let runtimeCount = Int(record[3])
    let expectedAggregate = Int(UInt16(record[4]) << 8 | UInt16(record[5]))
    guard fieldsCount > 0,
          fieldsCount + runtimeCount <= maxFields,
          expectedAggregate <= 4_096
    else { throw VescProtocolError.truncatedPayload }
    var cursor = RecordCursor(record, index: 8)
    let fields = try cursor.readNames(count: fieldsCount, maxBytes: maxFieldBytes)
    let runtimeFields = try cursor.readNames(count: runtimeCount, maxBytes: maxFieldBytes)
    let all = fields + runtimeFields
    guard cursor.isAtEnd,
          all.reduce(0, { $0 + $1.utf8.count }) == expectedAggregate,
          Set(all).count == all.count
    else { throw VescProtocolError.truncatedPayload }
    return RefloatDynamicTelemetryLayout(
      fields: fields,
      runtimeFields: runtimeFields,
      record: record
    )
  }

  static func telemetry(from record: [UInt8]) throws -> RefloatTelemetryRecord {
    guard record.count >= Int(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_SAMPLE_FIXED_BYTES),
          record.count <= Int(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_SAMPLE_RECORD_BYTES),
          record[0] == recordVersion,
          record[1] == UInt8(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_SAMPLE_RECORD_KIND),
          record[7] & ~knownBehaviorFlags == 0,
          record[11] & ~knownOptionFlags == 0,
          record[13] == 0,
          record[14] == 0,
          record[15] == 0
    else { throw VescProtocolError.truncatedPayload }

    let packageState = try packageState(record[2])
    let packageMode = try packageMode(record[3])
    let footpadState = try footpadState(record[4])
    let stopCondition = try stopCondition(record[5])
    let setpointAdjustment = try setpointAdjustment(record[6])
    let alertReason = try alertReason(record[8])
    let behavior = record[7]
    let options = record[11]
    let extensionCount = Int(record[12])
    guard extensionCount <= maxExtensions else { throw VescProtocolError.truncatedPayload }

    let hasFatal = options & 0x01 != 0
    let fatalValue = options & 0x02 != 0
    let hasAlerts = options & 0x04 != 0
    guard hasFatal == hasAlerts, hasFatal || !fatalValue else {
      throw VescProtocolError.truncatedPayload
    }
    let word0 = uint32(record, at: 24)
    let word1 = uint32(record, at: 28)
    let rawFault = record[10]
    let faultCode = record[9]
    guard hasAlerts || (word0 == 0 && word1 == 0),
          (rawFault == 0) == (faultCode == 0),
          (rawFault != 0) == (word0 & 1 != 0),
          rawFault == 0 || faultCode == (rawFault <= 27 ? rawFault : 28)
    else { throw VescProtocolError.truncatedPayload }
    let firmwareFault = faultCode == 0 ? nil : try firmwareFault(faultCode)

    let presence = uint32(record, at: 20)
    guard presence >> UInt32(fixedValueCount) == 0 else {
      throw VescProtocolError.truncatedPayload
    }
    var cursor = RecordCursor(record, index: 32)
    var values = [Double?]()
    values.reserveCapacity(fixedValueCount)
    for index in 0 ..< fixedValueCount {
      let bits = try cursor.readUInt64()
      if presence & (UInt32(1) << UInt32(index)) == 0 {
        guard bits == 0 else { throw VescProtocolError.truncatedPayload }
        values.append(nil)
      } else {
        let value = Double(bitPattern: bits)
        guard value.isFinite else { throw VescProtocolError.truncatedPayload }
        values.append(value)
      }
    }

    var extensions: [String: Double] = [:]
    for _ in 0 ..< extensionCount {
      let name = try cursor.readName(maxBytes: maxFieldBytes)
      let value = try Double(bitPattern: cursor.readUInt64())
      guard value.isFinite, extensions[name] == nil else {
        throw VescProtocolError.truncatedPayload
      }
      extensions[name] = value
    }
    let charging = behavior & 0x04 != 0
    guard cursor.isAtEnd,
          charging == (values[26] != nil && values[27] != nil),
          (values[26] == nil) == (values[27] == nil)
    else { throw VescProtocolError.truncatedPayload }

    return RefloatTelemetryRecord(
      kind: .realtime,
      rideState: packageState == .running ? .riding : .idle,
      packageState: packageState,
      packageMode: packageMode,
      footpadState: footpadState,
      stopCondition: stopCondition,
      setpointAdjustment: setpointAdjustment,
      wheelslip: behavior & 0x01 != 0,
      darkride: behavior & 0x02 != 0,
      charging: charging,
      alertReason: alertReason,
      firmwareFault: firmwareFault,
      fatalErrorLatched: hasFatal ? fatalValue : nil,
      packVoltageV: values[6],
      dutyCycle: values[5],
      controllerTempC: values[8],
      motorTempC: values[9],
      motorCurrentA: values[2],
      batteryCurrentA: values[7],
      erpm: values[1],
      speedKph: values[0],
      pitchDeg: values[10],
      balancePitchDeg: values[11],
      rollDeg: values[12],
      directionalMotorCurrentA: values[3],
      filteredMotorCurrentA: values[4],
      requestedBalanceCurrentA: values[22],
      boosterCurrentA: values[25],
      footpadAdc1V: values[13],
      footpadAdc2V: values[14],
      remoteInput: values[15],
      setpointDeg: values[16],
      atrSetpointDeg: values[17],
      brakeTiltSetpointDeg: values[18],
      torqueTiltSetpointDeg: values[19],
      turnTiltSetpointDeg: values[20],
      remoteSetpointDeg: values[21],
      atrAccelerationDifferenceErpmPerSecond: values[23],
      atrSpeedBoost: values[24],
      chargingCurrentA: values[26],
      chargingVoltageV: values[27],
      extensions: extensions
    )
  }

  private static func requireSuccess(
    _ status: UInt32,
    recordLength: Int,
    capacity: Int
  ) throws {
    switch status {
    case UInt32(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_OK):
      guard recordLength <= capacity else {
        report("invalid result length")
        throw VescProtocolError.truncatedPayload
      }
    case UInt32(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_TRUNCATED_PAYLOAD):
      throw VescProtocolError.truncatedPayload
    case UInt32(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_UNSUPPORTED_ENVELOPE),
         UInt32(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_LAYOUT),
         UInt32(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_TELEMETRY):
      throw VescProtocolError.invalidFrame
    default:
      report("bridge status \(status)")
      throw VescProtocolError.truncatedPayload
    }
  }

  private static func packageState(_ code: UInt8) throws -> PackageState {
    try closed(code, [1: .disabled, 2: .startup, 3: .ready, 4: .running])
  }

  private static func packageMode(_ code: UInt8) throws -> PackageMode {
    try closed(code, [1: .normal, 2: .handTest, 3: .flywheel, 4: .unknown])
  }

  private static func footpadState(_ code: UInt8) throws -> FootpadState {
    try closed(code, [1: .none, 2: .left, 3: .right, 4: .both])
  }

  private static func stopCondition(_ code: UInt8) throws -> StopCondition {
    try closed(
      code,
      [
        1: .none, 2: .pitch, 3: .roll, 4: .switchHalf, 5: .switchFull,
        6: .reverseStop, 7: .quickStop, 8: .unknown,
      ]
    )
  }

  private static func setpointAdjustment(_ code: UInt8) throws -> SetpointAdjustment {
    try closed(
      code,
      [
        1: .none, 2: .centering, 3: .reverseStop, 4: .speed, 5: .duty,
        6: .error, 7: .highVoltage, 8: .lowVoltage, 9: .temperature, 10: .unknown,
      ]
    )
  }

  private static func alertReason(_ code: UInt8) throws -> AlertReason {
    try closed(
      code,
      [
        1: .none, 2: .lowVoltage, 3: .highVoltage, 4: .controllerTemperature,
        5: .motorTemperature, 6: .current, 7: .duty, 8: .sensors,
        9: .lowBattery, 10: .idle, 11: .error, 12: .speed,
        13: .cellUnderTemperature, 14: .cellOverTemperature, 15: .cellLowVoltage,
        16: .cellHighVoltage, 17: .cellImbalance, 18: .bmsConnection,
        19: .bmsOverTemperature, 20: .firmwareFault, 21: .unknown,
      ]
    )
  }

  private static func firmwareFault(_ code: UInt8) throws -> FirmwareFault {
    try closed(
      code,
      [
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
    )
  }

  private static func closed<T>(_ code: UInt8, _ values: [UInt8: T]) throws -> T {
    guard let value = values[code] else { throw VescProtocolError.truncatedPayload }
    return value
  }

  private static func uint32(_ bytes: [UInt8], at index: Int) -> UInt32 {
    UInt32(bytes[index]) << 24 | UInt32(bytes[index + 1]) << 16
      | UInt32(bytes[index + 2]) << 8 | UInt32(bytes[index + 3])
  }

  private static func report(_ reason: String) {
    print("[FloatView] Rust Refloat dynamic telemetry decode failed (\(reason))")
  }
}

private struct RecordCursor {
  private let bytes: [UInt8]
  private(set) var index: Int

  init(_ bytes: [UInt8], index: Int) {
    self.bytes = bytes
    self.index = index
  }

  var isAtEnd: Bool {
    index == bytes.count
  }

  mutating func readNames(count: Int, maxBytes: Int) throws -> [String] {
    try (0 ..< count).map { _ in try readName(maxBytes: maxBytes) }
  }

  mutating func readName(maxBytes: Int) throws -> String {
    let length = try Int(readUInt8())
    guard length > 0, length <= maxBytes, index <= bytes.count - length,
          let value = String(bytes: bytes[index ..< index + length], encoding: .utf8)
    else { throw VescProtocolError.truncatedPayload }
    index += length
    return value
  }

  mutating func readUInt8() throws -> UInt8 {
    guard index < bytes.count else { throw VescProtocolError.truncatedPayload }
    defer { index += 1 }
    return bytes[index]
  }

  mutating func readUInt64() throws -> UInt64 {
    guard index <= bytes.count - MemoryLayout<UInt64>.size else {
      throw VescProtocolError.truncatedPayload
    }
    var value: UInt64 = 0
    for byte in bytes[index ..< index + MemoryLayout<UInt64>.size] {
      value = value << 8 | UInt64(byte)
    }
    index += MemoryLayout<UInt64>.size
    return value
  }
}
