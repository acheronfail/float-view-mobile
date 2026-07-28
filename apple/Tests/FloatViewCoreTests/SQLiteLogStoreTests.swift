import CSQLite
import Foundation
import Testing
@testable import FloatViewCore
@testable import FloatViewLogging

private func repositoryRoot() -> URL {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0..<4 { root.deleteLastPathComponent() }
  return root
}

@Test func sqliteStoreCommitsAndClosesDurably() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-store-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "logging/sql"),
    availableBytes: { 2_000_000_000 }
  )
  let record = try store.create(deviceID: DeviceID("fixture"), at: .distantPast)
  try store.append(
    jobID: record.jobID,
    samples: [
      TelemetrySample(
        sequence: 1, observedAt: .distantPast, elapsedMs: 100,
        rideState: .riding, packVoltageV: 51.6
      ),
    ]
  )
  let closed = try store.close(jobID: record.jobID, at: .distantFuture, reason: "test")
  #expect(closed.committedSequence == 1)
  #expect(!closed.active)
  #expect(store.active() == nil)
}

@Test func sessionCatalogAndBoundedQueriesReadCanonicalSamples() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-session-query-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "logging/sql"),
    availableBytes: { 2_000_000_000 }
  )
  let startedAt = Date(timeIntervalSince1970: 1_700_000_000)
  let record = try store.create(deviceID: DeviceID("query-fixture"), at: startedAt)
  var samples: [TelemetrySample] = []
  for index in 0..<100 {
    let sequence = Int64(index + 1)
    let elapsed = Int64(index * 1_000)
    let voltage = 60.0 - Double(index) / 10.0
    let duty = Double(index) / 100.0
    samples.append(
      TelemetrySample(
        sequence: sequence,
        observedAt: startedAt.addingTimeInterval(Double(index)),
        elapsedMs: elapsed,
        rideState: index == 0 ? RideState.idle : RideState.riding,
        packVoltageV: voltage,
        dutyCycle: duty,
        extensionFields: ["speedKph": 18.0]
      )
    )
  }
  try store.append(
    jobID: record.jobID,
    samples: samples
  )
  _ = try store.close(
    jobID: record.jobID,
    at: startedAt.addingTimeInterval(100),
    reason: "test"
  )

  let page = try store.sessions(SessionListQuery(limit: 1))
  let summary = try #require(page.items.first)
  #expect(summary.sessionID == record.sessionID.value)
  #expect(summary.sampleCount == 100)
  #expect(summary.totalElapsedMs == 99_000)
  #expect(summary.ridingElapsedMs == 99_000)
  #expect(summary.distanceM.map { abs($0 - 495) < 0.001 } == true)
  #expect(summary.maxSpeedMps == 5)
  #expect(summary.maxDutyCycle == 0.99)

  let overview = try store.querySeries(
    sessionID: record.sessionID.value,
    startElapsedMs: 0,
    endElapsedMs: 99_000,
    fields: ["packVoltageV", "dutyCycle", "speedKph"],
    maxPoints: 12
  )
  #expect(!overview.isEmpty)
  #expect(overview.count <= 12)
  #expect(overview.allSatisfy { $0.values.keys.count == 3 })

  let diagnostic = try store.diagnosticWindow(
    sessionID: record.sessionID.value,
    centerElapsedMs: 50_000,
    beforeMs: 2_000,
    afterMs: 2_000,
    fields: ["packVoltageV"]
  )
  #expect(diagnostic.map(\.elapsedMs) == [48_000, 49_000, 50_000, 51_000, 52_000])
  #expect(try store.sessionEvents(
    sessionID: record.sessionID.value,
    startElapsedMs: 0,
    endElapsedMs: 99_000
  ).isEmpty)

  let short = try store.create(
    deviceID: DeviceID("query-fixture"),
    at: startedAt.addingTimeInterval(200)
  )
  try store.append(
    jobID: short.jobID,
    samples: [
      TelemetrySample(
        sequence: 1, observedAt: startedAt.addingTimeInterval(200),
        elapsedMs: 500, rideState: .idle, packVoltageV: 58
      ),
    ]
  )
  _ = try store.close(
    jobID: short.jobID,
    at: startedAt.addingTimeInterval(201),
    reason: "test"
  )
  let firstPage = try store.sessions(
    SessionListQuery(limit: 1, sortField: "totalElapsedMs", sortDescending: false)
  )
  #expect(firstPage.items.map(\.sessionID) == [short.sessionID.value])
  let nextCursor = try #require(firstPage.nextCursor)
  let secondPage = try store.sessions(
    SessionListQuery(
      cursor: nextCursor, limit: 1,
      minimumTotalElapsedMs: 1_000,
      sortField: "totalElapsedMs", sortDescending: false
    )
  )
  #expect(secondPage.items.map(\.sessionID) == [record.sessionID.value])
}

@Test func sqliteStoreRecoversAnInterruptedSessionAtStartupBoundary() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-recovery-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let schema = repositoryRoot().appending(path: "logging/sql")
  let interrupted = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  _ = try interrupted.create(deviceID: DeviceID("fixture"), at: .distantPast)

  let restarted = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  #expect(try restarted.recoverInterrupted() == 1)
  #expect(try restarted.recoverInterrupted() == 0)
}

@Test func schemaV2BackfillsExistingSessionSummaries() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-summary-migration-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
  let schema = repositoryRoot().appending(path: "logging/sql")
  let catalogURL = root.appending(path: "catalog.sqlite")
  try sqliteExecute(catalogURL, String(contentsOf: schema.appending(path: "catalog-v1.sql")))
  try sqliteExecute(
    catalogURL,
    """
    INSERT INTO session_catalog(
      session_id, job_id, device_history_id, device_snapshot_json,
      profile_snapshot_json, profile_revision, file_name, started_at_ms,
      state, integrity, requested_idle_ms, effective_idle_ms,
      requested_riding_ms, effective_riding_ms
    ) VALUES (
      'legacy-session', 'legacy-job', 'legacy-history', '{"displayName":"Legacy board"}',
      '{}', 0, 'session-legacy.sqlite', 0, 'completed', 'complete',
      1000, 1000, 100, 100
    );
    """
  )
  let sessionURL = root.appending(path: "session-legacy.sqlite")
  try sqliteExecute(sessionURL, String(contentsOf: schema.appending(path: "session-v1.sql")))
  try sqliteExecute(
    sessionURL,
    """
    INSERT INTO sample(
      sequence, observed_at_ms, monotonic_elapsed_ms, ride_state,
      raw_pack_voltage_v, duty_cycle, extension_json
    ) VALUES
      (1, 0, 0, 'idle', 60, 0.1, '{"speedKph":0}'),
      (2, 1000, 1000, 'riding', 59, 0.7, '{"speedKph":18}');
    """
  )

  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  let summary = try store.session(sessionID: "legacy-session")
  #expect(summary.displayName == "Legacy board")
  #expect(summary.sampleCount == 2)
  #expect(summary.totalElapsedMs == 1_000)
  #expect(summary.ridingElapsedMs == 1_000)
  #expect(summary.maxDutyCycle == 0.7)
}

@Test func storageThresholdsMatchContract() {
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.warningBytes) == .normal)
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.criticalBytes) == .warning)
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.stopBytes) == .critical)
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.stopBytes - 1) == .stop)
}

@Test func boardProfilesPersistOpaqueIdentityMergeAndRevisionConflicts() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-profile-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let schema = repositoryRoot().appending(path: "logging/sql")
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  let session = try store.create(deviceID: DeviceID("platform-private-id"), at: .distantPast)
  let saved = try #require(store.savedDevices().first)
  #expect(saved.savedDeviceID != "saved-platform-private-id")
  #expect(!saved.savedDeviceID.contains("platform-private-id"))
  #expect(!saved.profileComplete)

  let initial = try store.boardProfile(savedDeviceID: saved.savedDeviceID)
  #expect(initial.revision == 0)
  #expect(initial.missingFields == ["seriesCount", "parallelCount", "motor"])
  #expect(throws: ProductStoreError.invalidProfilePatch) {
    try store.updateBoardProfile(
      savedDeviceID: saved.savedDeviceID,
      expectedRevision: 0,
      patchJSON: #"{"seriesCount":0}"#,
      at: Date(timeIntervalSince1970: 1)
    )
  }

  let updated = try store.updateBoardProfile(
    savedDeviceID: saved.savedDeviceID,
    expectedRevision: 0,
    patchJSON: #"{"motor":"hypercore_xr","seriesCount":18}"#,
    at: .distantPast.addingTimeInterval(1)
  )
  #expect(updated.revision == 1)
  #expect(updated.missingFields == ["parallelCount"])
  #expect(updated.valuesJSON.contains(#""seriesCount":18"#))
  #expect(throws: ProductStoreError.profileRevisionConflict(currentRevision: 1)) {
    try store.updateBoardProfile(
      savedDeviceID: saved.savedDeviceID,
      expectedRevision: 0,
      patchJSON: #"{"seriesCount":20}"#,
      at: .distantPast.addingTimeInterval(2)
    )
  }
  _ = try store.close(jobID: session.jobID, at: .distantPast.addingTimeInterval(3), reason: "test")

  let catalogURL = root.appending(path: "catalog.sqlite")
  #expect(try sqliteText(catalogURL, "SELECT profile_snapshot_json FROM session_catalog") == "{}")
  #expect(try sqliteInt(catalogURL, "SELECT profile_revision FROM session_catalog") == 0)
  #expect(try sqliteInt(catalogURL, "SELECT event_count FROM session_catalog") == 1)
  let storedSessionFile = try sqliteText(
    catalogURL,
    "SELECT file_name FROM session_catalog"
  )
  let sessionFile = try #require(storedSessionFile)
  let sessionURL = root.appending(path: sessionFile)
  #expect(
    try sqliteText(sessionURL, "SELECT kind FROM session_event") == "board_profile_changed"
  )
  #expect(
    try sqliteText(sessionURL, "SELECT details_json FROM session_event")?
      .contains(#""revision":1"#) == true
  )

  let reopened = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  let persisted = try reopened.boardProfile(savedDeviceID: saved.savedDeviceID)
  #expect(persisted == updated)
  #expect(try reopened.savedDevices().first?.profileComplete == false)
}

@Test func boardProfileMigrationRekeysLegacyPlatformDerivedIdentifiers() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-profile-migration-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
  let catalogURL = root.appending(path: "catalog.sqlite")
  try sqliteExecute(
    catalogURL,
    String(contentsOf: repositoryRoot().appending(path: "logging/sql/catalog-v1.sql"))
  )
  try sqliteExecute(
    catalogURL,
    """
    INSERT INTO saved_device(
      saved_device_id, device_history_id, opaque_platform_id, display_name, created_at_ms
    ) VALUES ('saved-platform-id', 'history-platform-id', 'platform-id', 'Board', 0);
    INSERT INTO board_profile(saved_device_id, revision, json, updated_at_ms)
      VALUES ('saved-platform-id', 1, '{"seriesCount":18}', 0);
    INSERT INTO session_catalog(
      session_id, job_id, device_history_id, device_snapshot_json,
      profile_snapshot_json, profile_revision, file_name, started_at_ms,
      state, integrity, requested_idle_ms, effective_idle_ms,
      requested_riding_ms, effective_riding_ms
    ) VALUES (
      'session', 'job', 'history-platform-id',
      '{"savedDeviceId":"saved-platform-id","deviceHistoryId":"history-platform-id"}',
      '{}', 0, 'session.sqlite', 0, 'completed', 'complete', 1000, 1000, 100, 100
    );
    """
  )

  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "logging/sql"),
    availableBytes: { 2_000_000_000 }
  )
  let migrated = try #require(store.savedDevices().first)
  #expect(!migrated.savedDeviceID.contains("platform-id"))
  #expect(!migrated.deviceHistoryID.contains("platform-id"))
  #expect(try store.boardProfile(savedDeviceID: migrated.savedDeviceID).revision == 1)
  let storedSnapshot = try sqliteText(
    catalogURL, "SELECT device_snapshot_json FROM session_catalog"
  )
  let snapshot = try #require(storedSnapshot)
  #expect(snapshot.contains(migrated.savedDeviceID))
  #expect(snapshot.contains(migrated.deviceHistoryID))
  #expect(!snapshot.contains("platform-id"))
}

private func sqliteExecute(_ url: URL, _ sql: String) throws {
  var database: OpaquePointer?
  guard sqlite3_open_v2(
    url.path, &database, SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE, nil
  ) == SQLITE_OK else {
    throw SQLiteLogStoreError.sqlite("Unable to create test database")
  }
  defer { sqlite3_close(database) }
  var message: UnsafeMutablePointer<CChar>?
  guard sqlite3_exec(database, sql, nil, nil, &message) == SQLITE_OK else {
    let text = message.map { String(cString: $0) } ?? "Unable to seed test database"
    sqlite3_free(message)
    throw SQLiteLogStoreError.sqlite(text)
  }
}

private func sqliteText(_ url: URL, _ sql: String) throws -> String? {
  var database: OpaquePointer?
  guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
    throw SQLiteLogStoreError.sqlite("Unable to inspect test database")
  }
  defer { sqlite3_close(database) }
  var statement: OpaquePointer?
  guard sqlite3_prepare_v2(database, sql, -1, &statement, nil) == SQLITE_OK else {
    throw SQLiteLogStoreError.sqlite("Unable to prepare test query")
  }
  defer { sqlite3_finalize(statement) }
  guard sqlite3_step(statement) == SQLITE_ROW else { return nil }
  guard sqlite3_column_type(statement, 0) != SQLITE_NULL,
        let value = sqlite3_column_text(statement, 0)
  else { return nil }
  return String(cString: value)
}

private func sqliteInt(_ url: URL, _ sql: String) throws -> Int? {
  var database: OpaquePointer?
  guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
    throw SQLiteLogStoreError.sqlite("Unable to inspect test database")
  }
  defer { sqlite3_close(database) }
  var statement: OpaquePointer?
  guard sqlite3_prepare_v2(database, sql, -1, &statement, nil) == SQLITE_OK else {
    throw SQLiteLogStoreError.sqlite("Unable to prepare test query")
  }
  defer { sqlite3_finalize(statement) }
  guard sqlite3_step(statement) == SQLITE_ROW else { return nil }
  return Int(sqlite3_column_int64(statement, 0))
}
