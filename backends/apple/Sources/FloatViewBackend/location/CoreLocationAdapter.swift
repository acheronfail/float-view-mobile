import CoreLocation
import Foundation
#if os(iOS)
  import UIKit
#elseif os(macOS)
  import AppKit
#endif

/// Native location ownership for both Apple hosts. The WebView receives only
/// the mapped status; observations are forwarded directly to the shared service.
public final class CoreLocationAdapter: NSObject, LocationProviderPort, CLLocationManagerDelegate, @unchecked Sendable {
  private let manager = CLLocationManager()
  private let lock = NSLock()
  private let observationQueue = DispatchQueue(label: "fail.acheron.float-view.location")
  private var handler: (@Sendable (LocationProviderObservation) -> Void)?
  private var lastReason: String?
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
      (servicesEnabled: servicesEnabled, authorization: authorization, reason: lastReason)
    }
    return Self.providerState(
      servicesEnabled: cached.servicesEnabled,
      authorization: cached.authorization,
      reason: cached.reason
    )
  }

  static func providerState(
    servicesEnabled: Bool,
    authorization: CLAuthorizationStatus,
    reason: String? = nil
  ) -> LocationProviderState {
    guard servicesEnabled else {
      return LocationProviderState(
        authorization: .unavailable,
        backgroundCapture: .unavailable,
        reason: "Location services are disabled."
      )
    }
    switch authorization {
    case .notDetermined:
      return LocationProviderState(
        authorization: .notDetermined,
        backgroundCapture: .unavailable,
        reason: "Location permission has not been resolved."
      )
    case .denied:
      return LocationProviderState(
        authorization: .denied,
        backgroundCapture: .unavailable,
        reason: reason ?? "Location permission was denied."
      )
    case .restricted:
      return LocationProviderState(
        authorization: .restricted,
        backgroundCapture: .unavailable,
        reason: reason ?? "Location permission is restricted."
      )
    case .authorizedAlways, .authorizedWhenInUse:
      // Rides start while the app is in use. With background location updates
      // enabled and automatic pauses disabled, When In Use authorization also
      // supports continuing that capture after the app enters the background.
      return LocationProviderState(authorization: .authorized, backgroundCapture: .available)
    @unknown default:
      return LocationProviderState(
        authorization: .unavailable,
        backgroundCapture: .unavailable,
        reason: "The Apple location provider returned an unknown status."
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
        manager.requestAlwaysAuthorization()
      #endif
    }
  }

  public func openSettings() {
    DispatchQueue.main.async {
      #if os(iOS)
        guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
        UIApplication.shared.open(url)
      #elseif os(macOS)
        NSWorkspace.shared
          .open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices")!)
      #endif
    }
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

  public func locationManager(_: CLLocationManager, didFailWithError _: Error) {
    lock.withLock { lastReason = "Location provider unavailable." }
  }

  public func locationManager(_: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
    guard let location = locations.last,
          location.coordinate.latitude.isFinite,
          location.coordinate.longitude.isFinite
    else { return }
    let observation = LocationProviderObservation(
      observedAt: location.timestamp,
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
      if authorization == .denied {
        lastReason = "Location permission was denied."
      }
    }
  }

  private func performOnMain(_ action: @escaping @Sendable () -> Void) {
    if Thread.isMainThread {
      action()
    } else {
      DispatchQueue.main.async(execute: action)
    }
  }
}
