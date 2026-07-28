import Foundation
import SQLite3

private let expectedName = "FloatView Test VESC"
private let expectedTelemetryCount = 168

private func emit(_ kind: String, _ fields: [String: Any] = [:]) {
  var event = fields
  event["kind"] = kind
  event["schemaVersion"] = 1
  let data = try! JSONSerialization.data(withJSONObject: event, options: [.sortedKeys])
  print(String(decoding: data, as: UTF8.self))
  fflush(stdout)
}

private final class ClosureEventSink: EventSink, @unchecked Sendable {
  var handler: (@Sendable (any Sendable) -> Void)?
  func publish(_ event: any Sendable) { handler?(event) }
}

private struct T3SystemClock: ClockPort {
  func monotonicMilliseconds() -> Int64 {
    Int64(ProcessInfo.processInfo.systemUptime * 1_000)
  }
  func now() -> Date { Date() }
}

private final class T3MacSessionDriver: @unchecked Sendable {
  private let logRoot: URL
  private let store: SQLiteLogStore
  private let sink = ClosureEventSink()
  private var adapter: CoreBluetoothAdapter!
  private var service: ApplicationService!
  private var selectedDevice: DeviceID?
  private var discoveryStarted = false
  private var disconnectRequested = false
  private var finished = false
  private var telemetryCount = 0
  private var ridingCount = 0
  private var idleCount = 0
  private var sawFootpadWarning = false

  init(logRoot: URL, schemaDirectory: URL) throws {
    self.logRoot = logRoot
    store = try SQLiteLogStore(
      root: logRoot,
      schemaDirectory: schemaDirectory,
      availableBytes: { Int64.max }
    )
    adapter = CoreBluetoothAdapter(
      eventHandler: { [weak self] event in self?.receive(event) },
      diagnosticHandler: { diagnostic in emit("diagnostic", ["value": diagnostic]) }
    )
    service = ApplicationService(
      bluetooth: adapter,
      clock: T3SystemClock(),
      logStore: store,
      productStore: store,
      sessionStore: store,
      protocolCodec: VescProtocolService(),
      scheduler: SystemScheduler(),
      events: sink
    )
    sink.handler = { [weak self] event in self?.observe(event) }
  }

  func run() -> Never {
    DispatchQueue.global().asyncAfter(deadline: .now() + 45) { [weak self] in
      guard let self, !self.finished else { return }
      self.fail("timeout", "telemetryCount=\(self.telemetryCount)")
    }
    RunLoop.main.run()
    fatalError("main run loop returned")
  }

  private func receive(_ event: BluetoothEvent) {
    do {
      try service.receive(event)
      switch event {
      case let .stateChanged(state):
        if state.radio == .poweredOn, !discoveryStarted {
          discoveryStarted = true
          _ = try service.startDiscovery()
        }
      case let .deviceFound(deviceID, name, _):
        guard selectedDevice == nil, name == expectedName else { return }
        selectedDevice = deviceID
        try service.connect(deviceID)
      case let .connectionChanged(deviceID, state):
        guard deviceID == selectedDevice else { return }
        if state == .disconnected, disconnectRequested {
          try finish()
        } else if state == .failed {
          fail("connectionFailed")
        }
      case let .scanFailed(reason):
        fail("scanFailed", reason)
      case .notification:
        break
      }
    } catch {
      fail("applicationFailure", String(describing: error))
    }
  }

  private func observe(_ event: any Sendable) {
    guard let sample = event as? TelemetrySample else { return }
    telemetryCount += 1
    if sample.rideState == .riding { ridingCount += 1 } else { idleCount += 1 }
    if sample.footpadState == 1 { sawFootpadWarning = true }
    guard telemetryCount == expectedTelemetryCount, !disconnectRequested else { return }
    disconnectRequested = true
    do {
      try service.disconnect()
    } catch {
      fail("explicitDisconnectFailed", String(describing: error))
    }
  }

  private func finish() throws -> Never {
    finished = true
    guard store.active() == nil else { fail("activeSessionRemains") }
    guard service.activeConnectionSnapshot() == nil else {
      fail("activeConnectionRemains")
    }
    let sessions = try store.sessions(SessionListQuery(limit: 10))
    guard sessions.items.count == 1, let session = sessions.items.first else {
      fail("sessionCount", "\(sessions.items.count)")
    }
    guard session.state == "completed", session.integrity == "complete",
          session.endedAt != nil, session.sampleCount >= 160,
          session.ridingElapsedMs > 0, session.totalElapsedMs >= 18_000
    else {
      fail(
        "sessionSummary",
        "state=\(session.state) integrity=\(session.integrity) samples=\(session.sampleCount) " +
          "ridingMs=\(session.ridingElapsedMs) totalMs=\(session.totalElapsedMs)"
      )
    }
    let footpad = try store.diagnosticWindow(
      sessionID: session.sessionID,
      centerElapsedMs: session.totalElapsedMs / 2,
      beforeMs: session.totalElapsedMs,
      afterMs: session.totalElapsedMs,
      fields: ["footpadState"]
    )
    guard footpad.contains(where: { $0.values["footpadState"] == 1 }) else {
      fail("footpadEvidenceMissing")
    }
    let sqliteFiles = try FileManager.default.contentsOfDirectory(
      at: logRoot,
      includingPropertiesForKeys: nil
    ).filter { $0.pathExtension == "sqlite" }
    guard !sqliteFiles.isEmpty,
          try sqliteFiles.allSatisfy({ try sqliteIntegrity($0) == "ok" })
    else {
      fail("sqliteIntegrity")
    }
    emit("passed", [
      "sessionId": session.sessionID,
      "sessionState": session.state,
      "sessionIntegrity": session.integrity,
      "sampleCount": session.sampleCount,
      "ridingElapsedMs": session.ridingElapsedMs,
      "totalElapsedMs": session.totalElapsedMs,
      "telemetryCount": telemetryCount,
      "ridingTelemetryCount": ridingCount,
      "idleTelemetryCount": idleCount,
      "sawFootpadWarning": sawFootpadWarning,
      "sqliteIntegrity": "ok",
      "activeConnection": false,
      "activeSession": false
    ])
    exit(0)
  }

  private func fail(_ code: String, _ detail: String = "") -> Never {
    emit("failed", [
      "code": code,
      "detail": detail,
      "telemetryCount": telemetryCount
    ])
    exit(1)
  }
}

private func sqliteIntegrity(_ url: URL) throws -> String {
  var database: OpaquePointer?
  guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
    defer { sqlite3_close(database) }
    throw NSError(domain: "T3SQLite", code: 1)
  }
  defer { sqlite3_close(database) }
  var statement: OpaquePointer?
  guard sqlite3_prepare_v2(database, "PRAGMA integrity_check", -1, &statement, nil) == SQLITE_OK else {
    throw NSError(domain: "T3SQLite", code: 2)
  }
  defer { sqlite3_finalize(statement) }
  guard sqlite3_step(statement) == SQLITE_ROW,
        let value = sqlite3_column_text(statement, 0)
  else { throw NSError(domain: "T3SQLite", code: 3) }
  return String(cString: value)
}

@main
private enum T3MacSessionClient {
  static func main() throws {
    guard CommandLine.arguments.count == 3 else {
      emit("failed", ["code": "arguments"])
      exit(2)
    }
    let driver = try T3MacSessionDriver(
      logRoot: URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true),
      schemaDirectory: URL(fileURLWithPath: CommandLine.arguments[2], isDirectory: true)
    )
    driver.run()
  }
}
