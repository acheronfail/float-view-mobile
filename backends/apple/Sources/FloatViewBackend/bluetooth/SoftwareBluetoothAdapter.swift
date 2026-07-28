import Foundation

public final class SoftwareBluetoothAdapter: BluetoothPort, @unchecked Sendable {
  public static let deviceID = DeviceID("software-vesc")
  private let event: @Sendable (BluetoothEvent) -> Void

  public init(eventHandler: @escaping @Sendable (BluetoothEvent) -> Void) {
    event = eventHandler
  }

  public func currentState() -> BluetoothState {
    BluetoothState(
      radio: .poweredOn,
      scanPermission: .granted,
      connectPermission: .granted
    )
  }

  public func requestPermission() {
    event(.stateChanged(currentState()))
  }

  public func startDiscovery() {
    event(.deviceFound(Self.deviceID, name: "Software VESC", rssi: -42))
  }

  public func stopDiscovery() {}

  public func connect(_ deviceID: DeviceID) {
    event(.connectionChanged(deviceID, .connecting))
    event(.connectionChanged(deviceID, .ready))
  }

  public func write(_: Data, to _: DeviceID) {}

  public func disconnect(_ deviceID: DeviceID) {
    event(.connectionChanged(deviceID, .disconnected))
  }
}
