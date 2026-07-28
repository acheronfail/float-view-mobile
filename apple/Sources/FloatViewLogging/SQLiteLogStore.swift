#if SWIFT_PACKAGE
import CSQLite
import FloatViewCore
#else
import SQLite3
#endif
import CoreFoundation
import Foundation

public enum SQLiteLogStoreError: Error {
  case sqlite(String)
  case noActiveSession
  case wrongJob
  case storageFull
}

private final class Database: @unchecked Sendable {
  private var handle: OpaquePointer?

  init(url: URL) throws {
    if sqlite3_open_v2(
      url.path,
      &handle,
      SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE | SQLITE_OPEN_FULLMUTEX,
      nil
    ) != SQLITE_OK {
      throw SQLiteLogStoreError.sqlite("Unable to open \(url.lastPathComponent)")
    }
  }

  deinit { sqlite3_close(handle) }

  func execute(_ sql: String) throws {
    var message: UnsafeMutablePointer<CChar>?
    guard sqlite3_exec(handle, sql, nil, nil, &message) == SQLITE_OK else {
      let text = message.map { String(cString: $0) } ?? "SQLite execution failed"
      sqlite3_free(message)
      throw SQLiteLogStoreError.sqlite(text)
    }
  }

  func statement(_ sql: String, values: [SQLiteValue]) throws {
    var statement: OpaquePointer?
    guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    defer { sqlite3_finalize(statement) }
    bind(values, to: statement)
    guard sqlite3_step(statement) == SQLITE_DONE else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
  }

  func query<T>(
    _ sql: String,
    values: [SQLiteValue] = [],
    map: (OpaquePointer) throws -> T
  ) throws -> [T] {
    var statement: OpaquePointer?
    guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK,
          let statement
    else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    defer { sqlite3_finalize(statement) }
    bind(values, to: statement)
    var result: [T] = []
    while true {
      switch sqlite3_step(statement) {
      case SQLITE_ROW: result.append(try map(statement))
      case SQLITE_DONE: return result
      default:
        throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
      }
    }
  }

  func scalarInt(_ sql: String) throws -> Int {
    var statement: OpaquePointer?
    guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    defer { sqlite3_finalize(statement) }
    guard sqlite3_step(statement) == SQLITE_ROW else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    return Int(sqlite3_column_int64(statement, 0))
  }

  func optionalScalarInt(_ sql: String) throws -> Int? {
    var statement: OpaquePointer?
    guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    defer { sqlite3_finalize(statement) }
    guard sqlite3_step(statement) == SQLITE_ROW else { return nil }
    return Int(sqlite3_column_int64(statement, 0))
  }

  private func bind(_ values: [SQLiteValue], to statement: OpaquePointer?) {
    for (index, value) in values.enumerated() {
      let position = Int32(index + 1)
      switch value {
      case .integer(let value): sqlite3_bind_int64(statement, position, value)
      case .real(let value): sqlite3_bind_double(statement, position, value)
      case .text(let value):
        sqlite3_bind_text(statement, position, value, -1, SQLITE_TRANSIENT)
      case .null: sqlite3_bind_null(statement, position)
      }
    }
  }
}

private let SQLITE_TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

private enum SQLiteValue {
  case integer(Int64)
  case real(Double)
  case text(String)
  case null
}

public final class SQLiteLogStore: LogStorePort, ProductStorePort, SessionStorePort, @unchecked Sendable {
  private static let initialMissingFields = ["seriesCount", "parallelCount", "motor"]
  private let root: URL
  private let schemaDirectory: URL
  private let catalog: Database
  private let availableBytes: @Sendable () -> Int64
  private var current: SessionRecord?
  private var session: Database?
  private let lock = NSLock()

  public init(
    root: URL,
    schemaDirectory: URL,
    availableBytes: @escaping @Sendable () -> Int64
  ) throws {
    self.root = root
    self.schemaDirectory = schemaDirectory
    self.availableBytes = availableBytes
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    catalog = try Database(url: root.appending(path: "catalog.sqlite"))
    try catalog.execute(String(contentsOf: schemaDirectory.appending(path: "catalog-v1.sql")))
    if try catalog.scalarInt(
      "SELECT CAST(value AS INTEGER) FROM schema_metadata WHERE key = 'catalog_version'"
    ) < LogSchema.catalogVersion {
      try catalog.execute(String(contentsOf: schemaDirectory.appending(path: "catalog-v2.sql")))
    }
    try backfillV2Summaries()
    try migrateLegacySavedDeviceIdentifiers()
  }

  public func create(deviceID: DeviceID, at: Date) throws -> SessionRecord {
    lock.lock()
    defer { lock.unlock() }
    if let current { return current }
    guard StoragePolicy.level(availableBytes: availableBytes()) != .stop else {
      throw SQLiteLogStoreError.storageFull
    }
    let sessionID = SessionID(UUID().uuidString)
    let jobID = JobID(UUID().uuidString)
    let fileName = "session-\(sessionID.value).sqlite"
    let session = try Database(url: root.appending(path: fileName))
    try session.execute(String(contentsOf: schemaDirectory.appending(path: "session-v1.sql")))
    try session.execute(String(contentsOf: schemaDirectory.appending(path: "session-v2.sql")))
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    let savedDevice = try ensureSavedDeviceLocked(deviceID: deviceID, at: at)
    let profile = try boardProfileLocked(savedDeviceID: savedDevice.savedDeviceID, at: at)
    let deviceSnapshot = try Self.jsonString([
      "savedDeviceId": savedDevice.savedDeviceID,
      "deviceHistoryId": savedDevice.deviceHistoryID,
      "displayName": savedDevice.displayName,
    ])
    try catalog.statement(
      """
      INSERT INTO session_catalog(
        session_id, job_id, device_history_id, device_snapshot_json,
        profile_snapshot_json, profile_revision, file_name, started_at_ms,
        state, integrity, requested_idle_ms, effective_idle_ms,
        requested_riding_ms, effective_riding_ms
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', 'complete', 1000, 1000, 100, 100)
      """,
      values: [
        .text(sessionID.value), .text(jobID.value), .text(savedDevice.deviceHistoryID),
        .text(deviceSnapshot), .text(profile.valuesJSON), .integer(Int64(profile.revision)),
        .text(fileName), .integer(timestamp),
      ]
    )
    self.session = session
    let record = SessionRecord(
      sessionID: sessionID, jobID: jobID, deviceID: deviceID, startedAt: at,
      committedSequence: 0, active: true
    )
    current = record
    return record
  }

  private func migrateLegacySavedDeviceIdentifiers() throws {
    let legacy = try catalog.query(
      """
      SELECT saved_device_id, device_history_id
      FROM saved_device
      WHERE saved_device_id = 'saved-' || opaque_platform_id
         OR device_history_id = 'history-' || opaque_platform_id
      """,
      map: { statement in
        (
          savedDeviceID: Self.columnText(statement, 0)!,
          deviceHistoryID: Self.columnText(statement, 1)!
        )
      }
    )
    guard !legacy.isEmpty else { return }
    try catalog.execute("BEGIN IMMEDIATE")
    do {
      try catalog.execute("PRAGMA defer_foreign_keys = ON")
      for item in legacy {
        let savedDeviceID = UUID().uuidString
        let deviceHistoryID = UUID().uuidString
        let snapshots = try catalog.query(
          """
          SELECT session_id, device_snapshot_json FROM session_catalog
          WHERE device_history_id = ?
          """,
          values: [.text(item.deviceHistoryID)],
          map: { statement in
            (
              sessionID: Self.columnText(statement, 0)!,
              json: Self.columnText(statement, 1)!
            )
          }
        )
        try catalog.statement(
          """
          UPDATE saved_device SET saved_device_id = ?, device_history_id = ?
          WHERE saved_device_id = ?
          """,
          values: [
            .text(savedDeviceID), .text(deviceHistoryID), .text(item.savedDeviceID),
          ]
        )
        try catalog.statement(
          "UPDATE board_profile SET saved_device_id = ? WHERE saved_device_id = ?",
          values: [.text(savedDeviceID), .text(item.savedDeviceID)]
        )
        try catalog.statement(
          "UPDATE session_catalog SET device_history_id = ? WHERE device_history_id = ?",
          values: [.text(deviceHistoryID), .text(item.deviceHistoryID)]
        )
        try catalog.statement(
          "UPDATE session_group SET device_history_id = ? WHERE device_history_id = ?",
          values: [.text(deviceHistoryID), .text(item.deviceHistoryID)]
        )
        for snapshot in snapshots {
          guard var value = try JSONSerialization.jsonObject(
            with: Data(snapshot.json.utf8)
          ) as? [String: Any]
          else { throw ProductStoreError.invalidProfilePatch }
          value["savedDeviceId"] = savedDeviceID
          value["deviceHistoryId"] = deviceHistoryID
          try catalog.statement(
            "UPDATE session_catalog SET device_snapshot_json = ? WHERE session_id = ?",
            values: [.text(try Self.jsonString(value)), .text(snapshot.sessionID)]
          )
        }
      }
      try catalog.execute("COMMIT")
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
  }

  public func append(jobID: JobID, samples: [TelemetrySample]) throws {
    lock.lock()
    defer { lock.unlock() }
    guard var record = current, let session else { throw SQLiteLogStoreError.noActiveSession }
    guard record.jobID == jobID else { throw SQLiteLogStoreError.wrongJob }
    guard StoragePolicy.level(availableBytes: availableBytes()) != .stop else {
      throw SQLiteLogStoreError.storageFull
    }
    guard !samples.isEmpty else { return }
    try session.execute("BEGIN IMMEDIATE")
    do {
      for sample in samples {
        let optional: (Double?) -> SQLiteValue = { $0.map(SQLiteValue.real) ?? .null }
        let extensionData = try JSONSerialization.data(
          withJSONObject: sample.extensionFields,
          options: [.sortedKeys]
        )
        guard let extensionJSON = String(data: extensionData, encoding: .utf8) else {
          throw SQLiteLogStoreError.sqlite("Unable to encode sample extension fields")
        }
        try session.statement(
          """
          INSERT OR IGNORE INTO sample(
            sequence, observed_at_ms, monotonic_elapsed_ms, ride_state,
            raw_pack_voltage_v, duty_cycle, controller_temp_c, motor_temp_c,
            footpad_state, extension_json, motor_current_a, battery_current_a, erpm, speed_kph
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          """,
          values: [
            .integer(sample.sequence),
            .integer(Int64(sample.observedAt.timeIntervalSince1970 * 1_000)),
            .integer(sample.elapsedMs), .text(sample.rideState.rawValue),
            optional(sample.packVoltageV), optional(sample.dutyCycle),
            optional(sample.controllerTempC), optional(sample.motorTempC),
            sample.footpadState.map { .integer(Int64($0)) } ?? .null,
            .text(extensionJSON),
            optional(sample.extensionFields["motorCurrentA"]),
            optional(sample.extensionFields["batteryCurrentA"]),
            optional(sample.extensionFields["erpm"]),
            optional(sample.extensionFields["speedKph"]),
          ]
        )
      }
      let last = samples.map(\.sequence).max()!
      try session.statement(
        "INSERT OR REPLACE INTO commit_marker(singleton, committed_sequence, committed_at_ms) VALUES (1, ?, ?)",
        values: [.integer(last), .integer(Int64(samples.last!.observedAt.timeIntervalSince1970 * 1_000))]
      )
      try session.execute("COMMIT")
      try updateCatalogSummaryLocked(record: record, samples: samples)
      record = SessionRecord(
        sessionID: record.sessionID, jobID: record.jobID, deviceID: record.deviceID,
        startedAt: record.startedAt, committedSequence: max(record.committedSequence, last),
        active: true
      )
      current = record
    } catch {
      try? session.execute("ROLLBACK")
      throw error
    }
  }

  public func close(jobID: JobID, at: Date, reason: String) throws -> SessionRecord {
    lock.lock()
    defer { lock.unlock() }
    guard let record = current else { throw SQLiteLogStoreError.noActiveSession }
    guard record.jobID == jobID else { throw SQLiteLogStoreError.wrongJob }
    try session?.execute("PRAGMA wal_checkpoint(FULL)")
    try catalog.statement(
      """
      UPDATE session_catalog SET ended_at_ms = ?, state = 'completed',
      committed_sequence = ?, close_reason = ? WHERE job_id = ? AND state = 'active'
      """,
      values: [
        .integer(Int64(at.timeIntervalSince1970 * 1_000)), .integer(record.committedSequence),
        .text(reason), .text(jobID.value),
      ]
    )
    session = nil
    current = nil
    return SessionRecord(
      sessionID: record.sessionID, jobID: record.jobID, deviceID: record.deviceID,
      startedAt: record.startedAt, committedSequence: record.committedSequence, active: false
    )
  }

  public func active() -> SessionRecord? {
    lock.withLock { current }
  }

  public func sessions(_ query: SessionListQuery) throws -> SessionPage {
    lock.lock()
    defer { lock.unlock() }
    guard (1 ... 100).contains(query.limit) else { throw SessionStoreError.invalidRange }
    guard let sortColumn = Self.sessionSortColumns[query.sortField] else {
      throw SessionStoreError.invalidRange
    }
    let cursor = try query.cursor.map(Self.decodeCursor)
    if let cursor, cursor.field != query.sortField { throw SessionStoreError.invalidCursor }
    var conditions: [String] = []
    var values: [SQLiteValue] = []
    if let cursor {
      let comparison = query.sortDescending ? "<" : ">"
      conditions.append(
        "(\(sortColumn) \(comparison) ? OR (\(sortColumn) = ? AND session_id \(comparison) ?))"
      )
      values += [.real(cursor.value), .real(cursor.value), .text(cursor.sessionID)]
    }
    if let value = query.deviceHistoryID {
      conditions.append("device_history_id = ?"); values.append(.text(value))
    }
    if let value = query.startedAfter {
      conditions.append("started_at_ms >= ?")
      values.append(.integer(Int64(value.timeIntervalSince1970 * 1_000)))
    }
    if let value = query.startedBefore {
      conditions.append("started_at_ms <= ?")
      values.append(.integer(Int64(value.timeIntervalSince1970 * 1_000)))
    }
    if let value = query.minimumTotalElapsedMs {
      conditions.append("total_elapsed_ms >= ?"); values.append(.integer(value))
    }
    if let value = query.maximumTotalElapsedMs {
      conditions.append("total_elapsed_ms <= ?"); values.append(.integer(value))
    }
    if let value = query.minimumRidingElapsedMs {
      conditions.append("riding_elapsed_ms >= ?"); values.append(.integer(value))
    }
    if let value = query.maximumRidingElapsedMs {
      conditions.append("riding_elapsed_ms <= ?"); values.append(.integer(value))
    }
    if let value = query.minimumDistanceM {
      conditions.append("distance_m >= ?"); values.append(.real(value))
    }
    if let value = query.maximumDistanceM {
      conditions.append("distance_m <= ?"); values.append(.real(value))
    }
    if let value = query.state {
      conditions.append("state = ?"); values.append(.text(value))
    }
    if let value = query.integrity {
      conditions.append("integrity = ?"); values.append(.text(value))
    }
    let whereSQL = conditions.isEmpty ? "" : "WHERE " + conditions.joined(separator: " AND ")
    values.append(.integer(Int64(query.limit + 1)))
    var rows = try catalog.query(
      """
      SELECT session_id, device_history_id, device_snapshot_json, started_at_ms, ended_at_ms,
             state, integrity, sample_count, event_count, riding_elapsed_ms, total_elapsed_ms,
             distance_m, max_speed_mps, max_duty_cycle
      FROM session_catalog \(whereSQL)
      ORDER BY \(sortColumn) \(query.sortDescending ? "DESC" : "ASC"),
               session_id \(query.sortDescending ? "DESC" : "ASC") LIMIT ?
      """,
      values: values,
      map: Self.sessionSummary
    )
    let hasMore = rows.count > query.limit
    if hasMore { rows.removeLast() }
    let next = hasMore ? rows.last.map { Self.encodeCursor($0, field: query.sortField) } : nil
    return SessionPage(items: rows, nextCursor: next)
  }

  public func session(sessionID: String) throws -> SessionSummaryRecord {
    lock.lock()
    defer { lock.unlock() }
    guard let value = try catalog.query(
      """
      SELECT session_id, device_history_id, device_snapshot_json, started_at_ms, ended_at_ms,
             state, integrity, sample_count, event_count, riding_elapsed_ms, total_elapsed_ms,
             distance_m, max_speed_mps, max_duty_cycle
      FROM session_catalog WHERE session_id = ?
      """,
      values: [.text(sessionID)],
      map: Self.sessionSummary
    ).first else { throw SessionStoreError.sessionNotFound }
    return value
  }

  public func querySeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int
  ) throws -> [SessionSeriesPoint] {
    guard startElapsedMs <= endElapsedMs, (1 ... 10_000).contains(maxPoints) else {
      throw SessionStoreError.invalidRange
    }
    return try readSeries(
      sessionID: sessionID, start: startElapsedMs, end: endElapsedMs,
      fields: fields, maxPoints: maxPoints, lossless: false
    )
  }

  public func diagnosticWindow(
    sessionID: String,
    centerElapsedMs: Int64,
    beforeMs: Int64,
    afterMs: Int64,
    fields: [String]
  ) throws -> [SessionSeriesPoint] {
    guard centerElapsedMs >= 0, (0 ... 300_000).contains(beforeMs),
          (0 ... 300_000).contains(afterMs)
    else { throw SessionStoreError.invalidRange }
    return try readSeries(
      sessionID: sessionID, start: max(0, centerElapsedMs - beforeMs),
      end: centerElapsedMs + afterMs, fields: fields, maxPoints: 10_000, lossless: true
    )
  }

  public func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64
  ) throws -> [SessionEventRecord] {
    guard startElapsedMs <= endElapsedMs else { throw SessionStoreError.invalidRange }
    lock.lock()
    defer { lock.unlock() }
    let database = try openSessionLocked(sessionID: sessionID)
    return try database.query(
      """
      SELECT sequence, monotonic_elapsed_ms, kind, details_json FROM session_event
      WHERE monotonic_elapsed_ms BETWEEN ? AND ? ORDER BY monotonic_elapsed_ms, sequence LIMIT 10000
      """,
      values: [.integer(startElapsedMs), .integer(endElapsedMs)]
    ) { statement in
      SessionEventRecord(
        eventID: String(sqlite3_column_int64(statement, 0)),
        elapsedMs: sqlite3_column_int64(statement, 1),
        kind: Self.columnText(statement, 2)!,
        detailsJSON: Self.columnText(statement, 3) ?? "{}"
      )
    }
  }

  public func savedDevices() throws -> [SavedDeviceRecord] {
    lock.lock()
    defer { lock.unlock() }
    return try catalog.query(
      """
      SELECT d.saved_device_id, d.device_history_id, d.display_name,
             d.advertised_name, d.last_seen_at_ms, d.last_connected_at_ms,
             COALESCE(p.json, '{}')
      FROM saved_device d
      LEFT JOIN board_profile p ON p.saved_device_id = d.saved_device_id
      ORDER BY COALESCE(d.last_connected_at_ms, d.created_at_ms) DESC
      """
    ) { statement in
      let valuesJSON = Self.columnText(statement, 6) ?? "{}"
      return SavedDeviceRecord(
        savedDeviceID: Self.columnText(statement, 0)!,
        deviceHistoryID: Self.columnText(statement, 1)!,
        displayName: Self.columnText(statement, 2)!,
        advertisedName: Self.columnText(statement, 3),
        lastSeenAt: Self.columnDate(statement, 4),
        lastConnectedAt: Self.columnDate(statement, 5),
        profileComplete: Self.missingFields(valuesJSON: valuesJSON).isEmpty
      )
    }
  }

  public func boardProfile(savedDeviceID: String) throws -> BoardProfileRecord {
    lock.lock()
    defer { lock.unlock() }
    return try boardProfileLocked(savedDeviceID: savedDeviceID, at: Date())
  }

  public func updateBoardProfile(
    savedDeviceID: String,
    expectedRevision: Int,
    patchJSON: String,
    at: Date
  ) throws -> BoardProfileRecord {
    lock.lock()
    defer { lock.unlock() }
    guard let patch = try? JSONSerialization.jsonObject(with: Data(patchJSON.utf8))
      as? [String: Any]
    else {
      throw ProductStoreError.invalidProfilePatch
    }
    try catalog.execute("BEGIN IMMEDIATE")
    do {
      let currentProfile = try boardProfileLocked(savedDeviceID: savedDeviceID, at: at)
      guard currentProfile.revision == expectedRevision else {
        throw ProductStoreError.profileRevisionConflict(
          currentRevision: currentProfile.revision
        )
      }
      guard var values = try JSONSerialization.jsonObject(
        with: Data(currentProfile.valuesJSON.utf8)
      ) as? [String: Any]
      else {
        throw ProductStoreError.invalidProfilePatch
      }
      patch.forEach { values[$0.key] = $0.value }
      guard Self.isValidInitialProfileValues(values) else {
        throw ProductStoreError.invalidProfilePatch
      }
      let valuesJSON = try Self.jsonString(values)
      let revision = currentProfile.revision + 1
      try catalog.statement(
        """
        UPDATE board_profile SET revision = ?, json = ?, updated_at_ms = ?
        WHERE saved_device_id = ? AND revision = ?
        """,
        values: [
          .integer(Int64(revision)), .text(valuesJSON),
          .integer(Int64(at.timeIntervalSince1970 * 1_000)), .text(savedDeviceID),
          .integer(Int64(expectedRevision)),
        ]
      )
      guard try catalog.scalarInt("SELECT changes()") == 1 else {
        throw ProductStoreError.profileRevisionConflict(
          currentRevision: currentProfile.revision
        )
      }
      try catalog.execute("COMMIT")
      let updated = BoardProfileRecord(
        savedDeviceID: savedDeviceID,
        revision: revision,
        valuesJSON: valuesJSON,
        missingFields: Self.missingFields(valuesJSON: valuesJSON)
      )
      try recordProfileChangeLocked(updated, at: at)
      return updated
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
  }

  @discardableResult
  public func recoverInterrupted() throws -> Int {
    try catalog.execute(
      """
      UPDATE session_catalog SET state = 'interrupted', integrity = 'recovered',
      close_reason = 'process_interruption' WHERE state = 'active'
      """
    )
    return try catalog.scalarInt("SELECT changes()")
  }

  private func updateCatalogSummaryLocked(
    record: SessionRecord,
    samples: [TelemetrySample]
  ) throws {
    let prior = try catalog.query(
      """
      SELECT last_sample_elapsed_ms, last_speed_mps, riding_elapsed_ms, distance_m,
             max_speed_mps, max_duty_cycle, sample_count
      FROM session_catalog WHERE job_id = ?
      """,
      values: [.text(record.jobID.value)]
    ) { statement in
      (
        elapsed: sqlite3_column_type(statement, 0) == SQLITE_NULL
          ? nil : sqlite3_column_int64(statement, 0),
        speed: Self.columnDouble(statement, 1),
        riding: sqlite3_column_int64(statement, 2),
        distance: Self.columnDouble(statement, 3) ?? 0,
        maxSpeed: Self.columnDouble(statement, 4),
        maxDuty: Self.columnDouble(statement, 5),
        count: sqlite3_column_int64(statement, 6)
      )
    }.first
    var previousElapsed = prior?.elapsed
    var previousSpeed = prior?.speed
    var riding = prior?.riding ?? 0
    var distance = prior?.distance ?? 0
    var maxSpeed = prior?.maxSpeed
    var maxDuty = prior?.maxDuty
    for sample in samples.sorted(by: { $0.elapsedMs < $1.elapsedMs }) {
      let speed = sample.extensionFields["speedKph"].map { abs($0) / 3.6 }
      if let previousElapsed {
        let delta = max(0, sample.elapsedMs - previousElapsed)
        if sample.rideState == .riding { riding += delta }
        if let speed {
          distance += ((previousSpeed ?? speed) + speed) * 0.5 * Double(delta) / 1_000
        }
      }
      if let speed { maxSpeed = max(maxSpeed ?? speed, speed) }
      if let duty = sample.dutyCycle.map(abs) { maxDuty = max(maxDuty ?? duty, duty) }
      previousElapsed = sample.elapsedMs
      previousSpeed = speed
    }
    guard let last = samples.max(by: { $0.elapsedMs < $1.elapsedMs }) else { return }
    try catalog.statement(
      """
      UPDATE session_catalog SET committed_sequence = ?, sample_count = ?,
        riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
        max_speed_mps = ?, max_duty_cycle = ?, last_sample_elapsed_ms = ?,
        last_speed_mps = ? WHERE job_id = ?
      """,
      values: [
        .integer(max(record.committedSequence, samples.map(\.sequence).max() ?? 0)),
        .integer((prior?.count ?? 0) + Int64(samples.count)),
        .integer(riding), .integer(last.elapsedMs), .real(distance),
        maxSpeed.map(SQLiteValue.real) ?? .null,
        maxDuty.map(SQLiteValue.real) ?? .null,
        .integer(last.elapsedMs),
        previousSpeed.map(SQLiteValue.real) ?? .null,
        .text(record.jobID.value),
      ]
    )
  }

  private func backfillV2Summaries() throws {
    if try catalog.optionalScalarInt(
      "SELECT CAST(value AS INTEGER) FROM schema_metadata WHERE key = 'summary_backfill_version'"
    ) == 2 {
      return
    }
    let files = try catalog.query(
      "SELECT session_id, file_name FROM session_catalog",
      map: { (sessionID: Self.columnText($0, 0)!, fileName: Self.columnText($0, 1)!) }
    )
    for item in files {
      let database = try Database(url: root.appending(path: item.fileName))
      let version = try database.scalarInt(
        "SELECT CAST(value AS INTEGER) FROM schema_metadata WHERE key = 'session_version'"
      )
      if version < LogSchema.sessionVersion {
        try database.execute(String(contentsOf: schemaDirectory.appending(path: "session-v2.sql")))
      }
      let samples = try database.query(
        """
        SELECT sequence, monotonic_elapsed_ms, ride_state, duty_cycle, extension_json
        FROM sample ORDER BY monotonic_elapsed_ms, sequence
        """
      ) { statement in
        (
          sequence: sqlite3_column_int64(statement, 0),
          elapsed: sqlite3_column_int64(statement, 1),
          riding: Self.columnText(statement, 2) == RideState.riding.rawValue,
          duty: Self.columnDouble(statement, 3),
          extensionJSON: Self.columnText(statement, 4) ?? "{}"
        )
      }
      var previousElapsed: Int64?
      var previousSpeed: Double?
      var ridingElapsed: Int64 = 0
      var distance = 0.0
      var maxSpeed: Double?
      var maxDuty: Double?
      for sample in samples {
        let extensionFields = (try? JSONSerialization.jsonObject(
          with: Data(sample.extensionJSON.utf8)
        )) as? [String: Double]
        let speed = extensionFields?["speedKph"].map { abs($0) / 3.6 }
        if let previousElapsed {
          let delta = max(0, sample.elapsed - previousElapsed)
          if sample.riding { ridingElapsed += delta }
          if let speed {
            distance += ((previousSpeed ?? speed) + speed) * 0.5 * Double(delta) / 1_000
          }
        }
        if let speed { maxSpeed = max(maxSpeed ?? speed, speed) }
        if let duty = sample.duty.map(abs) { maxDuty = max(maxDuty ?? duty, duty) }
        previousElapsed = sample.elapsed
        previousSpeed = speed
      }
      try catalog.statement(
        """
        UPDATE session_catalog SET committed_sequence = ?, sample_count = ?,
          riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
          max_speed_mps = ?, max_duty_cycle = ?, last_sample_elapsed_ms = ?,
          last_speed_mps = ? WHERE session_id = ?
        """,
        values: [
          .integer(samples.last?.sequence ?? 0), .integer(Int64(samples.count)),
          .integer(ridingElapsed), .integer(samples.last?.elapsed ?? 0), .real(distance),
          maxSpeed.map(SQLiteValue.real) ?? .null,
          maxDuty.map(SQLiteValue.real) ?? .null,
          samples.last.map { .integer($0.elapsed) } ?? .null,
          previousSpeed.map(SQLiteValue.real) ?? .null,
          .text(item.sessionID),
        ]
      )
    }
    try catalog.statement(
      "INSERT OR REPLACE INTO schema_metadata(key, value) VALUES ('summary_backfill_version', '2')",
      values: []
    )
  }

  private func openSessionLocked(sessionID: String) throws -> Database {
    guard let fileName = try catalog.query(
      "SELECT file_name FROM session_catalog WHERE session_id = ?",
      values: [.text(sessionID)],
      map: { Self.columnText($0, 0)! }
    ).first else { throw SessionStoreError.sessionNotFound }
    let database = try Database(url: root.appending(path: fileName))
    let version = try database.scalarInt(
      "SELECT CAST(value AS INTEGER) FROM schema_metadata WHERE key = 'session_version'"
    )
    if version < LogSchema.sessionVersion {
      try database.execute(String(contentsOf: schemaDirectory.appending(path: "session-v2.sql")))
    }
    return database
  }

  private func readSeries(
    sessionID: String,
    start: Int64,
    end: Int64,
    fields: [String],
    maxPoints: Int,
    lossless: Bool
  ) throws -> [SessionSeriesPoint] {
    let uniqueFields = Array(NSOrderedSet(array: fields)) as? [String] ?? fields
    guard !uniqueFields.isEmpty else { throw SessionStoreError.invalidRange }
    let columns = try uniqueFields.map { field -> String in
      guard let column = Self.seriesColumns[field] else {
        throw SessionStoreError.unsupportedField(field)
      }
      return column
    }
    lock.lock()
    defer { lock.unlock() }
    let database = try openSessionLocked(sessionID: sessionID)
    let selected = columns.enumerated().map { index, column in
      lossless ? "\(column) AS value_\(index)" : "AVG(\(column)) AS value_\(index)"
    }.joined(separator: ", ")
    let sql: String
    let values: [SQLiteValue]
    if lossless {
      sql = """
      SELECT monotonic_elapsed_ms, \(selected) FROM sample
      WHERE monotonic_elapsed_ms BETWEEN ? AND ?
      ORDER BY monotonic_elapsed_ms, sequence LIMIT ?
      """
      values = [.integer(start), .integer(end), .integer(Int64(maxPoints))]
    } else {
      let span = max(1, end - start + 1)
      sql = """
      SELECT MIN(monotonic_elapsed_ms), \(selected) FROM sample
      WHERE monotonic_elapsed_ms BETWEEN ? AND ?
      GROUP BY ((monotonic_elapsed_ms - ?) * ? / ?)
      ORDER BY MIN(monotonic_elapsed_ms) LIMIT ?
      """
      values = [
        .integer(start), .integer(end), .integer(start),
        .integer(Int64(maxPoints)), .integer(span), .integer(Int64(maxPoints)),
      ]
    }
    return try database.query(sql, values: values) { statement in
      var result: [String: Double?] = [:]
      for (index, field) in uniqueFields.enumerated() {
        result[field] = .some(Self.columnDouble(statement, Int32(index + 1)))
      }
      return SessionSeriesPoint(
        elapsedMs: sqlite3_column_int64(statement, 0),
        values: result
      )
    }
  }

  private static let seriesColumns = [
    "packVoltageV": "raw_pack_voltage_v",
    "dutyCycle": "duty_cycle",
    "controllerTempC": "controller_temp_c",
    "motorTempC": "motor_temp_c",
    "footpadState": "footpad_state",
    "motorCurrentA": "motor_current_a",
    "batteryCurrentA": "battery_current_a",
    "erpm": "erpm",
    "speedKph": "speed_kph",
  ]

  private static let sessionSortColumns = [
    "startedAt": "started_at_ms",
    "totalElapsedMs": "total_elapsed_ms",
    "ridingElapsedMs": "riding_elapsed_ms",
    "distanceM": "COALESCE(distance_m, -1)",
  ]

  private static func sessionSummary(_ statement: OpaquePointer) throws -> SessionSummaryRecord {
    let snapshot = columnText(statement, 2).flatMap { text in
      (try? JSONSerialization.jsonObject(with: Data(text.utf8))) as? [String: Any]
    }
    return SessionSummaryRecord(
      sessionID: columnText(statement, 0)!,
      deviceHistoryID: columnText(statement, 1)!,
      displayName: snapshot?["displayName"] as? String ?? "Board",
      startedAt: Date(timeIntervalSince1970: Double(sqlite3_column_int64(statement, 3)) / 1_000),
      endedAt: columnDate(statement, 4),
      state: columnText(statement, 5)!,
      integrity: columnText(statement, 6)!,
      sampleCount: sqlite3_column_int64(statement, 7),
      eventCount: sqlite3_column_int64(statement, 8),
      ridingElapsedMs: sqlite3_column_int64(statement, 9),
      totalElapsedMs: sqlite3_column_int64(statement, 10),
      distanceM: columnDouble(statement, 11),
      maxSpeedMps: columnDouble(statement, 12),
      maxDutyCycle: columnDouble(statement, 13)
    )
  }

  private static func encodeCursor(_ value: SessionSummaryRecord, field: String) -> String {
    let number: Double
    switch field {
    case "totalElapsedMs": number = Double(value.totalElapsedMs)
    case "ridingElapsedMs": number = Double(value.ridingElapsedMs)
    case "distanceM": number = value.distanceM ?? -1
    default: number = value.startedAt.timeIntervalSince1970 * 1_000
    }
    let raw = "\(field):\(number):\(value.sessionID)"
    return Data(raw.utf8).base64EncodedString()
  }

  private static func decodeCursor(
    _ value: String
  ) throws -> (field: String, value: Double, sessionID: String) {
    guard let data = Data(base64Encoded: value),
          let raw = String(data: data, encoding: .utf8),
          let first = raw.firstIndex(of: ":"),
          let second = raw[raw.index(after: first)...].firstIndex(of: ":"),
          let number = Double(raw[raw.index(after: first)..<second])
    else { throw SessionStoreError.invalidCursor }
    let field = String(raw[..<first])
    let sessionID = String(raw[raw.index(after: second)...])
    guard sessionSortColumns[field] != nil, !sessionID.isEmpty else {
      throw SessionStoreError.invalidCursor
    }
    return (field, number, sessionID)
  }

  private func ensureSavedDeviceLocked(deviceID: DeviceID, at: Date) throws -> SavedDeviceRecord {
    let existing = try catalog.query(
      """
      SELECT saved_device_id, device_history_id, display_name, advertised_name,
             last_seen_at_ms, last_connected_at_ms
      FROM saved_device WHERE opaque_platform_id = ? LIMIT 1
      """,
      values: [.text(deviceID.value)]
    ) { statement in
      SavedDeviceRecord(
        savedDeviceID: Self.columnText(statement, 0)!,
        deviceHistoryID: Self.columnText(statement, 1)!,
        displayName: Self.columnText(statement, 2)!,
        advertisedName: Self.columnText(statement, 3),
        lastSeenAt: Self.columnDate(statement, 4),
        lastConnectedAt: Self.columnDate(statement, 5),
        profileComplete: false
      )
    }.first
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    if let existing {
      try catalog.statement(
        "UPDATE saved_device SET last_connected_at_ms = ? WHERE saved_device_id = ?",
        values: [.integer(timestamp), .text(existing.savedDeviceID)]
      )
      _ = try boardProfileLocked(savedDeviceID: existing.savedDeviceID, at: at)
      return SavedDeviceRecord(
        savedDeviceID: existing.savedDeviceID,
        deviceHistoryID: existing.deviceHistoryID,
        displayName: existing.displayName,
        advertisedName: existing.advertisedName,
        lastSeenAt: existing.lastSeenAt,
        lastConnectedAt: at,
        profileComplete: false
      )
    }
    let savedDeviceID = UUID().uuidString
    let deviceHistoryID = UUID().uuidString
    try catalog.statement(
      """
      INSERT INTO saved_device(
        saved_device_id, device_history_id, opaque_platform_id, display_name,
        created_at_ms, last_connected_at_ms
      ) VALUES (?, ?, ?, 'Board', ?, ?)
      """,
      values: [
        .text(savedDeviceID), .text(deviceHistoryID), .text(deviceID.value),
        .integer(timestamp), .integer(timestamp),
      ]
    )
    _ = try boardProfileLocked(savedDeviceID: savedDeviceID, at: at)
    return SavedDeviceRecord(
      savedDeviceID: savedDeviceID,
      deviceHistoryID: deviceHistoryID,
      displayName: "Board",
      lastConnectedAt: at,
      profileComplete: false
    )
  }

  private func boardProfileLocked(savedDeviceID: String, at: Date) throws -> BoardProfileRecord {
    let savedExists = try catalog.query(
      "SELECT 1 FROM saved_device WHERE saved_device_id = ? LIMIT 1",
      values: [.text(savedDeviceID)]
    ) { _ in true }.first ?? false
    guard savedExists else { throw ProductStoreError.savedDeviceNotFound }
    try catalog.statement(
      """
      INSERT OR IGNORE INTO board_profile(saved_device_id, revision, json, updated_at_ms)
      VALUES (?, 0, '{}', ?)
      """,
      values: [.text(savedDeviceID), .integer(Int64(at.timeIntervalSince1970 * 1_000))]
    )
    guard let profile = try catalog.query(
      "SELECT revision, json FROM board_profile WHERE saved_device_id = ?",
      values: [.text(savedDeviceID)],
      map: { statement in
        let valuesJSON = Self.columnText(statement, 1) ?? "{}"
        return BoardProfileRecord(
          savedDeviceID: savedDeviceID,
          revision: Int(sqlite3_column_int64(statement, 0)),
          valuesJSON: valuesJSON,
          missingFields: Self.missingFields(valuesJSON: valuesJSON)
        )
      }
    ).first else {
      throw ProductStoreError.savedDeviceNotFound
    }
    return profile
  }

  private func recordProfileChangeLocked(_ profile: BoardProfileRecord, at: Date) throws {
    guard let current, let session else { return }
    let observedAtMs = Int64(at.timeIntervalSince1970 * 1_000)
    let elapsedMs = max(0, observedAtMs - Int64(current.startedAt.timeIntervalSince1970 * 1_000))
    let details = try Self.jsonString([
      "savedDeviceId": profile.savedDeviceID,
      "revision": profile.revision,
      "values": try JSONSerialization.jsonObject(with: Data(profile.valuesJSON.utf8)),
    ])
    try session.statement(
      """
      INSERT OR REPLACE INTO session_event(
        sequence, observed_at_ms, monotonic_elapsed_ms, kind, details_json
      ) VALUES (?, ?, ?, 'board_profile_changed', ?)
      """,
      values: [
        .integer(Int64(profile.revision)), .integer(observedAtMs),
        .integer(elapsedMs), .text(details),
      ]
    )
    try catalog.statement(
      """
      UPDATE session_catalog SET event_count = event_count + 1
      WHERE job_id = ? AND state = 'active'
      """,
      values: [.text(current.jobID.value)]
    )
  }

  private static func missingFields(valuesJSON: String) -> [String] {
    guard let values = try? JSONSerialization.jsonObject(with: Data(valuesJSON.utf8))
      as? [String: Any]
    else {
      return initialMissingFields
    }
    return initialMissingFields.filter { values[$0] == nil || values[$0] is NSNull }
  }

  private static func isValidInitialProfileValues(_ values: [String: Any]) -> Bool {
    func integer(_ key: String, range: ClosedRange<Int>) -> Bool {
      guard let raw = values[key], !(raw is NSNull) else { return true }
      guard let number = raw as? NSNumber,
            CFGetTypeID(number) != CFBooleanGetTypeID(),
            number.doubleValue.rounded() == number.doubleValue
      else { return false }
      return range.contains(number.intValue)
    }
    guard integer("seriesCount", range: 1 ... 36),
          integer("parallelCount", range: 1 ... 20)
    else { return false }
    guard let motor = values["motor"], !(motor is NSNull) else { return true }
    return (motor as? String).map { !$0.isEmpty && $0.utf8.count <= 128 } ?? false
  }

  private static func jsonString(_ object: Any) throws -> String {
    let data = try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])
    guard data.count <= 65_536, let result = String(data: data, encoding: .utf8) else {
      throw ProductStoreError.invalidProfilePatch
    }
    return result
  }

  private static func columnText(_ statement: OpaquePointer, _ index: Int32) -> String? {
    guard sqlite3_column_type(statement, index) != SQLITE_NULL,
          let text = sqlite3_column_text(statement, index)
    else { return nil }
    return String(cString: text)
  }

  private static func columnDate(_ statement: OpaquePointer, _ index: Int32) -> Date? {
    guard sqlite3_column_type(statement, index) != SQLITE_NULL else { return nil }
    return Date(timeIntervalSince1970: Double(sqlite3_column_int64(statement, index)) / 1_000)
  }

  private static func columnDouble(_ statement: OpaquePointer, _ index: Int32) -> Double? {
    guard sqlite3_column_type(statement, index) != SQLITE_NULL else { return nil }
    return sqlite3_column_double(statement, index)
  }
}
