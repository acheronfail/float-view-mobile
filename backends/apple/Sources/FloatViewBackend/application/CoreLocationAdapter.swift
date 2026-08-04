import CoreLocation
import Foundation
#if os(iOS)
  import UIKit
#elseif os(macOS)
  import AppKit
#endif

/// Native location ownership for both Apple hosts. The WebView receives only
/// the mapped status; observations are forwarded directly to ApplicationService.
public final class CoreLocationAdapter: NSObject, LocationProviderPort, CLLocationManagerDelegate, @unchecked Sendable {
  private let manager = CLLocationManager()
  private let lock = NSLock()
  private var handler: (@Sendable (LocationProviderObservation) -> Void)?
  private var lastReason: String?

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
  }

  public func state() -> LocationProviderState {
    guard CLLocationManager.locationServicesEnabled() else {
      return LocationProviderState(
        authorization: .unavailable,
        backgroundCapture: .unavailable,
        reason: "Location services are disabled."
      )
    }
    let authorization = manager.authorizationStatus
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
        reason: lastReason ?? "Location permission was denied."
      )
    case .restricted:
      return LocationProviderState(
        authorization: .restricted,
        backgroundCapture: .unavailable,
        reason: lastReason ?? "Location permission is restricted."
      )
    case .authorizedAlways:
      return LocationProviderState(authorization: .authorized, backgroundCapture: .available)
    case .authorizedWhenInUse:
      #if os(iOS)
        return LocationProviderState(
          authorization: .foregroundOnly,
          backgroundCapture: .foregroundOnly,
          reason: "Background location access is not enabled."
        )
      #else
        return LocationProviderState(authorization: .authorized, backgroundCapture: .available)
      #endif
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
      #if os(iOS)
        manager.requestWhenInUseAuthorization()
      #else
        manager.requestAlwaysAuthorization()
      #endif
    }
  }

  public func openSettings() {
    performOnMain {
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
    performOnMain { [weak self] in self?.manager.startUpdatingLocation() }
  }

  public func stopUpdates() {
    lock.withLock { handler = nil }
    performOnMain { [weak self] in self?.manager.stopUpdatingLocation() }
  }

  public func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
    if manager.authorizationStatus == .denied {
      lock.withLock { lastReason = "Location permission was denied." }
    }
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
    lock.withLock { handler }?(observation)
  }

  private func performOnMain(_ action: @escaping @Sendable () -> Void) {
    if Thread.isMainThread { action() }
    else { DispatchQueue.main.async(execute: action) }
  }
}
