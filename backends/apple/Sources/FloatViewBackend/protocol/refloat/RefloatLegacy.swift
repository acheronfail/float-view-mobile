import Foundation

enum RefloatLegacyTelemetryDecoder {
  static func decode(_ payload: [UInt8]) throws -> RefloatTelemetryRecord {
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
