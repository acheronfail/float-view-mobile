import CVescBmsValues
import Foundation

/// Shared Apple adapter for the Rust-owned COMM_BMS_GET_VALUES structural decoder.
public enum VescBmsValuesDecoder {
  private static let expectedABI: UInt32 = 1
  private static let recordVersion: UInt8 = 1
  private static let recordHeaderBytes = 8
  private static let fixedValueCount = 17
  private static let maximumCount = 50
  private static let maximumStatusBytes = 40
  private static let maximumRecordBytes = 1_034
  private static let extendedTailPresent: UInt8 = 1
  private static let knownFlags = extendedTailPresent

  public static func decode(
    _ payload: [UInt8], expectedCanID: UInt8?, observedAt: Date
  ) throws -> BmsSnapshot {
    try requireCompatibleABI(float_view_vesc_bms_values_abi_version())

    var record = [UInt8](repeating: 0, count: maximumRecordBytes)
    let recordCapacity = record.count
    var recordLength = 0
    var unsupportedCommand: UInt8 = 0
    let expected = expectedCanID ?? UInt8.max
    let status = payload.withUnsafeBytes { payloadBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_vesc_bms_values_decode(
          payloadBytes.bindMemory(to: UInt8.self).baseAddress,
          payload.count,
          expected,
          recordBytes.bindMemory(to: UInt8.self).baseAddress,
          recordCapacity,
          &recordLength,
          &unsupportedCommand
        )
      }
    }

    switch status {
    case UInt32(FLOAT_VIEW_VESC_BMS_VALUES_STATUS_OK):
      guard recordLength <= record.count else {
        report("invalid result length")
        throw VescProtocolError.truncatedPayload
      }
      return try snapshot(from: Array(record.prefix(recordLength)), observedAt: observedAt)
    case UInt32(FLOAT_VIEW_VESC_BMS_VALUES_STATUS_UNSUPPORTED_COMMAND):
      throw VescProtocolError.unsupportedCommand(unsupportedCommand)
    case UInt32(FLOAT_VIEW_VESC_BMS_VALUES_STATUS_TRUNCATED_PAYLOAD):
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

  static func snapshot(from record: [UInt8], observedAt: Date) throws -> BmsSnapshot {
    let fixedBytes = fixedValueCount * MemoryLayout<UInt64>.size
    guard record.count >= recordHeaderBytes + fixedBytes,
          record.count <= maximumRecordBytes,
          record[0] == recordVersion,
          record[1] & ~knownFlags == 0,
          record[2] <= maximumCount,
          record[3] <= maximumCount,
          record[4] <= 253,
          record[6] <= maximumStatusBytes,
          record[7] == 0
    else { throw VescProtocolError.truncatedPayload }

    let extended = record[1] & extendedTailPresent != 0
    let statusLength = Int(record[6])
    guard extended || (record[5] == 0 && statusLength == 0) else {
      throw VescProtocolError.truncatedPayload
    }
    let cellCount = Int(record[2])
    let temperatureCount = Int(record[3])
    let expectedLength = recordHeaderBytes + fixedBytes
      + cellCount * MemoryLayout<UInt64>.size + cellCount
      + temperatureCount * MemoryLayout<UInt64>.size + statusLength
    guard expectedLength == record.count else { throw VescProtocolError.truncatedPayload }

    var cursor = recordHeaderBytes
    var fixed: [Double] = []
    fixed.reserveCapacity(fixedValueCount)
    for _ in 0 ..< fixedValueCount {
      fixed.append(try double(from: record, cursor: &cursor))
    }
    var cells: [Double] = []
    cells.reserveCapacity(cellCount)
    for _ in 0 ..< cellCount {
      cells.append(try double(from: record, cursor: &cursor))
    }
    var balancing: [Bool] = []
    balancing.reserveCapacity(cellCount)
    for _ in 0 ..< cellCount {
      switch record[cursor] {
      case 0: balancing.append(false)
      case 1: balancing.append(true)
      default: throw VescProtocolError.truncatedPayload
      }
      cursor += 1
    }
    var temperatures: [Double] = []
    temperatures.reserveCapacity(temperatureCount)
    for _ in 0 ..< temperatureCount {
      temperatures.append(try double(from: record, cursor: &cursor))
    }
    let status: String?
    if extended {
      guard let decoded = String(bytes: record[cursor ..< record.count], encoding: .utf8) else {
        throw VescProtocolError.truncatedPayload
      }
      status = decoded
    } else {
      status = nil
    }

    return BmsSnapshot(
      observedAt: observedAt,
      totalVoltageV: fixed[0], chargeVoltageV: fixed[1],
      inputCurrentA: fixed[2], inputICCurrentA: fixed[3],
      sessionAmpHoursAh: fixed[4], sessionWattHoursWh: fixed[5],
      cellVoltagesV: cells, balancing: balancing, temperaturesC: temperatures,
      icTemperatureC: fixed[6], humiditySensorTemperatureC: fixed[7],
      humidityPercent: fixed[8], highestCellTemperatureC: fixed[9],
      stateOfCharge: fixed[10], stateOfHealth: fixed[11],
      reportedCanID: record[4], lifetimeChargeAmpHoursAh: fixed[12],
      lifetimeChargeWattHoursWh: fixed[13], lifetimeDischargeAmpHoursAh: fixed[14],
      lifetimeDischargeWattHoursWh: fixed[15], pressurePa: fixed[16],
      dataVersion: extended ? record[5] : nil, status: status
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

  private static func report(_ reason: String) {
    print("[FloatView] Rust BMS values decode failed (\(reason))")
  }
}
