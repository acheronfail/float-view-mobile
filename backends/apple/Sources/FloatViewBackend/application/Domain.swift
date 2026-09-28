import Foundation

public struct DeviceID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) {
    self.value = value
  }
}

public enum ConnectionState: String, Sendable, Codable {
  case disconnected
  case connecting
  case connected
  case discoveringServices
  case enablingNotifications
  case identifying
  case selectingController = "selecting_controller"
  case ready
  case reconnecting
  case disconnecting
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

public enum LocationProviderAuthorization: String, Sendable, Codable {
  case notDetermined
  case foregroundOnly
  case authorized
  case denied
  case restricted
  case unavailable
}

public enum LocationProviderBackgroundCapture: String, Sendable, Codable {
  case available
  case foregroundOnly
  case unavailable
}

public struct LocationProviderState: Sendable, Codable, Equatable {
  public let authorization: LocationProviderAuthorization
  public let backgroundCapture: LocationProviderBackgroundCapture
  public let reason: String?

  public init(
    authorization: LocationProviderAuthorization,
    backgroundCapture: LocationProviderBackgroundCapture,
    reason: String? = nil
  ) {
    self.authorization = authorization
    self.backgroundCapture = backgroundCapture
    self.reason = reason
  }
}

public struct LocationProviderObservation: Sendable, Codable, Equatable {
  public let observedAt: Date
  public let latitude: Double
  public let longitude: Double
  public let accuracyM: Double?

  public init(observedAt: Date, latitude: Double, longitude: Double, accuracyM: Double?) {
    self.observedAt = observedAt
    self.latitude = latitude
    self.longitude = longitude
    self.accuracyM = accuracyM
  }
}

public protocol LocationProviderPort: AnyObject, Sendable {
  func state() -> LocationProviderState
  func requestPermission()
  func openSettings()
  func startUpdates(_ handler: @escaping @Sendable (LocationProviderObservation) -> Void)
  func stopUpdates()
}

public enum ThemePreference: String, Sendable, Codable, Equatable, CaseIterable {
  case system
  case light
  case dark
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
  func configureSavedDeviceIDs(_ deviceIDs: Set<DeviceID>)
}

public extension BluetoothPort {
  func configureSavedDeviceIDs(_: Set<DeviceID>) {}
}
