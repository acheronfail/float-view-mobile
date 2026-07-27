import Foundation

public enum VescProtocolError: Error, Equatable {
  case invalidFrame
  case invalidCRC
  case payloadTooLarge
  case unsupportedCommand(UInt8)
  case truncatedPayload
}

public struct FirmwareInfo: Equatable, Sendable {
  public let major: UInt8
  public let minor: UInt8
  public let hardwareName: String
}

public struct RealtimeValues: Equatable, Sendable {
  public let controllerTempC: Double
  public let motorTempC: Double
  public let motorCurrentA: Double
  public let batteryCurrentA: Double
  public let dutyCycle: Double
  public let erpm: Int32
  public let packVoltageV: Double
  public let faultCode: UInt8
}

public struct RefloatRealtime: Equatable, Sendable {
  public let appId: UInt8
  public let packetId: UInt8
  public let rideState: UInt8
  public let footpadState: UInt8
}

public enum VescCRC {
  public static func checksum(_ bytes: some Sequence<UInt8>) -> UInt16 {
    var crc: UInt16 = 0
    for byte in bytes {
      crc ^= UInt16(byte) << 8
      for _ in 0..<8 {
        crc = (crc & 0x8000) != 0 ? (crc << 1) ^ 0x1021 : crc << 1
      }
    }
    return crc
  }
}

public enum VescFrameCodec {
  public static let maximumPayloadBytes = 4096

  public static func encode(_ payload: [UInt8]) throws -> [UInt8] {
    guard payload.count <= maximumPayloadBytes else { throw VescProtocolError.payloadTooLarge }
    var frame: [UInt8]
    if payload.count <= 255 {
      frame = [2, UInt8(payload.count)]
    } else {
      frame = [3, UInt8(payload.count >> 8), UInt8(payload.count & 0xff)]
    }
    frame.append(contentsOf: payload)
    let crc = VescCRC.checksum(payload)
    frame.append(UInt8(crc >> 8))
    frame.append(UInt8(crc & 0xff))
    frame.append(3)
    return frame
  }

  public static func decode(_ frame: [UInt8]) throws -> [UInt8] {
    guard let first = frame.first else { throw VescProtocolError.invalidFrame }
    let headerCount: Int
    let payloadCount: Int
    switch first {
    case 2:
      guard frame.count >= 5 else { throw VescProtocolError.invalidFrame }
      headerCount = 2
      payloadCount = Int(frame[1])
    case 3:
      guard frame.count >= 6 else { throw VescProtocolError.invalidFrame }
      headerCount = 3
      payloadCount = Int(frame[1]) << 8 | Int(frame[2])
    default:
      throw VescProtocolError.invalidFrame
    }
    guard payloadCount <= maximumPayloadBytes else { throw VescProtocolError.payloadTooLarge }
    let expectedCount = headerCount + payloadCount + 3
    guard frame.count == expectedCount, frame.last == 3 else {
      throw VescProtocolError.invalidFrame
    }
    let payload = Array(frame[headerCount..<(headerCount + payloadCount)])
    let received = UInt16(frame[headerCount + payloadCount]) << 8
      | UInt16(frame[headerCount + payloadCount + 1])
    guard VescCRC.checksum(payload) == received else { throw VescProtocolError.invalidCRC }
    return payload
  }
}

public struct VescFrameStreamDecoder: Sendable {
  private var buffer: [UInt8] = []
  public init() {}

  public mutating func append(_ bytes: some Sequence<UInt8>) throws -> [[UInt8]] {
    buffer.append(contentsOf: bytes)
    guard buffer.count <= VescFrameCodec.maximumPayloadBytes * 2 else {
      buffer.removeAll(keepingCapacity: true)
      throw VescProtocolError.payloadTooLarge
    }
    var payloads: [[UInt8]] = []
    while true {
      while let first = buffer.first, first != 2 && first != 3 { buffer.removeFirst() }
      guard let first = buffer.first else { return payloads }
      let headerCount = first == 2 ? 2 : 3
      guard buffer.count >= headerCount else { return payloads }
      let count = first == 2 ? Int(buffer[1]) : Int(buffer[1]) << 8 | Int(buffer[2])
      guard count <= VescFrameCodec.maximumPayloadBytes else {
        buffer.removeFirst()
        throw VescProtocolError.payloadTooLarge
      }
      let frameCount = headerCount + count + 3
      guard buffer.count >= frameCount else { return payloads }
      let frame = Array(buffer.prefix(frameCount))
      buffer.removeFirst(frameCount)
      payloads.append(try VescFrameCodec.decode(frame))
    }
  }
}

private struct ByteReader {
  let bytes: [UInt8]
  var offset = 0

  mutating func u8() throws -> UInt8 {
    guard offset < bytes.count else { throw VescProtocolError.truncatedPayload }
    defer { offset += 1 }
    return bytes[offset]
  }

  mutating func i16(scale: Double) throws -> Double {
    let raw = Int16(bitPattern: UInt16(try u8()) << 8 | UInt16(try u8()))
    return Double(raw) / scale
  }

  mutating func i32(scale: Double = 1) throws -> Double {
    var value: UInt32 = 0
    for _ in 0..<4 { value = value << 8 | UInt32(try u8()) }
    return Double(Int32(bitPattern: value)) / scale
  }

  mutating func skip(_ count: Int) throws {
    guard offset + count <= bytes.count else { throw VescProtocolError.truncatedPayload }
    offset += count
  }
}

public enum VescPayloadDecoder {
  public static func firmware(_ payload: [UInt8]) throws -> FirmwareInfo {
    guard payload.first == 0, payload.count >= 4 else {
      throw VescProtocolError.unsupportedCommand(payload.first ?? 255)
    }
    let nameBytes = payload.dropFirst(3).prefix { $0 != 0 }
    return FirmwareInfo(
      major: payload[1],
      minor: payload[2],
      hardwareName: String(decoding: nameBytes, as: UTF8.self)
    )
  }

  public static func realtime(_ payload: [UInt8]) throws -> RealtimeValues {
    var reader = ByteReader(bytes: payload)
    let command = try reader.u8()
    guard command == 4 else { throw VescProtocolError.unsupportedCommand(command) }
    let controllerTemp = try reader.i16(scale: 10)
    let motorTemp = try reader.i16(scale: 10)
    let motorCurrent = try reader.i32(scale: 100)
    let batteryCurrent = try reader.i32(scale: 100)
    try reader.skip(8)
    let duty = try reader.i16(scale: 1000)
    let erpm = Int32(try reader.i32())
    let packVoltage = try reader.i16(scale: 10)
    let fault = try reader.u8()
    return RealtimeValues(
      controllerTempC: controllerTemp,
      motorTempC: motorTemp,
      motorCurrentA: motorCurrent,
      batteryCurrentA: batteryCurrent,
      dutyCycle: duty,
      erpm: erpm,
      packVoltageV: packVoltage,
      faultCode: fault
    )
  }

  public static func refloatRealtime(_ payload: [UInt8]) throws -> RefloatRealtime {
    guard payload.count >= 5, payload[0] == 36 else {
      throw VescProtocolError.unsupportedCommand(payload.first ?? 255)
    }
    return RefloatRealtime(
      appId: payload[1],
      packetId: payload[2],
      rideState: payload[3],
      footpadState: payload[4]
    )
  }
}
