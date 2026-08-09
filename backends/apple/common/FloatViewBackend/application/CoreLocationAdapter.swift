import CoreLocation
import Foundation
#if os(iOS)
  import UIKit
#elseif os(macOS)
  import AppKit
#endif

/// Native location ownership for both Apple hosts. The WebView receives only
/// the mapped status; observations are forwarded directly to the Rust bridge.
public final class CoreLocationAdapter: NSObject, LocationProviderPort, CLLocationManagerDelegate, @unchecked Sendable {
  private let manager = CLLocationManager()
  private let lock = NSLock()
  private let observationQueue = DispatchQueue(label: "fail.acheron.float-view.location")
  private var handler: (@Sendable (LocationProviderObservation) -> Void)?
  private var stateHandler: (@Sendable (LocationProviderState) -> Void)?
  private var servicesEnabled = false
  private var authorization: CLAuthorizationStatus = .notDetermined

  override public init() {
    super.init()
    manager.delegate = self
    manager.desiredAccuracy = kCLLocationAccuracyNearestTenMeters
    manager.distanceFilter = 5
    manager.pausesLocationUpdatesAutomatically = false
    #if os(iOS)
      manager.allowsBackgroundLocationUpdates = true
      manager.showsBackgroundLocationIndicator = false
    #endif
    performOnMain { [weak self] in self?.refreshState() }
  }

  public func state() -> LocationProviderState {
    let cached = lock.withLock {
      (servicesEnabled: servicesEnabled, authorization: authorization)
    }
    return Self.mappedState(
      servicesEnabled: cached.servicesEnabled,
      authorization: cached.authorization
    )
  }

  static func mappedState(
    servicesEnabled: Bool,
    authorization: CLAuthorizationStatus
  ) -> LocationProviderState {
    guard servicesEnabled else {
      return LocationProviderState(
        authorization: .unavailable,
        backgroundCapture: .unavailable
      )
    }
    switch authorization {
    case .notDetermined:
      return LocationProviderState(
        authorization: .notDetermined,
        backgroundCapture: .unavailable
      )
    case .denied:
      return LocationProviderState(
        authorization: .denied,
        backgroundCapture: .unavailable
      )
    case .restricted:
      return LocationProviderState(
        authorization: .restricted,
        backgroundCapture: .unavailable
      )
    case .authorizedAlways:
      return LocationProviderState(authorization: .authorized, backgroundCapture: .available)
    case .authorizedWhenInUse:
      #if os(iOS)
        return LocationProviderState(
          authorization: .foregroundOnly,
          backgroundCapture: .foregroundOnly
        )
      #else
        return LocationProviderState(authorization: .authorized, backgroundCapture: .available)
      #endif
    @unknown default:
      return LocationProviderState(
        authorization: .unavailable,
        backgroundCapture: .unavailable
      )
    }
  }

  public func requestPermission() {
    performOnMain { [weak self] in
      guard let self else { return }
      refreshState()
      #if os(iOS)
        manager.requestWhenInUseAuthorization()
      #else
        // macOS has no mobile background suspension mechanism here. Normal
        // process-lifetime updates need only the narrower when-in-use grant.
        manager.requestWhenInUseAuthorization()
      #endif
    }
  }

  public func setStateHandler(
    _ handler: @escaping @Sendable (LocationProviderState) -> Void
  ) {
    lock.withLock { stateHandler = handler }
    observationQueue.async { [weak self] in
      guard let self else { return }
      handler(state())
    }
  }

  public func openSettings() {
    #if os(iOS)
      Task { @MainActor in
        guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
        UIApplication.shared.open(url)
      }
    #elseif os(macOS)
      performOnMain {
        NSWorkspace.shared
          .open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices")!)
      }
    #endif
  }

  public func startUpdates(_ handler: @escaping @Sendable (LocationProviderObservation) -> Void) {
    lock.withLock { self.handler = handler }
    performOnMain { [weak self] in
      self?.refreshState()
      self?.manager.startUpdatingLocation()
    }
  }

  public func stopUpdates() {
    lock.withLock { handler = nil }
    performOnMain { [weak self] in self?.manager.stopUpdatingLocation() }
  }

  public func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
    refreshState(manager)
  }

  public func locationManager(_: CLLocationManager, didFailWithError _: Error) {}

  public func locationManager(_: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
    guard let location = locations.last,
          location.coordinate.latitude.isFinite,
          location.coordinate.longitude.isFinite
    else { return }
    let observation = LocationProviderObservation(
      latitude: location.coordinate.latitude,
      longitude: location.coordinate.longitude,
      accuracyM: location.horizontalAccuracy >= 0 ? location.horizontalAccuracy : nil
    )
    guard let handler = lock.withLock({ handler }) else { return }
    observationQueue.async { handler(observation) }
  }

  /// CLLocationManager is main-thread-owned. Backend snapshots read only this
  /// cached value so they cannot participate in a Core Location/application
  /// service lock inversion while telemetry and location callbacks are active.
  private func refreshState(_ manager: CLLocationManager? = nil) {
    let manager = manager ?? self.manager
    let servicesEnabled = CLLocationManager.locationServicesEnabled()
    let authorization = manager.authorizationStatus
    lock.withLock {
      self.servicesEnabled = servicesEnabled
      self.authorization = authorization
    }
    submitState()
  }

  private func submitState() {
    guard let handler = lock.withLock({ stateHandler }) else { return }
    let value = state()
    observationQueue.async { handler(value) }
  }

  private func performOnMain(_ action: @escaping @Sendable () -> Void) {
    if Thread.isMainThread { action() }
    else { DispatchQueue.main.async(execute: action) }
  }
}
