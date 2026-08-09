import CoreBluetooth
@testable import FloatViewBackend
import Foundation
import Testing

@Test
func coreBluetoothOsStatesMapWithoutApplicationPolicy() {
  #expect(CoreBluetoothAdapter.radioState(.unknown) == .unknown)
  #expect(CoreBluetoothAdapter.radioState(.resetting) == .resetting)
  #expect(CoreBluetoothAdapter.radioState(.unsupported) == .unsupported)
  #expect(CoreBluetoothAdapter.radioState(.unauthorized) == .unknown)
  #expect(CoreBluetoothAdapter.radioState(.poweredOff) == .poweredOff)
  #expect(CoreBluetoothAdapter.radioState(.poweredOn) == .poweredOn)

  #expect(CoreBluetoothAdapter.permissionState(.notDetermined) == .notDetermined)
  #expect(CoreBluetoothAdapter.permissionState(.restricted) == .restricted)
  #expect(CoreBluetoothAdapter.permissionState(.denied) == .denied)
  #expect(CoreBluetoothAdapter.permissionState(.allowedAlways) == .granted)
}

@Test
func bluetoothWritesUseDefaultAndEnlargedPayloadsWithinQueueBounds() {
  let bytes = Data((0 ..< 50).map(UInt8.init))
  #expect(VescBluetoothPolicy.chunks(bytes, maximumPayloadBytes: 20).map(\.count) == [20, 20, 10])
  #expect(VescBluetoothPolicy.chunks(bytes, maximumPayloadBytes: 244).map(\.count) == [50])
  #expect(VescBluetoothPolicy.acceptsQueuedWrite(
    pendingWrites: 127,
    pendingBytes: 65_535,
    newWrites: 1,
    newBytes: 1
  ))
  #expect(!VescBluetoothPolicy.acceptsQueuedWrite(
    pendingWrites: 128,
    pendingBytes: 0,
    newWrites: 1,
    newBytes: 1
  ))
  #expect(!VescBluetoothPolicy.acceptsQueuedWrite(
    pendingWrites: 0,
    pendingBytes: 65_536,
    newWrites: 1,
    newBytes: 1
  ))
  #expect(!VescBluetoothPolicy.acceptsQueuedWrite(
    pendingWrites: Int.max,
    pendingBytes: 0,
    newWrites: Int.max,
    newBytes: 1
  ))
  #expect(VescBluetoothPolicy.chunks(bytes, maximumPayloadBytes: 20).reduce(Data(), +) == bytes)
}

@Test
func bluetoothSetupValidatesElementsAndWriteResponseModes() {
  #expect(VescBluetoothPolicy.writeMode(
    supportsWriteWithResponse: true,
    supportsWriteWithoutResponse: true
  ) == .withResponse)
  #expect(VescBluetoothPolicy.writeMode(
    supportsWriteWithResponse: false,
    supportsWriteWithoutResponse: true
  ) == .withoutResponse)
  #expect(VescBluetoothPolicy.writeMode(
    supportsWriteWithResponse: false,
    supportsWriteWithoutResponse: false
  ) == .unsupported)

  #expect(VescBluetoothPolicy.setupFailure(
    hasService: true,
    hasWriteCharacteristic: true,
    writeMode: .withoutResponse,
    hasNotifyCharacteristic: true,
    hasNotifyProperty: true
  ) == nil)
  #expect(VescBluetoothPolicy.setupFailure(
    hasService: false,
    hasWriteCharacteristic: false,
    writeMode: .unsupported,
    hasNotifyCharacteristic: false,
    hasNotifyProperty: false
  ) == "vesc_service_unavailable")
  #expect(VescBluetoothPolicy.setupFailure(
    hasService: true,
    hasWriteCharacteristic: true,
    writeMode: .unsupported,
    hasNotifyCharacteristic: true,
    hasNotifyProperty: true
  ) == "vesc_characteristics_unavailable")
  #expect(VescBluetoothPolicy.setupFailure(
    hasService: true,
    hasWriteCharacteristic: true,
    writeMode: .withResponse,
    hasNotifyCharacteristic: true,
    hasNotifyProperty: false
  ) == "vesc_characteristics_unavailable")
}

@Test
func bluetoothCancellationAndNewGenerationRejectStaleCallbacks() {
  #expect(VescBluetoothPolicy.acceptsCallback(
    callbackGeneration: 7,
    activeGeneration: 7,
    isCancelled: false
  ))
  #expect(!VescBluetoothPolicy.acceptsCallback(
    callbackGeneration: 7,
    activeGeneration: 8,
    isCancelled: false
  ))
  #expect(!VescBluetoothPolicy.acceptsCallback(
    callbackGeneration: 7,
    activeGeneration: 7,
    isCancelled: true
  ))
  #expect(!VescBluetoothPolicy.acceptsCallback(
    callbackGeneration: 7,
    activeGeneration: nil,
    isCancelled: false
  ))
}
