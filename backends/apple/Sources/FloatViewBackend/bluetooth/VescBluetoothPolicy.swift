import Foundation

enum VescBluetoothPolicy {
  enum WriteMode: Equatable {
    case withResponse
    case withoutResponse
    case unsupported
  }

  static let maximumPendingWrites = 128
  static let maximumPendingBytes = 65_536

  static func chunks(_ bytes: Data, maximumPayloadBytes: Int) -> [Data] {
    let chunkSize = max(1, maximumPayloadBytes)
    return stride(from: 0, to: bytes.count, by: chunkSize).map {
      bytes.subdata(in: $0 ..< min($0 + chunkSize, bytes.count))
    }
  }

  static func writeMode(
    supportsWriteWithResponse: Bool,
    supportsWriteWithoutResponse: Bool
  ) -> WriteMode {
    if supportsWriteWithResponse {
      return .withResponse
    }
    if supportsWriteWithoutResponse {
      return .withoutResponse
    }
    return .unsupported
  }

  static func setupFailure(
    hasService: Bool,
    hasWriteCharacteristic: Bool,
    writeMode: WriteMode,
    hasNotifyCharacteristic: Bool,
    hasNotifyProperty: Bool
  ) -> String? {
    guard hasService else { return "vesc_service_unavailable" }
    guard hasWriteCharacteristic, writeMode != .unsupported,
          hasNotifyCharacteristic, hasNotifyProperty
    else {
      return "vesc_characteristics_unavailable"
    }
    return nil
  }

  static func acceptsCallback(
    callbackGeneration: UInt64,
    activeGeneration: UInt64?,
    isCancelled: Bool
  ) -> Bool {
    !isCancelled && activeGeneration == callbackGeneration
  }

  static func acceptsQueuedWrite(
    pendingWrites: Int,
    pendingBytes: Int,
    newWrites: Int,
    newBytes: Int
  ) -> Bool {
    guard pendingWrites >= 0, pendingBytes >= 0, newWrites >= 0, newBytes >= 0,
          pendingWrites <= maximumPendingWrites,
          pendingBytes <= maximumPendingBytes
    else {
      return false
    }
    return newWrites <= maximumPendingWrites - pendingWrites
      && newBytes <= maximumPendingBytes - pendingBytes
  }
}
