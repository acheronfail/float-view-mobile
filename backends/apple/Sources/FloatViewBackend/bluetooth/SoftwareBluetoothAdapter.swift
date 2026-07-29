import Foundation

public final class SoftwareBluetoothAdapter: BluetoothPort, @unchecked Sendable {
  public static let deviceID = DeviceID("software-vesc")
  private let event: @Sendable (BluetoothEvent) -> Void
  private let response: @Sendable (Data) -> Data?

  public init(
    eventHandler: @escaping @Sendable (BluetoothEvent) -> Void,
    responseHandler: @escaping @Sendable (Data) -> Data? = { _ in nil }
  ) {
    event = eventHandler
    response = responseHandler
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

  public func write(_ bytes: Data, to deviceID: DeviceID) {
    guard let bytes = response(bytes) else { return }
    event(.notification(deviceID, bytes))
  }

  public func disconnect(_ deviceID: DeviceID) {
    event(.connectionChanged(deviceID, .disconnected))
  }
}
