import Foundation
import SQLite3

private let expectedName = "FloatView Test VESC"

private enum VerificationMode: String {
  case ride
  case unexpectedDisconnect = "unexpected-disconnect"

  var expectedTelemetryCount: Int {
    switch self {
    case .ride: 168
    case .unexpectedDisconnect: 5
    }
  }
}

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
  private let mode: VerificationMode
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
  private var sawUnexpectedDisconnectReason = false
  private var preReadyDisconnects = 0

  init(mode: VerificationMode, logRoot: URL, schemaDirectory: URL) throws {
    self.mode = mode
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
      events: sink,
      samplingSettings: mode == .unexpectedDisconnect
        ? SamplingSettings(reconnectDurationMs: 0)
        : SamplingSettings()
    )
    sink.handler = { [weak self] event in self?.observe(event) }
  }

  func run() -> Never {
    DispatchQueue.global().asyncAfter(deadline: .now() + 45) { [weak self] in
      guard let self, !self.finished else { return }
      self.fail("timeout", "mode=\(self.mode.rawValue) telemetryCount=\(self.telemetryCount)")
    }
    RunLoop.main.run()
    fatalError("main run loop returned")
  }

  private func receive(_ event: BluetoothEvent) {
    do {
      try service.receive(event)
      switch event {
      case let .stateChanged(state):
        emit("bluetoothState", [
          "radio": state.radio.rawValue,
          "scanPermission": state.scanPermission.rawValue,
          "connectPermission": state.connectPermission.rawValue
        ])
        if state.radio == .poweredOn, !discoveryStarted {
          discoveryStarted = true
          _ = try service.startDiscovery()
        }
      case let .deviceFound(deviceID, name, _):
        emit("discovery", ["matchedExpectedName": name == expectedName])
        guard selectedDevice == nil, name == expectedName else { return }
        selectedDevice = deviceID
        try service.connect(deviceID)
      case let .connectionChanged(deviceID, state):
        guard deviceID == selectedDevice else { return }
        if state == .disconnected {
          if disconnectRequested
            || (mode == .unexpectedDisconnect
              && telemetryCount == mode.expectedTelemetryCount
              && sawUnexpectedDisconnectReason)
          {
            try finish()
          } else if telemetryCount == 0, preReadyDisconnects < 2 {
            preReadyDisconnects += 1
            selectedDevice = nil
            _ = try service.startDiscovery()
          } else {
            fail(
              "unexpectedEarlyDisconnect",
              "telemetryCount=\(telemetryCount) retries=\(preReadyDisconnects)"
            )
          }
        } else if state == .failed, mode == .ride, telemetryCount > 0 {
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
    if let reason = event as? String, reason == "unexpected_disconnect" {
      sawUnexpectedDisconnectReason = true
      return
    }
    guard let sample = event as? TelemetrySample else { return }
    telemetryCount += 1
    if sample.rideState == .riding { ridingCount += 1 } else { idleCount += 1 }
    if sample.footpadState == 1 { sawFootpadWarning = true }
    guard mode == .ride,
          telemetryCount == mode.expectedTelemetryCount,
          !disconnectRequested
    else { return }
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
    let closeReason = try sqliteText(
      logRoot.appending(path: "catalog.sqlite"),
      sql: "SELECT close_reason FROM session_catalog WHERE session_id = ?",
      value: session.sessionID
    )
    let coveragePassed = switch mode {
    case .ride:
      session.sampleCount >= 160
        && session.ridingElapsedMs > 0
        && session.totalElapsedMs >= 18_000
        && closeReason == "explicit_disconnect"
    case .unexpectedDisconnect:
      telemetryCount == mode.expectedTelemetryCount
        && session.sampleCount >= 3
        && session.ridingElapsedMs > 0
        && session.totalElapsedMs >= 1_000
        && closeReason == "unexpected_disconnect"
        && sawUnexpectedDisconnectReason
    }
    guard session.state == "completed", session.integrity == "complete",
          session.endedAt != nil, coveragePassed
    else {
      fail(
        "sessionSummary",
        "state=\(session.state) integrity=\(session.integrity) samples=\(session.sampleCount) " +
          "ridingMs=\(session.ridingElapsedMs) totalMs=\(session.totalElapsedMs) " +
          "closeReason=\(closeReason ?? "nil")"
      )
    }
    if mode == .ride {
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
      "sawUnexpectedDisconnectReason": sawUnexpectedDisconnectReason,
      "closeReason": closeReason ?? "",
      "mode": mode.rawValue,
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

private func sqliteText(_ url: URL, sql: String, value: String) throws -> String? {
  var database: OpaquePointer?
  guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
    defer { sqlite3_close(database) }
    throw NSError(domain: "T3SQLite", code: 4)
  }
  defer { sqlite3_close(database) }
  var statement: OpaquePointer?
  guard sqlite3_prepare_v2(database, sql, -1, &statement, nil) == SQLITE_OK else {
    throw NSError(domain: "T3SQLite", code: 5)
  }
  defer { sqlite3_finalize(statement) }
  let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
  sqlite3_bind_text(statement, 1, value, -1, transient)
  guard sqlite3_step(statement) == SQLITE_ROW else { return nil }
  guard let text = sqlite3_column_text(statement, 0) else { return nil }
  return String(cString: text)
}

@main
private enum T3MacSessionClient {
  static func main() throws {
    guard CommandLine.arguments.count == 4,
          let mode = VerificationMode(rawValue: CommandLine.arguments[3])
    else {
      emit("failed", ["code": "arguments"])
      exit(2)
    }
    let driver = try T3MacSessionDriver(
      mode: mode,
      logRoot: URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true),
      schemaDirectory: URL(fileURLWithPath: CommandLine.arguments[2], isDirectory: true)
    )
    driver.run()
  }
}
