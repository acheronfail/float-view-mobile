import Foundation
#if canImport(FloatViewCore)
import FloatViewCore
#endif

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
  public let firmwareName: String?
  public let hardwareType: UInt8

  public init(
    major: UInt8,
    minor: UInt8,
    hardwareName: String,
    firmwareName: String? = nil,
    hardwareType: UInt8 = 0
  ) {
    self.major = major
    self.minor = minor
    self.hardwareName = hardwareName
    self.firmwareName = firmwareName
    self.hardwareType = hardwareType
  }
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
  public let fault: VescFault
}

public struct VescFault: Equatable, Sendable {
  public enum Kind: String, Equatable, Sendable {
    case none
    case overVoltage = "over_voltage"
    case unknown
  }

  public let code: UInt8
  public let kind: Kind

  public init(code: UInt8) {
    self.code = code
    self.kind = switch code {
    case 0: .none
    case 1: .overVoltage
    default: .unknown
    }
  }
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
    var offset = min(3 + nameBytes.count + 1, payload.count)
    if payload.count - offset >= 12 { offset += 12 } // UUID
    let hardwareType = payload.count - offset >= 3 ? payload[offset + 2] : 0
    for _ in 0..<8 where offset < payload.count {
      offset += 1 // paired through NRF flags
    }
    let firmwareName: String?
    if offset < payload.count {
      let bytes = payload[offset...].prefix { $0 != 0 }
      let decoded = String(decoding: bytes, as: UTF8.self)
      firmwareName = decoded.isEmpty ? nil : decoded
    } else {
      firmwareName = nil
    }
    return FirmwareInfo(
      major: payload[1],
      minor: payload[2],
      hardwareName: String(decoding: nameBytes, as: UTF8.self),
      firmwareName: firmwareName,
      hardwareType: hardwareType
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
      faultCode: fault,
      fault: VescFault(code: fault)
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

public final class VescProtocolService: VescProtocolPort, @unchecked Sendable {
  private let lock = NSLock()
  private var stream = VescFrameStreamDecoder()
  private var pendingFirmwareCanID: UInt8?
  private var realtimeFields: [String] = []
  private var realtimeRuntimeFields: [String] = []

  public init() {}

  public func reset() {
    lock.lock()
    stream = VescFrameStreamDecoder()
    pendingFirmwareCanID = nil
    realtimeFields.removeAll()
    realtimeRuntimeFields.removeAll()
    lock.unlock()
  }

  public func firmwareRequest(canID: UInt8?) throws -> Data {
    lock.lock()
    pendingFirmwareCanID = canID
    lock.unlock()
    let payload: [UInt8] = canID.map { [34, $0, 0] } ?? [0]
    return Data(try VescFrameCodec.encode(payload))
  }

  public func canPingRequest() throws -> Data {
    Data(try VescFrameCodec.encode([62]))
  }

  public func refloatInfoRequest(canID: UInt8?) throws -> Data {
    let command: [UInt8] = [36, 101, 0, 2, 0]
    let payload = canID.map { [34, $0] + command } ?? command
    return Data(try VescFrameCodec.encode(payload))
  }

  public func refloatTelemetryIDsRequest(canID: UInt8) throws -> Data {
    Data(try VescFrameCodec.encode([34, canID, 36, 101, 32]))
  }

  public func refloatTelemetryRequest(canID: UInt8) throws -> Data {
    Data(try VescFrameCodec.encode([34, canID, 36, 101, 31]))
  }

  public func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    lock.lock()
    defer { lock.unlock() }
    return try stream.append(bytes).compactMap { payload in
      switch payload.first {
      case 0:
        let info = try VescPayloadDecoder.firmware(payload)
        return .firmware(
          FirmwareRecord(
            major: info.major,
            minor: info.minor,
            hardwareName: info.hardwareName,
            firmwareName: info.firmwareName,
            hardwareType: info.hardwareType,
            canID: pendingFirmwareCanID
          )
        )
      case 62:
        return .canDevices(Array(Set(payload.dropFirst())).sorted())
      case 36:
        guard payload.count >= 4, payload[1] == 101 else { return nil }
        if payload[2] == 0 {
          return .refloatInfo(version: try Self.decodeRefloatVersion(payload))
        }
        if payload[2] == 32 {
          let (fields, runtimeFields) = try Self.decodeRealtimeFields(payload)
          realtimeFields = fields
          realtimeRuntimeFields = runtimeFields
          return .refloatTelemetryLayout(fields: fields, runtimeFields: runtimeFields)
        }
        if payload[2] == 31 {
          return .refloatTelemetry(
            try Self.decodeRealtimeData(
              payload,
              fields: realtimeFields,
              runtimeFields: realtimeRuntimeFields
            )
          )
        }
        if payload[2] == 10 {
          return .refloatTelemetry(try Self.decodeRefloatTelemetry(payload))
        }
        return nil
      default:
        return nil
      }
    }
  }

  private static func decodeRefloatVersion(_ payload: [UInt8]) throws -> String {
    let responseVersion = payload[3]
    if responseVersion > 10 {
      return "\(responseVersion / 10).\(responseVersion % 10)"
    }
    guard responseVersion == 2, payload.count >= 61 else {
      throw VescProtocolError.truncatedPayload
    }
    return "\(payload[25]).\(payload[26]).\(payload[27])"
  }

  private static func decodeRealtimeFields(
    _ payload: [UInt8]
  ) throws -> (fields: [String], runtimeFields: [String]) {
    var index = 3
    func readList() throws -> [String] {
      guard index < payload.count else { throw VescProtocolError.truncatedPayload }
      let count = Int(payload[index])
      index += 1
      return try (0..<count).map { _ in
        guard index < payload.count else { throw VescProtocolError.truncatedPayload }
        let length = Int(payload[index])
        index += 1
        guard index + length <= payload.count else { throw VescProtocolError.truncatedPayload }
        let value = String(decoding: payload[index..<(index + length)], as: UTF8.self)
        index += length
        return value
      }
    }
    return (try readList(), try readList())
  }

  private static func decodeRealtimeData(
    _ payload: [UInt8],
    fields: [String],
    runtimeFields: [String]
  ) throws -> RefloatTelemetryRecord {
    guard !fields.isEmpty else { throw VescProtocolError.invalidFrame }
    let mask = payload[3]
    let includedRuntimeFields = mask & 0x1 != 0 ? runtimeFields : []
    guard payload.count >= 13 + (fields.count + includedRuntimeFields.count) * 2 else {
      throw VescProtocolError.truncatedPayload
    }
    var index = 13
    var values: [String: Double] = [:]
    for field in fields + includedRuntimeFields {
      values[field] = decodeFloat16(payload[index], payload[index + 1])
      index += 2
    }
    let state = payload[9] & 0x03
    return RefloatTelemetryRecord(
      rideState: state == 3 ? .riding : .idle,
      boardState: state,
      footpadState: payload[10] >> 6 & 0x03,
      packVoltageV: values["motor.batt_voltage"] ?? 0,
      dutyCycle: values["motor.duty_cycle"] ?? 0,
      controllerTempC: values["motor.mosfet_temp"],
      motorTempC: values["motor.motor_temp"],
      motorCurrentA: values["motor.current"] ?? 0,
      batteryCurrentA: values["motor.batt_current"] ?? 0,
      erpm: Int(values["motor.erpm"] ?? 0),
      speedKph: (values["motor.speed"] ?? 0) * 3.6
    )
  }

  private static func decodeFloat16(_ high: UInt8, _ low: UInt8) -> Double {
    let bits = UInt16(high) << 8 | UInt16(low)
    let sign = bits & 0x8000 == 0 ? 1.0 : -1.0
    let exponent = Int(bits >> 10 & 0x1f)
    let fraction = Int(bits & 0x03ff)
    if exponent == 0 {
      return sign * Double(fraction) * pow(2, -24)
    }
    return sign * (1 + Double(fraction) / 1024) * pow(2, Double(exponent - 15))
  }

  private static func decodeRefloatTelemetry(
    _ payload: [UInt8]
  ) throws -> RefloatTelemetryRecord {
    let detailLevel = payload[3]
    guard (1...3).contains(detailLevel) else { throw VescProtocolError.invalidFrame }
    let required = detailLevel >= 3 ? 55 : (detailLevel >= 2 ? 42 : 35)
    guard payload.count >= required else { throw VescProtocolError.truncatedPayload }
    let signed16: (Int) -> Int = { index in
      Int(Int16(bitPattern: UInt16(payload[index]) << 8 | UInt16(payload[index + 1])))
    }
    let state = payload[10] & 0x0f
    let switchBits = payload[11] & 0x0f
    let adc1 = Double(payload[12]) / 50
    let adc2 = Double(payload[13]) / 50
    let footpad: UInt8 = switch switchBits {
    case 2: 3
    case 1: adc1 > adc2 ? 1 : 2
    default: 0
    }
    return RefloatTelemetryRecord(
      rideState: (1...5).contains(state) ? .riding : .idle,
      boardState: state,
      footpadState: footpad,
      packVoltageV: Double(signed16(23)) / 10,
      dutyCycle: Double(Int(payload[33]) - 128) / 100,
      controllerTempC: detailLevel >= 2 ? Double(payload[39]) / 2 : nil,
      motorTempC: detailLevel >= 2 ? Double(payload[40]) / 2 : nil,
      motorCurrentA: Double(signed16(29)) / 10,
      batteryCurrentA: Double(signed16(31)) / 10,
      erpm: signed16(25),
      speedKph: Double(signed16(27)) / 10 * 3.6
    )
  }
}
