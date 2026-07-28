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
