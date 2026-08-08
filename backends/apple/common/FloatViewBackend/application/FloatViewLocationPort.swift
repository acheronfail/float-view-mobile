public enum LocationProviderAuthorization: String, Sendable {
  case notDetermined
  case foregroundOnly
  case authorized
  case denied
  case restricted
  case unavailable
}

public enum LocationProviderBackgroundCapture: String, Sendable {
  case available
  case foregroundOnly
  case unavailable
}

public struct LocationProviderState: Sendable, Equatable {
  public let authorization: LocationProviderAuthorization
  public let backgroundCapture: LocationProviderBackgroundCapture

  public init(
    authorization: LocationProviderAuthorization,
    backgroundCapture: LocationProviderBackgroundCapture
  ) {
    self.authorization = authorization
    self.backgroundCapture = backgroundCapture
  }
}

public struct LocationProviderObservation: Sendable, Equatable {
  public let latitude: Double
  public let longitude: Double
  public let accuracyM: Double?

  public init(latitude: Double, longitude: Double, accuracyM: Double?) {
    self.latitude = latitude
    self.longitude = longitude
    self.accuracyM = accuracyM
  }
}

public protocol LocationProviderPort: AnyObject, Sendable {
  func state() -> LocationProviderState
  func setStateHandler(_ handler: @escaping @Sendable (LocationProviderState) -> Void)
  func requestPermission()
  func openSettings()
  func startUpdates(_ handler: @escaping @Sendable (LocationProviderObservation) -> Void)
  func stopUpdates()
}

public extension LocationProviderPort {
  func setStateHandler(_: @escaping @Sendable (LocationProviderState) -> Void) {}
}
