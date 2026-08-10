import CVescControllerData
import Foundation

/// Shared Apple adapter for Rust-owned command-4 and command-14 decoding.
public enum VescControllerDataDecoder {
  private static let expectedABI: UInt32 = 1
  private static let recordVersion: UInt8 = 1
  private static let recordHeaderBytes = 8
  private static let maximumRecordBytes = 60
  private static let seriesCellsPresent: UInt8 = 1

  public static func realtime(_ payload: [UInt8]) throws -> RealtimeValues {
    try realtime(from: decode(
      UInt32(FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_REALTIME), payload: payload
    ))
  }

  public static func currentLimits(_ payload: [UInt8]) throws -> ControllerCurrentLimits {
    try currentLimits(from: decode(
      UInt32(FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_CURRENT_LIMITS), payload: payload
    ))
  }

  private static func decode(_ operation: UInt32, payload: [UInt8]) throws -> [UInt8] {
    try requireCompatibleABI(float_view_vesc_controller_data_abi_version())

    var record = [UInt8](repeating: 0, count: maximumRecordBytes)
    let capacity = record.count
    var recordLength = 0
    var unsupportedCommand: UInt8 = 0
    let status = payload.withUnsafeBytes { payloadBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_vesc_controller_data_decode(
          operation,
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
    case UInt32(FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_OK):
      guard recordLength <= record.count else {
        report("invalid result length")
        throw VescProtocolError.truncatedPayload
      }
      return Array(record.prefix(recordLength))
    case UInt32(FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_UNSUPPORTED_COMMAND):
      throw VescProtocolError.unsupportedCommand(unsupportedCommand)
    case UInt32(FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_TRUNCATED_PAYLOAD):
      throw VescProtocolError.truncatedPayload
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

  static func realtime(from record: [UInt8]) throws -> RealtimeValues {
    guard record.count == Int(FLOAT_VIEW_VESC_CONTROLLER_DATA_REALTIME_RECORD_BYTES),
          record[0] == recordVersion,
          record[1] == UInt8(FLOAT_VIEW_VESC_CONTROLLER_DATA_RESULT_REALTIME),
          record[2] == 0,
          record[5] == 0,
          record[6] == 0,
          record[7] == 0
    else { throw VescProtocolError.truncatedPayload }

    let fault: VescFault.Kind = switch record[3] {
    case UInt8(FLOAT_VIEW_VESC_CONTROLLER_DATA_FAULT_NONE) where record[4] == 0: .none
    case UInt8(FLOAT_VIEW_VESC_CONTROLLER_DATA_FAULT_OVER_VOLTAGE) where record[4] == 1:
      .overVoltage
    case UInt8(FLOAT_VIEW_VESC_CONTROLLER_DATA_FAULT_UNKNOWN) where record[4] > 1: .unknown
    default: throw VescProtocolError.truncatedPayload
    }
    var cursor = recordHeaderBytes
    let controllerTemp = try double(from: record, cursor: &cursor)
    let motorTemp = try double(from: record, cursor: &cursor)
    let motorCurrent = try double(from: record, cursor: &cursor)
    let batteryCurrent = try double(from: record, cursor: &cursor)
    let duty = try double(from: record, cursor: &cursor)
    let packVoltage = try double(from: record, cursor: &cursor)
    let erpm = try int32(from: record, cursor: &cursor)
    guard cursor == record.count else { throw VescProtocolError.truncatedPayload }
    return RealtimeValues(
      controllerTempC: controllerTemp,
      motorTempC: motorTemp,
      motorCurrentA: motorCurrent,
      batteryCurrentA: batteryCurrent,
      dutyCycle: duty,
      erpm: erpm,
      packVoltageV: packVoltage,
      faultCode: record[4],
      fault: VescFault(code: record[4], kind: fault)
    )
  }

  static func currentLimits(from record: [UInt8]) throws -> ControllerCurrentLimits {
    guard record.count == Int(FLOAT_VIEW_VESC_CONTROLLER_DATA_CURRENT_LIMITS_RECORD_BYTES),
          record[0] == recordVersion,
          record[1] == UInt8(FLOAT_VIEW_VESC_CONTROLLER_DATA_RESULT_CURRENT_LIMITS),
          record[2] & ~seriesCellsPresent == 0,
          record[3] == 0,
          record[4] == 0,
          record[6] == 0,
          record[7] == 0
    else { throw VescProtocolError.truncatedPayload }
    let hasCells = record[2] & seriesCellsPresent != 0
    guard hasCells == (record[5] > 0) else { throw VescProtocolError.truncatedPayload }

    var cursor = recordHeaderBytes
    let values = try (0 ..< 4).map { _ in try double(from: record, cursor: &cursor) }
    guard cursor == record.count, values.allSatisfy({ abs($0) > 0 }) else {
      throw VescProtocolError.truncatedPayload
    }
    return ControllerCurrentLimits(
      motorCurrentMaxA: values[0],
      motorCurrentMinA: values[1],
      batteryCurrentMaxA: values[2],
      batteryCurrentMinA: values[3],
      batterySeriesCells: hasCells ? record[5] : nil
    )
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

  private static func int32(from record: [UInt8], cursor: inout Int) throws -> Int32 {
    guard cursor <= record.count - MemoryLayout<UInt32>.size else {
      throw VescProtocolError.truncatedPayload
    }
    var bits: UInt32 = 0
    for byte in record[cursor ..< cursor + MemoryLayout<UInt32>.size] {
      bits = bits << 8 | UInt32(byte)
    }
    cursor += MemoryLayout<UInt32>.size
    return Int32(bitPattern: bits)
  }

  private static func report(_ reason: String) {
    print("[FloatView] Rust controller-data decode failed (\(reason))")
  }
}
