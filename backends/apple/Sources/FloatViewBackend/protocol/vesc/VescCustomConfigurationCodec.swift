import Foundation

public enum VescCustomConfigurationResponse: Sendable, Equatable {
  case schemaChunk(index: UInt8, totalLength: Int, offset: Int, bytes: Data)
  case values(command: VescCommand, index: UInt8, bytes: Data)
  case writeAcknowledgement
}

public enum VescCustomConfigurationCodec {
  public static let index: UInt8 = 0
  public static let maximumChunkBytes = 400

  public static func schemaRequest(index: UInt8 = index, length: Int, offset: Int) throws -> [UInt8] {
    guard length > 0, length <= maximumChunkBytes, offset >= 0, offset <= Int(Int32.max) else {
      throw VescProtocolError.truncatedPayload
    }
    return [VescCommand.getCustomConfigurationXML.rawValue, index] + i32(length) + i32(offset)
  }

  public static func valuesRequest(index: UInt8 = index, defaults: Bool) -> [UInt8] {
    [defaults ? VescCommand.getCustomConfigurationDefault.rawValue : VescCommand.getCustomConfiguration.rawValue, index]
  }

  public static func writeRequest(index: UInt8 = index, values: Data) throws -> [UInt8] {
    guard !values.isEmpty, values.count <= VescScheduledRequest.maxPayloadBytes - 2 else {
      throw VescProtocolError.truncatedPayload
    }
    return [VescCommand.setCustomConfiguration.rawValue, index] + values
  }

  public static func decode(_ payload: [UInt8]) throws -> VescCustomConfigurationResponse {
    guard let rawCommand = payload.first, let command = VescCommand(rawValue: rawCommand) else {
      throw VescProtocolError.truncatedPayload
    }
    switch command {
    case .getCustomConfigurationXML:
      guard payload.count >= 10 else { throw VescProtocolError.truncatedPayload }
      let total = try positiveI32(payload, at: 2)
      let offset = try nonnegativeI32(payload, at: 6)
      let bytes = Data(payload.dropFirst(10))
      guard bytes.count <= maximumChunkBytes, offset <= total, bytes.count <= total - offset else {
        throw VescProtocolError.truncatedPayload
      }
      return .schemaChunk(index: payload[1], totalLength: total, offset: offset, bytes: bytes)
    case .getCustomConfiguration, .getCustomConfigurationDefault:
      guard payload.count >= 6 else { throw VescProtocolError.truncatedPayload }
      return .values(command: command, index: payload[1], bytes: Data(payload.dropFirst(2)))
    case .setCustomConfiguration:
      guard payload.count == 1 else { throw VescProtocolError.truncatedPayload }
      return .writeAcknowledgement
    default:
      throw VescProtocolError.unsupportedCommand(rawCommand)
    }
  }

  private static func i32(_ value: Int) -> [UInt8] {
    let raw = UInt32(value)
    return [
      UInt8(raw >> 24),
      UInt8(truncatingIfNeeded: raw >> 16),
      UInt8(truncatingIfNeeded: raw >> 8),
      UInt8(truncatingIfNeeded: raw),
    ]
  }

  private static func positiveI32(_ bytes: [UInt8], at offset: Int) throws -> Int {
    let value = try nonnegativeI32(bytes, at: offset)
    guard value > 0 else { throw VescProtocolError.truncatedPayload }
    return value
  }

  private static func nonnegativeI32(_ bytes: [UInt8], at offset: Int) throws -> Int {
    guard offset + 4 <= bytes.count else { throw VescProtocolError.truncatedPayload }
    let value = bytes[offset ..< offset + 4].reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
    guard value <= UInt32(Int32.max) else { throw VescProtocolError.truncatedPayload }
    return Int(value)
  }
}
