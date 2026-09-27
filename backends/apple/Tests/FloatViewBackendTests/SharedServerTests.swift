@testable import FloatViewBackend
import Foundation
import Testing

private final class ServerRelay: @unchecked Sendable {
  var app: ApplicationService?
  func receive(_ event: BluetoothEvent) {
    try? app?.receive(event)
  }
}

private struct ServerClock: ClockPort {
  func now() -> Date {
    Date()
  }

  func monotonicMilliseconds() -> Int64 {
    Int64(ProcessInfo.processInfo.systemUptime * 1000)
  }
}

@Suite(.serialized)
struct SharedServerTests {
  @Test func canonicalCandidate() throws {
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let temporary = FileManager.default.temporaryDirectory
      .appendingPathComponent("float-view-shared-\(UUID().uuidString)")
    defer { try? FileManager.default.removeItem(at: temporary) }
    let store = try SQLiteLogStore(
      root: temporary,
      schemaDirectory: root.appendingPathComponent("backends/sql"),
      availableBytes: { Int64.max }
    )
    let seed = try store.create(deviceID: DeviceID("large-archive-fixture"), at: Date(timeIntervalSince1970: 0))
    for batch in 0 ..< 150 {
      let samples = (0 ..< 100).map { index in
        let index = batch * 100 + index
        return TelemetrySample(
          sequence: Int64(index + 1),
          observedAt: Date(timeIntervalSince1970: Double(index)),
          elapsedMs: Int64(index * 1000),
          rideState: .riding,
          packVoltageV: 60,
          dutyCycle: 0.2,
          motorCurrentA: 10,
          speedKph: 20
        )
      }
      try store.append(jobID: seed.jobID, samples: samples)
    }
    _ = try store.close(jobID: seed.jobID, at: Date(timeIntervalSince1970: 15000), reason: "test_seed")
    let events = EventHub(), relay = ServerRelay(), peripheral = SoftwareVescPeripheral()
    let bluetooth = SoftwareBluetoothAdapter(
      eventHandler: { relay.receive($0) },
      responseHandler: { peripheral.response(to: $0) }
    )
    let app = ApplicationService(
      bluetooth: bluetooth,
      clock: ServerClock(),
      logStore: store,
      productStore: store,
      sessionStore: store,
      scheduler: SystemScheduler(),
      events: events
    )
    relay.app = app
    let server = try RustLoopbackHost(
      html: Data("<html>candidate</html>".utf8),
      applicationService: app,
      events: events,
      platform: "macos"
    )
    try server.start()
    defer { server.stop(); try? app.disconnect(); relay.app = nil }
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
    process.arguments = ["node", "--strip-types", root.appendingPathComponent("conformance/src/shared-server.ts").path]
    var environment = ProcessInfo.processInfo.environment
    environment["FLOAT_VIEW_SHARED_WS_URL"] = "ws://127.0.0.1:\(server.port)/api/ws?token=\(server.token)"
    process.environment = environment
    try process.run()
    process.waitUntilExit()
    #expect(process.terminationStatus == 0)
    if process.terminationStatus != 0 {
      return
    }
    _ = try app.startDiscovery()
    try app.connect(SoftwareBluetoothAdapter.deviceID, trackingMode: .saved)
    let deadline = Date().addingTimeInterval(8)
    while (app.activeConnectionSnapshot()?.latestSample?.sequence ?? 0) < 2,
          Date() < deadline
    {
      Thread.sleep(forTimeInterval: 0.025)
    }
    let before = try #require(app.activeConnectionSnapshot()?.latestSample?.sequence)
    let token = server.token
    let stopped = Date()
    server.stop()
    #expect(Date().timeIntervalSince(stopped) < 2)
    Thread.sleep(forTimeInterval: 1.2)
    let after = try #require(app.activeConnectionSnapshot()?.latestSample?.sequence)
    #expect(after > before, "Listener stop must not stop recording")
    try server.start()
    #expect(server.token == token)
    #expect(app.activeConnectionSnapshot()?.state == .ready)
  }
}
