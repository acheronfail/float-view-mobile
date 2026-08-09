import CoreLocation
@testable import FloatViewBackend
import Foundation
import Testing

@Test
func coreLocationOsAuthorizationMapsDirectlyWithoutWaitingForCoordinates() {
  #expect(CoreLocationAdapter.mappedState(
    servicesEnabled: false,
    authorization: .authorizedAlways
  ) == LocationProviderState(
    authorization: .unavailable,
    backgroundCapture: .unavailable
  ))
  #expect(CoreLocationAdapter.mappedState(
    servicesEnabled: true,
    authorization: .notDetermined
  ) == LocationProviderState(
    authorization: .notDetermined,
    backgroundCapture: .unavailable
  ))
  #expect(CoreLocationAdapter.mappedState(
    servicesEnabled: true,
    authorization: .denied
  ) == LocationProviderState(
    authorization: .denied,
    backgroundCapture: .unavailable
  ))
  #expect(CoreLocationAdapter.mappedState(
    servicesEnabled: true,
    authorization: .restricted
  ) == LocationProviderState(
    authorization: .restricted,
    backgroundCapture: .unavailable
  ))
  #expect(CoreLocationAdapter.mappedState(
    servicesEnabled: true,
    authorization: .authorizedAlways
  ) == LocationProviderState(
    authorization: .authorized,
    backgroundCapture: .available
  ))
  #if os(iOS)
    #expect(CoreLocationAdapter.mappedState(
      servicesEnabled: true,
      authorization: .authorizedWhenInUse
    ) == LocationProviderState(
      authorization: .foregroundOnly,
      backgroundCapture: .foregroundOnly
    ))
  #endif
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

@Test
func coreLocationAuthorizationChangesReachTheFocusedStateSinkWithoutCoordinates() {
  let adapter = CoreLocationAdapter()
  let delivery = LocationStateDeliveryCapture()
  adapter.setStateHandler { state in delivery.record(state) }

  #expect(delivery.wait() == .success)
  adapter.locationManagerDidChangeAuthorization(CLLocationManager())
  #expect(delivery.wait() == .success)
  #expect(delivery.values.count >= 2)
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

private final class LocationStateDeliveryCapture: @unchecked Sendable {
  private let lock = NSLock()
  private let semaphore = DispatchSemaphore(value: 0)
  private var stored: [LocationProviderState] = []

  var values: [LocationProviderState] {
    lock.withLock { stored }
  }

  func record(_ value: LocationProviderState) {
    lock.withLock { stored.append(value) }
    semaphore.signal()
  }

  func wait() -> DispatchTimeoutResult {
    semaphore.wait(timeout: .now() + 2)
  }
}
