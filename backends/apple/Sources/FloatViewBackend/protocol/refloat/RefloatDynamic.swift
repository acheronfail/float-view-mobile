import Foundation

enum RefloatDynamicTelemetryDecoder {
  static func decodeFields(
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

  static func decodeData(
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
}
