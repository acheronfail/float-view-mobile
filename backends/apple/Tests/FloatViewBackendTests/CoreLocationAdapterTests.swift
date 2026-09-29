import CoreLocation
@testable import FloatViewBackend
import Foundation
import Testing

@Test
func coreLocationRidePermissions() throws {
  // Core Location's When In Use case (4) is unavailable by name on macOS;
  // exercise its mapping here as well as on iOS.
  let whenInUse = try #require(CLAuthorizationStatus(rawValue: 4))
  for authorization in [whenInUse, .authorizedAlways] {
    #expect(CoreLocationAdapter.providerState(servicesEnabled: true, authorization: authorization)
      == LocationProviderState(authorization: .authorized, backgroundCapture: .available))
    #expect(CoreLocationAdapter.providerState(servicesEnabled: false, authorization: authorization)
      .authorization == .unavailable)
  }
  for (authorization, expected) in [
    (CLAuthorizationStatus.notDetermined, LocationProviderAuthorization.notDetermined),
    (.denied, .denied),
    (.restricted, .restricted),
  ] {
    let state = CoreLocationAdapter.providerState(servicesEnabled: true, authorization: authorization)
    #expect(state.authorization == expected)
    #expect(state.backgroundCapture == .unavailable)
  }
}

@Test
func coreLocationObservationDeliveryLeavesTheDelegateThread() {
  let adapter = CoreLocationAdapter()
  let delivery = LocationDeliveryCapture()
  let delegateThread = pthread_mach_thread_np(pthread_self())
  adapter.startUpdates { _ in delivery.recordCurrentThread() }

  adapter.locationManager(
    CLLocationManager(),
    didUpdateLocations: [CLLocation(
      coordinate: CLLocationCoordinate2D(latitude: 1, longitude: 2),
      altitude: 0,
      horizontalAccuracy: 5,
      verticalAccuracy: 5,
      timestamp: Date()
    )]
  )

  #expect(delivery.wait() == .success)
  #expect(delivery.threadID != delegateThread)
  adapter.stopUpdates()
}

private final class LocationDeliveryCapture: @unchecked Sendable {
  private let lock = NSLock()
  private let semaphore = DispatchSemaphore(value: 0)
  private var value: UInt32?

  var threadID: UInt32? {
    lock.withLock { value }
  }

  func recordCurrentThread() {
    lock.withLock { value = pthread_mach_thread_np(pthread_self()) }
    semaphore.signal()
  }

  func wait() -> DispatchTimeoutResult {
    semaphore.wait(timeout: .now() + 2)
  }
}
