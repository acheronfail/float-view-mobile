import CVescCustomConfiguration
import Foundation

public enum VescCustomConfigurationResponse: Sendable, Equatable {
  case schemaChunk(index: UInt8, totalLength: Int, offset: Int, bytes: Data)
  case values(command: VescCommand, index: UInt8, bytes: Data)
  case writeAcknowledgement
}

public enum VescCustomConfigurationCodec {
  public static let index: UInt8 = 0
  public static let maximumChunkBytes = 400

  private static let expectedABI: UInt32 = 1
  private static let resultRecordBytes = 28
  private static let resultRecordVersion: UInt8 = 1

  public static func schemaRequest(index: UInt8 = index, length: Int, offset: Int) throws -> [UInt8] {
    try encode(
      operation: UInt8(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_SCHEMA),
      index: index,
      length: length,
      offset: offset,
      values: Data(),
      outputCapacity: 10
    )
  }

  public static func valuesRequest(index: UInt8 = index, defaults: Bool) throws -> [UInt8] {
    try encode(
      operation: UInt8(defaults
        ? FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_DEFAULT
        : FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_CURRENT),
      index: index,
      length: 0,
      offset: 0,
      values: Data(),
      outputCapacity: 2
    )
  }

  public static func writeRequest(index: UInt8 = index, values: Data) throws -> [UInt8] {
    try encode(
      operation: UInt8(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_WRITE),
      index: index,
      length: 0,
      offset: 0,
      values: values,
      outputCapacity: values.count + 2
    )
  }

  public static func decode(_ payload: [UInt8]) throws -> VescCustomConfigurationResponse {
    try requireCompatibleABI(float_view_vesc_custom_configuration_abi_version())
    var record = [UInt8](repeating: 0, count: resultRecordBytes)
    let recordCapacity = record.count
    var recordLength = 0
    var unsupportedCommand: UInt8 = 0
    let status = payload.withUnsafeBytes { payloadBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_vesc_custom_configuration_decode(
          payloadBytes.bindMemory(to: UInt8.self).baseAddress,
          payload.count,
          recordBytes.bindMemory(to: UInt8.self).baseAddress,
          recordCapacity,
          &recordLength,
          &unsupportedCommand
        )
      }
    }
    switch status {
    case UInt32(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OK):
      guard recordLength == resultRecordBytes else {
        report("invalid result length")
        throw VescProtocolError.truncatedPayload
      }
      do {
        return try response(from: record, payload: payload)
      } catch {
        report("invalid result record")
        throw error
      }
    case UInt32(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_UNSUPPORTED_COMMAND):
      throw VescProtocolError.unsupportedCommand(unsupportedCommand)
    case UInt32(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INVALID_PAYLOAD):
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

  static func response(
    from record: [UInt8], payload: [UInt8]
  ) throws -> VescCustomConfigurationResponse {
    guard record.count == resultRecordBytes,
          record[0] == resultRecordVersion,
          let total = Int(exactly: u64(record, at: 4)),
          let offset = Int(exactly: u64(record, at: 12)),
          let dataOffset = Int(exactly: u32(record, at: 20)),
          let dataLength = Int(exactly: u32(record, at: 24)),
          dataOffset <= payload.count,
          dataLength <= payload.count - dataOffset
    else { throw VescProtocolError.truncatedPayload }
    let opaque = Data(payload[dataOffset ..< dataOffset + dataLength])
    switch record[1] {
    case UInt8(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_SCHEMA):
      guard record[2] == VescCommand.getCustomConfigurationXML.rawValue else {
        throw VescProtocolError.truncatedPayload
      }
      return .schemaChunk(index: record[3], totalLength: total, offset: offset, bytes: opaque)
    case UInt8(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_CURRENT),
         UInt8(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_DEFAULT):
      guard total == 0, offset == 0, let command = VescCommand(rawValue: record[2]),
            command == .getCustomConfiguration || command == .getCustomConfigurationDefault
      else { throw VescProtocolError.truncatedPayload }
      return .values(command: command, index: record[3], bytes: opaque)
    case UInt8(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_WRITE_ACKNOWLEDGEMENT):
      guard record[2] == VescCommand.setCustomConfiguration.rawValue,
            record[3] == 0, total == 0, offset == 0, dataLength == 0
      else { throw VescProtocolError.truncatedPayload }
      return .writeAcknowledgement
    default:
      throw VescProtocolError.truncatedPayload
    }
  }

  private static func encode(
    operation: UInt8,
    index: UInt8,
    length: Int,
    offset: Int,
    values: Data,
    outputCapacity: Int
  ) throws -> [UInt8] {
    try requireCompatibleABI(float_view_vesc_custom_configuration_abi_version())
    guard let bridgeLength = Int64(exactly: length), let bridgeOffset = Int64(exactly: offset),
          outputCapacity >= 0,
          outputCapacity <= Int(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_MAX_PAYLOAD_BYTES)
    else { throw VescProtocolError.truncatedPayload }
    var output = [UInt8](repeating: 0, count: outputCapacity)
    let capacity = output.count
    var outputLength = 0
    let status = values.withUnsafeBytes { valueBytes in
      output.withUnsafeMutableBytes { outputBytes in
        float_view_vesc_custom_configuration_encode(
          operation,
          index,
          bridgeLength,
          bridgeOffset,
          valueBytes.bindMemory(to: UInt8.self).baseAddress,
          values.count,
          outputBytes.bindMemory(to: UInt8.self).baseAddress,
          capacity,
          &outputLength
        )
      }
    }
    guard status == UInt32(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OK),
          outputLength <= output.count
    else {
      if status != UInt32(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INVALID_PAYLOAD) {
        report("bridge status \(status)")
      }
      throw VescProtocolError.truncatedPayload
    }
    return Array(output.prefix(outputLength))
  }

  private static func u64(_ record: [UInt8], at offset: Int) -> UInt64 {
    record[offset ..< offset + 8].reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
  }

  private static func u32(_ record: [UInt8], at offset: Int) -> UInt32 {
    record[offset ..< offset + 4].reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
  }

  private static func report(_ reason: String) {
    NSLog("[FloatView] Rust custom configuration codec failed (%@)", reason)
  }
}
