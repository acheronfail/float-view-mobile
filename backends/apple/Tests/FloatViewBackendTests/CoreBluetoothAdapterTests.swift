import CoreBluetooth
@testable import FloatViewBackend
import Foundation
import Testing

@Test func observingAndStoppingBluetoothDoesNotCreatePermissionPrompt() {
  let queue = DispatchQueue(label: "float-view.bluetooth-permission-test")
  let adapter = CoreBluetoothAdapter(queue: queue, eventHandler: { _ in }, authorization: { .notDetermined })
  #expect(adapter.currentState().scanPermission == .notDetermined)
  #expect(adapter.currentState().radio == .unknown)
  adapter.stopDiscovery()
  // Flush the same queue used by shutdown; merely stopping must not create a manager.
  queue.sync {
    #expect(adapter.central == nil)
    #expect(adapter.currentState().scanPermission == .notDetermined)
  }
}
