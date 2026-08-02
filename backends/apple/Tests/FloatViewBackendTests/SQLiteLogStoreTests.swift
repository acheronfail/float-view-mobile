import CSQLite
@testable import FloatViewBackend
import Foundation
import Testing

private func repositoryRoot() -> URL {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  return root
}

private func explainPlan(database: URL, sql: String) throws -> String {
  var handle: OpaquePointer?
  guard sqlite3_open_v2(database.path, &handle, SQLITE_OPEN_READONLY | SQLITE_OPEN_FULLMUTEX, nil) == SQLITE_OK,
        let handle
  else { throw SQLiteLogStoreError.sqlite("Unable to open plan database") }
  defer { sqlite3_close(handle) }
  var statement: OpaquePointer?
  guard sqlite3_prepare_v2(handle, "EXPLAIN QUERY PLAN \(sql)", -1, &statement, nil) == SQLITE_OK,
        let statement
  else { throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle))) }
  defer { sqlite3_finalize(statement) }
  var details: [String] = []
  while sqlite3_step(statement) == SQLITE_ROW {
    if let text = sqlite3_column_text(statement, 3) {
      details.append(String(cString: text))
    }
  }
  return details.joined(separator: " | ")
}

private func executeSQLite(database: URL, sql: String) throws {
  var handle: OpaquePointer?
  guard sqlite3_open_v2(database.path, &handle, SQLITE_OPEN_READWRITE | SQLITE_OPEN_FULLMUTEX, nil) == SQLITE_OK,
        let handle
  else { throw SQLiteLogStoreError.sqlite("Unable to open SQLite database") }
  defer { sqlite3_close(handle) }
  guard sqlite3_exec(handle, sql, nil, nil, nil) == SQLITE_OK else {
    throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
  }
}

@Test func sqliteStoreDiscardsSessionsBelowMinimumDistance() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-store-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
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
  #expect(try store.sessions(SessionListQuery()).items.isEmpty)
}

@Test func sessionCatalogAndBoundedQueriesReadCanonicalSamples() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-session-query-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
  let startedAt = Date(timeIntervalSince1970: 1_700_000_000)
  let record = try store.create(deviceID: DeviceID("query-fixture"), at: startedAt)
  var samples: [TelemetrySample] = []
  for index in 0 ..< 100 {
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
        packageState: [.disabled, .startup, .ready, .running][index % 4],
        packVoltageV: voltage,
        dutyCycle: duty,
        speedKph: 18.0,
        pitchDeg: Double(index % 4),
        extensionFields: ["future.energy": Double(index) + 0.25]
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
    fields: ["packVoltageV", "dutyCycle", "speedKph", "future.energy"],
    maxPoints: 12
  )
  #expect(!overview.isEmpty)
  #expect(overview.count <= 12)
  #expect(overview.allSatisfy { $0.values.keys.count == 4 })
  let nearest = try store.queryNearestSample(
    sessionID: record.sessionID.value,
    targetElapsedMs: 500,
    fields: ["packVoltageV", "future.energy"],
    publishedSampleSequence: 100
  )
  #expect(nearest?.sequence == 1)
  #expect(nearest?.rideState == RideState.idle.rawValue)
  #expect(try store.sessionExtensionFields(sessionID: record.sessionID.value) == [
    "future.energy",
  ])

  let diagnostic = try store.querySampleWindow(
    sessionID: record.sessionID.value,
    centerElapsedMs: 50_000,
    beforeMs: 2_000,
    afterMs: 2_000,
    fields: ["packVoltageV", "pitchDeg"]
  )
  #expect(diagnostic.map(\.elapsedMs) == [48_000, 49_000, 50_000, 51_000, 52_000])
  #expect(diagnostic.compactMap { $0.values["pitchDeg"] ?? nil } == [0.0, 1.0, 2.0, 3.0, 0.0])
  #expect(try store.sessionEvents(
    sessionID: record.sessionID.value,
    startElapsedMs: 0,
    endElapsedMs: 99_000
  ).filter { $0.kind == "packageState" }.count == 100)

  let short = try store.create(
    deviceID: DeviceID("query-fixture"),
    at: startedAt.addingTimeInterval(200)
  )
  try store.append(
    jobID: short.jobID,
    samples: [
      TelemetrySample(
        sequence: 1, observedAt: startedAt.addingTimeInterval(200),
        elapsedMs: 0, rideState: .idle, packVoltageV: 58, speedKph: 36
      ),
      TelemetrySample(
        sequence: 2, observedAt: startedAt.addingTimeInterval(220),
        elapsedMs: 20_000, rideState: .idle, packVoltageV: 58, speedKph: 36
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
  #expect(firstPage.items.map(\.sessionID) == [record.sessionID.value])
  #expect(firstPage.nextCursor == nil)
}

@Test func historicalReadsHonorThePublishedSnapshotBoundary() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-snapshot-boundary-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
  let startedAt = Date(timeIntervalSince1970: 1_700_100_000)
  let record = try store.create(deviceID: DeviceID("snapshot-fixture"), at: startedAt)
  try store.append(jobID: record.jobID, samples: [
    TelemetrySample(
      sequence: 1,
      observedAt: startedAt,
      elapsedMs: 0,
      rideState: .idle,
      packageState: .ready,
      packVoltageV: 58
    ),
  ])
  let published = try store.sessionSnapshot(sessionID: record.sessionID.value)
  try store.append(jobID: record.jobID, samples: [
    TelemetrySample(
      sequence: 2,
      observedAt: startedAt.addingTimeInterval(1),
      elapsedMs: 1_000,
      rideState: .riding,
      packageState: .running,
      packVoltageV: 57
    ),
  ])

  let boundedSamples = try store.querySampleWindow(
    sessionID: record.sessionID.value,
    centerElapsedMs: 500,
    beforeMs: 1_000,
    afterMs: 1_000,
    fields: ["packVoltageV"],
    publishedSampleSequence: published.committedSampleSequence
  )
  #expect(boundedSamples.map(\.sequence) == [1])
  #expect(try store.querySampleWindow(
    sessionID: record.sessionID.value,
    centerElapsedMs: 500,
    beforeMs: 1_000,
    afterMs: 1_000,
    fields: ["packVoltageV"]
  ).map(\.sequence) == [1, 2])

  let boundedEvents = try store.sessionEvents(
    sessionID: record.sessionID.value,
    startElapsedMs: 0,
    endElapsedMs: 1_000,
    publishedEventSequence: published.committedEventSequence
  )
  #expect(boundedEvents.allSatisfy { (Int64($0.eventID) ?? 0) <= published.committedEventSequence })
  #expect(try store.sessionEvents(
    sessionID: record.sessionID.value,
    startElapsedMs: 0,
    endElapsedMs: 1_000
  ).count > boundedEvents.count)
}

@Test func representativeHistoricalQueriesUseElapsedIndexes() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-query-plans-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
  let record = try store.create(deviceID: DeviceID("plan-fixture"), at: Date(timeIntervalSince1970: 1_700_200_000))
  var samples: [TelemetrySample] = []
  for index in 0 ..< 4 {
    let sample = TelemetrySample(
      sequence: Int64(index + 1),
      observedAt: Date(timeIntervalSince1970: 1_700_200_000 + Double(index)),
      elapsedMs: Int64(index * 1_000), rideState: .riding,
      packageState: .running, packVoltageV: 58 - Double(index), speedKph: 360
    )
    samples.append(sample)
  }
  try store.append(jobID: record.jobID, samples: samples)
  _ = try store.close(jobID: record.jobID, at: Date(timeIntervalSince1970: 1_700_200_010), reason: "test")
  let database = root.appending(path: "session-\(record.sessionID.value).sqlite")

  let series = try explainPlan(
    database: database,
    sql: "SELECT sequence, monotonic_elapsed_ms, raw_pack_voltage_v FROM sample WHERE monotonic_elapsed_ms BETWEEN 0 AND 3000 ORDER BY monotonic_elapsed_ms, sequence LIMIT 12"
  )
  #expect(series.contains("sample_elapsed"))
  let keysetSamples = try explainPlan(
    database: database,
    sql: "SELECT sequence, monotonic_elapsed_ms FROM sample WHERE (monotonic_elapsed_ms > 1000 OR (monotonic_elapsed_ms = 1000 AND sequence > 1)) ORDER BY monotonic_elapsed_ms, sequence LIMIT 2000"
  )
  #expect(keysetSamples.contains("sample_elapsed"))
  let keysetEvents = try explainPlan(
    database: database,
    sql: "SELECT sequence, monotonic_elapsed_ms FROM session_event WHERE (monotonic_elapsed_ms > 1000 OR (monotonic_elapsed_ms = 1000 AND sequence > 1)) ORDER BY monotonic_elapsed_ms, sequence LIMIT 500"
  )
  #expect(keysetEvents.contains("session_event_elapsed"))
  let predecessor = try explainPlan(
    database: database,
    sql: "SELECT sequence FROM sample WHERE monotonic_elapsed_ms <= 1500 ORDER BY monotonic_elapsed_ms DESC, sequence DESC LIMIT 1"
  )
  #expect(predecessor.contains("sample_elapsed"))
  let successor = try explainPlan(
    database: database,
    sql: "SELECT sequence FROM sample WHERE monotonic_elapsed_ms >= 1500 ORDER BY monotonic_elapsed_ms ASC, sequence ASC LIMIT 1"
  )
  #expect(successor.contains("sample_elapsed"))
  let exact = try explainPlan(
    database: database,
    sql: "SELECT sequence FROM sample WHERE monotonic_elapsed_ms = 1000 AND sequence = 2 LIMIT 1"
  )
  #expect(exact.contains("sample_elapsed") || exact.contains("INTEGER PRIMARY KEY"))
  let retainedSet = try explainPlan(
    database: database,
    sql: "SELECT sequence FROM sample WHERE sequence IN (1, 2, 3) ORDER BY sequence LIMIT 3"
  )
  #expect(retainedSet.contains("INTEGER PRIMARY KEY") || retainedSet.contains("sample_sequence"))
}

@Test func deterministicLargeSessionBuilderUsesTheLosslessSourcePath() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-large-session-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
  let record = try store.create(deviceID: DeviceID("large-fixture"), at: Date(timeIntervalSince1970: 1_700_300_000))
  var samples: [TelemetrySample] = []
  samples.reserveCapacity(150_000)
  for index in 0 ..< 150_000 {
    let rideState: RideState = index < 149_000 ? .riding : .idle
    let packVoltage = 58 - Double(index % 20) * 0.01
    let speed = Double(index % 80)
    let extensionValue = Double(index % 17)
    samples.append(TelemetrySample(
      sequence: Int64(index + 1),
      observedAt: Date(timeIntervalSince1970: 1_700_300_000 + Double(index)),
      elapsedMs: Int64(index * 1_000),
      rideState: rideState,
      packageState: .running,
      packVoltageV: packVoltage,
      speedKph: speed,
      extensionFields: ["fixtureValue": extensionValue]
    ))
  }
  try store.append(jobID: record.jobID, samples: samples)
  _ = try store.close(
    jobID: record.jobID,
    at: Date(timeIntervalSince1970: 1_700_300_200),
    reason: "large-fixture"
  )
  let source = try store.querySeriesSource(
    sessionID: record.sessionID.value,
    startElapsedMs: 0,
    endElapsedMs: 149_999_000,
    fields: ["packVoltageV", "fixtureValue"],
    maxRows: 200_000,
    publishedSampleSequence: 150_000
  )
  #expect(source.count == 150_000)
  #expect(source.first?.sequence == 1)
  #expect(source.last?.sequence == 150_000)
}

@Test func prunePreviewAndCommitRetainRealProtectedRowsAndAdvanceRevision() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-prune-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root, schemaDirectory: repositoryRoot().appending(path: "sql"), availableBytes: { 2_000_000_000 }
  )
  let startedAt = Date(timeIntervalSince1970: 1_700_400_000)
  let record = try store.create(deviceID: DeviceID("prune-fixture"), at: startedAt)
  var samples: [TelemetrySample] = []
  for index in 0 ..< 24 {
    var extensions: [String: Double] = [:]
    if index >= 12 { extensions["late"] = Double(index) }
    samples.append(TelemetrySample(
      sequence: Int64(index + 1), observedAt: startedAt.addingTimeInterval(Double(index)),
      elapsedMs: Int64(index * 500), rideState: .riding, packageState: index == 5 ? .running : nil,
      packVoltageV: 60 - Double(index) * 0.2, speedKph: 360, extensionFields: extensions
    ))
  }
  try store.append(jobID: record.jobID, samples: samples)
  _ = try store.close(jobID: record.jobID, at: startedAt.addingTimeInterval(20), reason: "prune")
  let preview = try store.prunePreview(sessionID: record.sessionID.value, targetResolutionSeconds: 5)
  #expect(preview.recordedSampleCount == 24)
  #expect(preview.estimatedStoredSampleCount <= 24)
  _ = try store.pruneSession(sessionID: record.sessionID.value, targetResolutionSeconds: 5)
  let snapshot = try store.sessionSnapshot(sessionID: record.sessionID.value)
  #expect(snapshot.historyRevision == 1)
  #expect(snapshot.recordedSampleCount == 24)
  #expect(snapshot.storedSampleCount == preview.estimatedStoredSampleCount)
  #expect(try store.querySeriesSource(
    sessionID: record.sessionID.value, startElapsedMs: 0, endElapsedMs: 20_000,
    fields: ["speedKph"], maxRows: 200_000, publishedSampleSequence: snapshot.committedSampleSequence
  ).map(\.sequence).contains(1))
  #expect(throws: SessionStoreError.invalidRange) {
    try store.prunePreview(sessionID: record.sessionID.value, targetResolutionSeconds: 5)
  }
  let job = SessionMaintenanceJobRecord(
    operationID: "prune-test", kind: "prune", scope: "one", state: "running",
    sessionID: record.sessionID.value, targetResolutionSeconds: 10,
    upperSessionID: record.sessionID.value, lastSessionID: nil,
    completedSessions: 0, skippedSessions: 0, retainedSamples: 0,
    reclaimableBytes: 0, error: nil
  )
  try store.saveMaintenanceJob(job)
  let cancellation = try store.cancelMaintenanceJob(operationID: "prune-test")
  #expect(cancellation?.state == "cancel_requested")
  #expect(try store.maintenanceJob()?.operationID == "prune-test")
}

@Test func settingsExportsAndIdempotentDeletionUseCanonicalStorage() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-session-management-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
  #expect(try store.settings().themePreference == .system)
  #expect(try store.settings().minimumSessionDistance == 100)
  let settings = try store.updateSettings(
    expectedRevision: 0,
    patchJSON: #"{"minimumSessionDistance":80,"idlePersistIntervalMs":2345,"ridingPersistIntervalMs":125,"themePreference":"dark"}"#
  )
  #expect(settings.revision == 1)
  #expect(settings.minimumSessionDistance == 80)
  #expect(settings.themePreference == .dark)
  #expect(throws: ProductStoreError.invalidSettingsPatch) {
    try store.updateSettings(
      expectedRevision: 1,
      patchJSON: #"{"themePreference":"twilight"}"#
    )
  }
  #expect(throws: ProductStoreError.invalidSettingsPatch) {
    try store.updateSettings(
      expectedRevision: 1,
      patchJSON: #"{"rawPacketCaptureEnabled":true}"#
    )
  }
  #expect(throws: ProductStoreError.settingsRevisionConflict(currentRevision: 1)) {
    try store.updateSettings(
      expectedRevision: 0,
      patchJSON: #"{"themePreference":"light"}"#
    )
  }

  let base = Date(timeIntervalSince1970: 1_700_000_000)
  var sessionIDs: [String] = []
  for offset in [0.0, 120.0] {
    let record = try store.create(
      deviceID: DeviceID("management-fixture"),
      at: base.addingTimeInterval(offset)
    )
    sessionIDs.append(record.sessionID.value)
    try store.append(
      jobID: record.jobID,
      samples: [
        TelemetrySample(
          sequence: 1,
          observedAt: base.addingTimeInterval(offset),
          elapsedMs: 0,
          rideState: .idle,
          packageState: .ready,
          setpointAdjustment: SetpointAdjustment.none,
          packVoltageV: 60,
          dutyCycle: 0.1,
          speedKph: 0,
          fieldWeakeningCurrentA: 1.25,
          extensionFields: ["future.energy": 7]
        ),
        TelemetrySample(
          sequence: 2,
          observedAt: base.addingTimeInterval(offset + 60),
          elapsedMs: 60_000,
          rideState: .riding,
          packageState: .running,
          setpointAdjustment: .duty,
          packVoltageV: 58,
          dutyCycle: 0.5,
          speedKph: 18,
          fieldWeakeningCurrentA: 2.5,
          extensionFields: ["future.energy": 6]
        ),
      ]
    )
    _ = try store.close(
      jobID: record.jobID,
      at: base.addingTimeInterval(offset + 60),
      reason: "test"
    )
  }

  let csv = try store.exportSession(sessionID: sessionIDs[0], format: "csv")
  let csvText = try String(contentsOf: csv.fileURL, encoding: .utf8)
  #expect(csvText.contains("pack_voltage_v"))
  #expect(csvText.contains("setpoint_adjustment"))
  #expect(csvText.contains("fieldWeakeningCurrentA"))
  #expect(csvText.contains("future.energy"))
  #expect(!csvText.contains("management-fixture"))
  let archive = try store.exportSession(
    sessionID: sessionIDs[0],
    format: "float-view-archive"
  )
  #expect(
    try sqliteText(
      archive.fileURL,
      "SELECT value FROM float_view_archive_metadata WHERE key = 'format_version'"
    ) == "1"
  )
  let manifestText = try #require(try sqliteText(
    archive.fileURL,
    "SELECT value FROM float_view_archive_metadata WHERE key = 'session'"
  ))
  let manifest = try #require(
    try JSONSerialization.jsonObject(with: Data(manifestText.utf8)) as? [String: Any]
  )
  #expect((manifest["requestedIdleMs"] as? NSNumber)?.int64Value == 2_345)
  #expect((manifest["effectiveRidingMs"] as? NSNumber)?.int64Value == 125)
  #expect(manifest["deviceSnapshot"] is [String: Any])
  #expect(manifest["boardProfileSnapshot"] is [String: Any])
  #expect(try sqliteInt(archive.fileURL, "SELECT COUNT(*) FROM sample") == 2)

  try store.deleteSession(sessionID: sessionIDs[0])
  try store.deleteSession(sessionID: sessionIDs[0])
  #expect(throws: SessionStoreError.sessionNotFound) {
    try store.session(sessionID: sessionIDs[0])
  }
  let active = try store.create(
    deviceID: DeviceID("management-fixture"),
    at: base.addingTimeInterval(300)
  )
  #expect(throws: SessionStoreError.sessionActive) {
    try store.deleteSession(sessionID: active.sessionID.value)
  }
}

@Test func sqliteStoreRecoversAnInterruptedSessionAtStartupBoundary() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-recovery-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let schema = repositoryRoot().appending(path: "sql")
  let interrupted = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  let active = try interrupted.create(deviceID: DeviceID("fixture"), at: .distantPast)
  try interrupted.append(jobID: active.jobID, samples: [
    TelemetrySample(
      sequence: 1, observedAt: .distantPast, elapsedMs: 0,
      rideState: .riding, speedKph: 18
    ),
    TelemetrySample(
      sequence: 2, observedAt: .distantPast, elapsedMs: 20_000,
      rideState: .riding, speedKph: 18
    ),
  ])

  let restarted = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  #expect(try restarted.recoverInterrupted() == 1)
  #expect(try restarted.recoverInterrupted() == 0)
}

@Test func pendingDeletionCanBeCancelledBeforeFileRemoval() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-delete-cancel-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
  let record = try store.create(deviceID: DeviceID("delete-cancel"), at: Date(timeIntervalSince1970: 1_700_300_000))
  try store.append(jobID: record.jobID, samples: [
    TelemetrySample(
      sequence: 1, observedAt: Date(timeIntervalSince1970: 1_700_300_000),
      elapsedMs: 0, rideState: .riding, speedKph: 360
    ),
    TelemetrySample(
      sequence: 2, observedAt: Date(timeIntervalSince1970: 1_700_300_001),
      elapsedMs: 1_000, rideState: .riding, speedKph: 360
    ),
  ])
  _ = try store.close(
    jobID: record.jobID,
    at: Date(timeIntervalSince1970: 1_700_300_002),
    reason: "test"
  )

  try executeSQLite(
    database: root.appending(path: "catalog.sqlite"),
    sql: "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = '\(record.sessionID.value)'"
  )
  #expect(throws: SessionStoreError.sessionNotFound) {
    try store.sessionSnapshot(sessionID: record.sessionID.value)
  }
  #expect(try store.cancelPendingDelete(operationID: record.sessionID.value))
  #expect(try store.sessionSnapshot(sessionID: record.sessionID.value).committedSampleSequence == 2)
}

@Test func commitMarkerProjectionReconcilesDuplicatesAndStaleCatalog() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-marker-projection-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let schema = repositoryRoot().appending(path: "sql")
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  let record = try store.create(deviceID: DeviceID("marker-fixture"), at: .distantPast)
  try store.append(jobID: record.jobID, samples: [
    TelemetrySample(
      sequence: 1, observedAt: .distantPast, elapsedMs: 0,
      rideState: .idle, dutyCycle: 0.2, speedKph: 18
    ),
    TelemetrySample(
      sequence: 2, observedAt: .distantPast, elapsedMs: 20_000,
      rideState: .riding, dutyCycle: 0.5, speedKph: 36
    ),
  ])
  try store.append(jobID: record.jobID, samples: [
    // The duplicate sequence is recorded but must not alter stored rows or extrema.
    TelemetrySample(
      sequence: 2, observedAt: .distantPast, elapsedMs: 20_000,
      rideState: .riding, dutyCycle: 0.01, speedKph: 1
    ),
    TelemetrySample(
      sequence: 3, observedAt: .distantPast, elapsedMs: 40_000,
      rideState: .riding, dutyCycle: 0.3, speedKph: 18
    ),
  ])

  let catalogURL = root.appending(path: "catalog.sqlite")
  try sqliteExecute(
    catalogURL,
    "UPDATE session_catalog SET integrity = 'dirty', sample_count = 0, stored_sample_count = 0, distance_m = 0 WHERE session_id = '\(record.sessionID.value)'"
  )
  let summary = try store.session(sessionID: record.sessionID.value)
  #expect(summary.sampleCount == 4)
  #expect(summary.totalElapsedMs == 40_000)
  #expect(summary.ridingElapsedMs == 40_000)
  #expect(summary.distanceM.map { abs($0 - 300) < 0.001 } == true)
  #expect(summary.maxSpeedMps == 10)
  #expect(summary.maxDutyCycle == 0.5)
  #expect(try sqliteInt(catalogURL, "SELECT stored_sample_count FROM session_catalog") == 3)
  #expect(try sqliteInt(catalogURL, "SELECT max_speed_source_sequence FROM session_catalog") == 2)
  #expect(try sqliteInt(catalogURL, "SELECT max_duty_source_sequence FROM session_catalog") == 2)
}

@Test func sqliteStoreDiscardsInterruptedSessionWithoutRiding() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-idle-recovery-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let schema = repositoryRoot().appending(path: "sql")
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
  #expect(try restarted.recoverInterrupted() == 0)
  #expect(try restarted.sessions(SessionListQuery()).items.isEmpty)
}

@Test func startupResumesAnInterruptedSessionDeletion() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-delete-recovery-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let schema = repositoryRoot().appending(path: "sql")
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  let record = try store.create(deviceID: DeviceID("delete-recovery"), at: .distantPast)
  try store.append(jobID: record.jobID, samples: [
    TelemetrySample(
      sequence: 1, observedAt: .distantPast, elapsedMs: 0,
      rideState: .riding, speedKph: 18
    ),
    TelemetrySample(
      sequence: 2, observedAt: .distantPast, elapsedMs: 20_000,
      rideState: .riding, speedKph: 18
    ),
  ])
  _ = try store.close(jobID: record.jobID, at: .distantPast, reason: "test")
  let catalogURL = root.appending(path: "catalog.sqlite")
  let storedFileName = try sqliteText(
    catalogURL,
    "SELECT file_name FROM session_catalog WHERE session_id = '\(record.sessionID.value)'"
  )
  let fileName = try #require(storedFileName)
  try sqliteExecute(
    catalogURL,
    "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = '\(record.sessionID.value)'"
  )
  try FileManager.default.removeItem(at: root.appending(path: fileName))

  let restarted = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  #expect(try restarted.sessions(SessionListQuery()).items.isEmpty)
  #expect(try sqliteInt(catalogURL, "SELECT COUNT(*) FROM session_catalog") == 0)
}

@Test func outdatedStorageIsDeletedAndRecreated() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-schema-reset-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
  let schema = repositoryRoot().appending(path: "sql")
  let stale = root.appending(path: "old.sqlite")
  try Data("old data".utf8).write(to: stale)
  try Data("outdated".utf8).write(to: root.appending(path: ".schema-version"))

  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  #expect(!FileManager.default.fileExists(atPath: stale.path))
  #expect(try store.sessions(SessionListQuery()).items.isEmpty)
  #expect(
    try String(contentsOf: root.appending(path: ".schema-version"), encoding: .utf8)
      == LogSchema.storageVersion
  )
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
  let schema = repositoryRoot().appending(path: "sql")
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  let session = try store.create(
    deviceID: DeviceID("platform-private-id"),
    advertisedName: "Advertised",
    at: .distantPast
  )
  let saved = try #require(store.savedDevices().first)
  #expect(saved.savedDeviceID != "saved-platform-private-id")
  #expect(!saved.savedDeviceID.contains("platform-private-id"))
  #expect(!saved.profileComplete)
  #expect(saved.displayName == "Advertised")
  #expect(saved.bluetoothIdentifier == "platform-private-id")
  let renamed = try store.updateSavedDevice(
    savedDeviceID: saved.savedDeviceID,
    expectedRevision: 0,
    customDisplayName: "Trail board"
  )
  #expect(renamed.displayName == "Trail board")
  #expect(renamed.revision == 1)
  #expect(throws: ProductStoreError.savedDeviceRevisionConflict(currentRevision: 1)) {
    try store.updateSavedDevice(
      savedDeviceID: saved.savedDeviceID,
      expectedRevision: 0,
      customDisplayName: nil
    )
  }

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
  try store.append(jobID: session.jobID, samples: [
    TelemetrySample(
      sequence: 1, observedAt: .distantPast, elapsedMs: 0,
      rideState: .riding, speedKph: 18
    ),
    TelemetrySample(
      sequence: 2, observedAt: .distantPast, elapsedMs: 20_000,
      rideState: .riding, speedKph: 18
    ),
  ])
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

@Test func controllerPreferenceAndPrivateHintStayAtomicAndPrivate() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-routing-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
  let saved = try store.ensureSavedDevice(
    deviceID: DeviceID("private-platform-id"),
    advertisedName: "Board",
    at: .distantPast
  )
  #expect(store.active() == nil)
  #expect(throws: ProductStoreError.invalidProfilePatch) {
    try store.updateBoardProfile(
      savedDeviceID: saved.savedDeviceID,
      expectedRevision: 0,
      patchJSON: #"{"preferredCanId":254}"#,
      at: .distantPast
    )
  }

  let uuid = Data((1 ... 12).map(UInt8.init))
  try store.storeControllerRoutingHint(
    .init(savedDeviceID: saved.savedDeviceID, canID: 1, controllerUUID: uuid),
    at: .distantPast
  )
  let configured = try store.updateBoardProfile(
    savedDeviceID: saved.savedDeviceID,
    expectedRevision: 0,
    patchJSON: #"{"preferredCanId":42}"#,
    at: .distantPast
  )
  #expect(configured.missingFields == ["seriesCount", "parallelCount", "motor"])
  #expect(try store.controllerRoutingHint(savedDeviceID: saved.savedDeviceID) == nil)

  try store.storeControllerRoutingHint(
    .init(savedDeviceID: saved.savedDeviceID, canID: 42, controllerUUID: uuid),
    at: .distantPast
  )
  let editor = try store.updateBoardProfile(
    savedDeviceID: saved.savedDeviceID,
    expectedRevision: configured.revision,
    patchJSON: #"{"seriesCount":20}"#,
    at: .distantPast
  )
  let selected = try store.selectController(
    savedDeviceID: saved.savedDeviceID,
    canID: 7,
    controllerUUID: uuid,
    at: .distantPast
  )
  #expect(selected.revision == editor.revision + 1)
  #expect(selected.valuesJSON.contains(#""seriesCount":20"#))
  #expect(selected.valuesJSON.contains(#""preferredCanId":7"#))
  #expect(
    try store.controllerRoutingHint(savedDeviceID: saved.savedDeviceID)
      == .init(savedDeviceID: saved.savedDeviceID, canID: 7, controllerUUID: uuid)
  )

  _ = try store.updateBoardProfile(
    savedDeviceID: saved.savedDeviceID,
    expectedRevision: selected.revision,
    patchJSON: #"{"preferredCanId":null}"#,
    at: .distantPast
  )
  #expect(try store.controllerRoutingHint(savedDeviceID: saved.savedDeviceID) == nil)
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

private func sqliteDouble(_ url: URL, _ sql: String) throws -> Double? {
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
  guard sqlite3_step(statement) == SQLITE_ROW,
        sqlite3_column_type(statement, 0) != SQLITE_NULL
  else { return nil }
  return sqlite3_column_double(statement, 0)
}
