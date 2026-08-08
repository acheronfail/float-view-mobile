import Foundation

public struct DeviceID: Hashable, Sendable, Codable {
  public let value: String

  public init(_ value: String) {
    self.value = value
  }
}

public enum ConnectionState: String, Sendable, Codable {
  case disconnected
  case ready
  case failed
}

public enum BluetoothRadioState: String, Sendable, Codable {
  case unknown
  case unsupported
  case poweredOff = "powered_off"
  case poweredOn = "powered_on"
  case resetting
}

public enum BluetoothPermissionState: String, Sendable, Codable {
  case unknown
  case notRequired = "not_required"
  case notDetermined = "not_determined"
  case denied
  case restricted
  case granted
}

public struct BluetoothState: Sendable, Codable, Equatable {
  public let radio: BluetoothRadioState
  public let scanPermission: BluetoothPermissionState
  public let connectPermission: BluetoothPermissionState

  public init(
    radio: BluetoothRadioState,
    scanPermission: BluetoothPermissionState,
    connectPermission: BluetoothPermissionState
  ) {
    self.radio = radio
    self.scanPermission = scanPermission
    self.connectPermission = connectPermission
  }
}

public enum BluetoothEvent: Sendable {
  case stateChanged(BluetoothState)
  case deviceFound(DeviceID, name: String?, rssi: Int?)
  case scanFailed(String)
  case connectionChanged(DeviceID, ConnectionState)
  case notification(DeviceID, Data)
}

public protocol BluetoothPort: AnyObject, Sendable {
  func currentState() -> BluetoothState
  func requestPermission()
  func startDiscovery()
  func stopDiscovery()
  func connect(_ deviceID: DeviceID)
  func write(_ bytes: Data, to deviceID: DeviceID)
  func disconnect(_ deviceID: DeviceID)
}

public final class InactiveBluetoothPort: BluetoothPort, @unchecked Sendable {
  public init() {}

  public func currentState() -> BluetoothState {
    BluetoothState(radio: .unknown, scanPermission: .unknown, connectPermission: .unknown)
  }

  public func requestPermission() {}
  public func startDiscovery() {}
  public func stopDiscovery() {}
  public func connect(_: DeviceID) {}
  public func write(_: Data, to _: DeviceID) {}
  public func disconnect(_: DeviceID) {}
}
