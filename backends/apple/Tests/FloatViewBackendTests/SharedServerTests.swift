@testable import FloatViewBackend
import Foundation
import Testing

private final class UnavailableLocation: LocationProviderPort, @unchecked Sendable {
  func state() -> LocationProviderState {
    .init(authorization: .unavailable, backgroundCapture: .unavailable)
  }

  func requestPermission() {}
  func openSettings() {}
  func startUpdates(_: @escaping @Sendable (LocationProviderObservation) -> Void) {}
  func stopUpdates() {}
}

@Suite(.serialized)
struct SharedServerTests {
  @Test func listenerRecreation() throws {
    let temporary = FileManager.default.temporaryDirectory
      .appendingPathComponent("float-view-shared-\(UUID().uuidString)")
    defer { try? FileManager.default.removeItem(at: temporary) }
    let bluetooth = IdleBluetooth()
    let service = try SharedService(root: temporary, bluetooth: bluetooth, location: UnavailableLocation())
    try service.start(presenter: { value in
      // Native UI cancellation/completion may race; only the first wins.
      value.finish()
      value.finish()
      return true
    })
    defer { service.shutdown() }
    let server = try RustLoopbackHost(html: Data("<html>candidate</html>".utf8), service: service, platform: "macos")
    try server.start()
    defer { server.stop() }
    let token = server.token
    let stopped = Date()
    server.stop()
    #expect(Date().timeIntervalSince(stopped) < 2)
    try server.start()
    #expect(server.token == token)
  }
}

private final class IdleBluetooth: BluetoothPort, @unchecked Sendable {
  func currentState() -> BluetoothState {
    .init(
      radio: .poweredOff,
      scanPermission: .granted,
      connectPermission: .granted
    )
  }

  func requestPermission() {}
  func startDiscovery() {}
  func stopDiscovery() {}
  func connect(_: DeviceID, generation _: BluetoothConnection) {}
  func enableNotifications(generation _: BluetoothConnection, operation _: UInt64) {}
  func write(_: Data, to _: DeviceID, generation _: BluetoothConnection, operation _: UInt64, response _: Bool) {}
  func disconnect(_: DeviceID) {}
}
