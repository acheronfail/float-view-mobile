@testable import FloatViewBackend
import Foundation
import Testing

private final class ServerRelay: @unchecked Sendable {
  private let lock = NSLock()
  private weak var service: SharedService?
  func bind(_ service: SharedService) {
    lock.withLock { self.service = service }
  }

  func receive(_ event: BluetoothEvent) {
    lock.withLock { service }?.receive(event)
  }
}

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
  @Test func canonicalHostAndListenerRecreation() throws {
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let temporary = FileManager.default.temporaryDirectory
      .appendingPathComponent("float-view-shared-\(UUID().uuidString)")
    defer { try? FileManager.default.removeItem(at: temporary) }
    let relay = ServerRelay(), peripheral = SoftwareVescPeripheral()
    let bluetooth = SoftwareBluetoothAdapter(
      eventHandler: { relay.receive($0) },
      responseHandler: { peripheral.response(to: $0) }
    )
    let service = try SharedService(root: temporary, bluetooth: bluetooth, location: UnavailableLocation())
    relay.bind(service)
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
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
    process.arguments = ["node", "--strip-types", root.appendingPathComponent("conformance/src/shared-server.ts").path]
    var environment = ProcessInfo.processInfo.environment
    environment["FLOAT_VIEW_SHARED_WS_URL"] = "ws://127.0.0.1:\(server.port)/api/ws?token=\(server.token)"
    process.environment = environment
    try process.run()
    process.waitUntilExit()
    #expect(process.terminationStatus == 0)
    let token = server.token
    let stopped = Date()
    server.stop()
    #expect(Date().timeIntervalSince(stopped) < 2)
    try server.start()
    #expect(server.token == token)
  }
}
