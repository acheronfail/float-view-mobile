import Foundation
import SQLite3
import XCTest

final class T3VescTestDeviceTests: XCTestCase {
  func testRide() throws {
    try runScenario(.ride)
  }

  func testUnexpectedDisconnect() throws {
    try runScenario(.unexpectedDisconnect)
  }

  private func runScenario(_ mode: T3VerificationMode) throws {
    let completed = expectation(description: "\(mode.rawValue) scenario completed")
    let root = FileManager.default.temporaryDirectory
      .appending(path: "float-view-t3-ios-\(UUID().uuidString)", directoryHint: .isDirectory)
    guard let resources = Bundle(for: Self.self).resourceURL else {
      XCTFail("hardware-test resource directory is unavailable")
      return
    }
    let schemaDirectory = resources.appending(path: "sql", directoryHint: .isDirectory)
    let outcome = T3OutcomeBox()
    let driver = try T3IOSSessionDriver(
      mode: mode,
      logRoot: root,
      schemaDirectory: schemaDirectory
    ) { result in
      outcome.set(result)
      completed.fulfill()
    }

    driver.start()
    wait(for: [completed], timeout: 55)
    withExtendedLifetime(driver) {}

    guard let result = outcome.get() else {
      XCTFail("iOS T3 driver completed without an outcome")
      return
    }
    let evidence: T3IOSSessionEvidence
    switch result {
    case let .success(value):
      evidence = value
    case let .failure(error):
      XCTFail(String(describing: error))
      return
    }
    let encoded = try JSONEncoder().encode(evidence)
    print("T3_IOS_RESULT:\(encoded.base64EncodedString())")
  }
}

private enum T3VerificationMode: String {
  case ride
  case unexpectedDisconnect = "unexpected-disconnect"

  var expectedTelemetryCount: Int {
    switch self {
    case .ride: 168
    case .unexpectedDisconnect: 5
    }
  }
}

private struct T3IOSSessionEvidence: Codable, Sendable {
  let sessionId: String
  let sessionState: String
  let sessionIntegrity: String
  let sampleCount: Int64
  let ridingElapsedMs: Int64
  let totalElapsedMs: Int64
  let telemetryCount: Int
  let ridingTelemetryCount: Int
  let idleTelemetryCount: Int
  let sawFootpadWarning: Bool
  let sawUnexpectedDisconnectReason: Bool
  let closeReason: String
  let mode: String
  let sqliteIntegrity: String
  let activeConnection: Bool
  let activeSession: Bool
}

private struct T3TestFailure: Error, CustomStringConvertible {
  let code: String
  let detail: String

  var description: String {
    detail.isEmpty ? code : "\(code): \(detail)"
  }
}

private final class T3OutcomeBox: @unchecked Sendable {
  private let lock = NSLock()
  private var value: Result<T3IOSSessionEvidence, Error>?

  func set(_ value: Result<T3IOSSessionEvidence, Error>) {
    lock.lock()
    defer { lock.unlock() }
    self.value = value
  }

  func get() -> Result<T3IOSSessionEvidence, Error>? {
    lock.lock()
    defer { lock.unlock() }
    return value
  }
}

private final class T3ClosureEventSink: EventSink, @unchecked Sendable {
  var handler: (@Sendable (any Sendable) -> Void)?
  func publish(_ event: any Sendable) { handler?(event) }
}

private struct T3SystemClock: ClockPort {
  func monotonicMilliseconds() -> Int64 {
    Int64(ProcessInfo.processInfo.systemUptime * 1_000)
  }

  func now() -> Date { Date() }
}

private final class T3IOSSessionDriver: @unchecked Sendable {
  private static let expectedName = "FloatView Test VESC"
  // The deterministic scenario nominally persists 160 samples. CoreBluetooth
  // delivery jitter can legitimately coalesce due decisions around the 80 ms
  // early allowance, so iOS requires at least 90% of that nominal coverage.
  private static let minimumRideSampleCount: Int64 = 144

  private let mode: T3VerificationMode
  private let logRoot: URL
  private let store: SQLiteLogStore
  private let sink = T3ClosureEventSink()
  private let completion: @Sendable (Result<T3IOSSessionEvidence, Error>) -> Void
  private var adapter: CoreBluetoothAdapter!
  private var service: ApplicationService!
  private var selectedDevice: DeviceID?
  private var discoveryStarted = false
  private var disconnectRequested = false
  private var terminal = false
  private var telemetryCount = 0
  private var ridingCount = 0
  private var idleCount = 0
  private var sawFootpadWarning = false
  private var sawUnexpectedDisconnectReason = false
  private var preReadyDisconnects = 0

  init(
    mode: T3VerificationMode,
    logRoot: URL,
    schemaDirectory: URL,
    completion: @escaping @Sendable (Result<T3IOSSessionEvidence, Error>) -> Void
  ) throws {
    self.mode = mode
    self.logRoot = logRoot
    self.completion = completion
    store = try SQLiteLogStore(
      root: logRoot,
      schemaDirectory: schemaDirectory,
      availableBytes: { Int64.max }
    )
    adapter = CoreBluetoothAdapter(
      eventHandler: { [weak self] event in self?.receive(event) },
      diagnosticHandler: { diagnostic in
        print("T3_IOS_DIAGNOSTIC:\(diagnostic)")
      }
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

  func start() {
    DispatchQueue.global().asyncAfter(deadline: .now() + 45) { [weak self] in
      guard let self, !self.terminal else { return }
      self.fail("timeout", "mode=\(self.mode.rawValue) telemetryCount=\(self.telemetryCount)")
    }
    let state = adapter.currentState()
    if state.radio == .poweredOn {
      startDiscovery()
    }
  }

  private func receive(_ event: BluetoothEvent) {
    guard !terminal else { return }
    do {
      try service.receive(event)
      switch event {
      case let .stateChanged(state):
        if state.radio == .poweredOn {
          startDiscovery()
        } else if state.radio == .unsupported || state.radio == .poweredOff {
          fail("bluetooth_\(state.radio.rawValue)")
        }
      case let .deviceFound(deviceID, name, _):
        guard selectedDevice == nil, name == Self.expectedName else { return }
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
            discoveryStarted = false
            startDiscovery()
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

  private func startDiscovery() {
    guard !discoveryStarted, !terminal else { return }
    do {
      discoveryStarted = true
      _ = try service.startDiscovery()
    } catch {
      fail("discoveryFailure", String(describing: error))
    }
  }

  private func observe(_ event: any Sendable) {
    guard !terminal else { return }
    if let reason = event as? String, reason == "unexpected_disconnect" {
      sawUnexpectedDisconnectReason = true
      return
    }
    guard let sample = event as? TelemetrySample else { return }
    telemetryCount += 1
    if sample.rideState == .riding {
      ridingCount += 1
    } else {
      idleCount += 1
    }
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

  private func finish() throws {
    guard !terminal else { return }
    guard store.active() == nil else {
      fail("activeSessionRemains")
      return
    }
    guard service.activeConnectionSnapshot() == nil else {
      fail("activeConnectionRemains")
      return
    }
    let sessions = try store.sessions(SessionListQuery(limit: 10))
    guard sessions.items.count == 1, let session = sessions.items.first else {
      fail("sessionCount", "\(sessions.items.count)")
      return
    }
    let closeReason = try t3SQLiteText(
      logRoot.appending(path: "catalog.sqlite"),
      sql: "SELECT close_reason FROM session_catalog WHERE session_id = ?",
      value: session.sessionID
    )
    let coveragePassed: Bool
    switch mode {
    case .ride:
      coveragePassed = session.sampleCount >= Self.minimumRideSampleCount
        && session.ridingElapsedMs > 0
        && session.totalElapsedMs >= 18_000
        && closeReason == "explicit_disconnect"
    case .unexpectedDisconnect:
      coveragePassed = telemetryCount == mode.expectedTelemetryCount
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
        "state=\(session.state) integrity=\(session.integrity) samples=\(session.sampleCount) "
          + "ridingMs=\(session.ridingElapsedMs) totalMs=\(session.totalElapsedMs) "
          + "closeReason=\(closeReason ?? "nil")"
      )
      return
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
        return
      }
    }
    let sqliteFiles = try FileManager.default.contentsOfDirectory(
      at: logRoot,
      includingPropertiesForKeys: nil
    ).filter { $0.pathExtension == "sqlite" }
    guard !sqliteFiles.isEmpty,
          try sqliteFiles.allSatisfy({ try t3SQLiteIntegrity($0) == "ok" })
    else {
      fail("sqliteIntegrity")
      return
    }

    terminal = true
    completion(.success(T3IOSSessionEvidence(
      sessionId: session.sessionID,
      sessionState: session.state,
      sessionIntegrity: session.integrity,
      sampleCount: session.sampleCount,
      ridingElapsedMs: session.ridingElapsedMs,
      totalElapsedMs: session.totalElapsedMs,
      telemetryCount: telemetryCount,
      ridingTelemetryCount: ridingCount,
      idleTelemetryCount: idleCount,
      sawFootpadWarning: sawFootpadWarning,
      sawUnexpectedDisconnectReason: sawUnexpectedDisconnectReason,
      closeReason: closeReason ?? "",
      mode: mode.rawValue,
      sqliteIntegrity: "ok",
      activeConnection: false,
      activeSession: false
    )))
  }

  private func fail(_ code: String, _ detail: String = "") {
    guard !terminal else { return }
    terminal = true
    print("T3_IOS_FAILURE:\(code):\(detail)")
    completion(.failure(T3TestFailure(code: code, detail: detail)))
  }
}

private func t3SQLiteIntegrity(_ url: URL) throws -> String {
  var database: OpaquePointer?
  guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
    defer { sqlite3_close(database) }
    throw T3TestFailure(code: "sqliteOpen", detail: url.lastPathComponent)
  }
  defer { sqlite3_close(database) }
  var statement: OpaquePointer?
  guard sqlite3_prepare_v2(database, "PRAGMA integrity_check", -1, &statement, nil) == SQLITE_OK
  else {
    throw T3TestFailure(code: "sqlitePrepare", detail: url.lastPathComponent)
  }
  defer { sqlite3_finalize(statement) }
  guard sqlite3_step(statement) == SQLITE_ROW,
        let value = sqlite3_column_text(statement, 0)
  else {
    throw T3TestFailure(code: "sqliteIntegrityRead", detail: url.lastPathComponent)
  }
  return String(cString: value)
}

private func t3SQLiteText(
  _ url: URL,
  sql: String,
  value: String
) throws -> String? {
  var database: OpaquePointer?
  guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
    defer { sqlite3_close(database) }
    throw T3TestFailure(code: "sqliteOpen", detail: url.lastPathComponent)
  }
  defer { sqlite3_close(database) }
  var statement: OpaquePointer?
  guard sqlite3_prepare_v2(database, sql, -1, &statement, nil) == SQLITE_OK else {
    throw T3TestFailure(code: "sqlitePrepare", detail: url.lastPathComponent)
  }
  defer { sqlite3_finalize(statement) }
  let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
  sqlite3_bind_text(statement, 1, value, -1, transient)
  guard sqlite3_step(statement) == SQLITE_ROW else { return nil }
  guard let text = sqlite3_column_text(statement, 0) else { return nil }
  return String(cString: text)
}
