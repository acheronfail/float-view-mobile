#if SWIFT_PACKAGE
import CSQLite
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

  func forEachRow(
    _ sql: String,
    values: [SQLiteValue] = [],
    body: (OpaquePointer) throws -> Void
  ) throws {
    var statement: OpaquePointer?
    guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK,
          let statement
    else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    defer { sqlite3_finalize(statement) }
    bind(values, to: statement)
    while true {
      switch sqlite3_step(statement) {
      case SQLITE_ROW: try body(statement)
      case SQLITE_DONE: return
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
    try Self.prepareStorageRoot(root)
    #if os(iOS)
    // Sessions must remain writable after the user has unlocked once so native
    // background BLE logging does not depend on the WebView or an unlocked UI.
    // Newly created SQLite, WAL, and SHM files inherit this directory class.
    try FileManager.default.setAttributes(
      [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
      ofItemAtPath: root.path
    )
    #endif
    catalog = try Database(url: root.appending(path: "catalog.sqlite"))
    try catalog.execute(String(contentsOf: schemaDirectory.appending(path: "catalog.sql")))
    try resumePendingDeletes()
    try cleanupStagedExports()
  }

  private static func prepareStorageRoot(_ root: URL) throws {
    let fileManager = FileManager.default
    let marker = root.appending(path: ".schema-version")
    let storedVersion = try? String(contentsOf: marker, encoding: .utf8)
      .trimmingCharacters(in: .whitespacesAndNewlines)
    if fileManager.fileExists(atPath: root.path), storedVersion != LogSchema.storageVersion {
      try fileManager.removeItem(at: root)
    }
    try fileManager.createDirectory(at: root, withIntermediateDirectories: true)
    try Data(LogSchema.storageVersion.utf8).write(to: marker, options: .atomic)
  }

  public func create(
    deviceID: DeviceID,
    advertisedName: String?,
    at: Date
  ) throws -> SessionRecord {
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
    try session.execute(String(contentsOf: schemaDirectory.appending(path: "session.sql")))
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    let savedDevice = try ensureSavedDeviceLocked(
      deviceID: deviceID,
      advertisedName: advertisedName,
      at: at
    )
    let profile = try boardProfileLocked(savedDeviceID: savedDevice.savedDeviceID, at: at)
    let settings = try settingsLocked()
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
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', 'complete', ?, ?, ?, ?)
      """,
      values: [
        .text(sessionID.value), .text(jobID.value), .text(savedDevice.deviceHistoryID),
        .text(deviceSnapshot), .text(profile.valuesJSON), .integer(Int64(profile.revision)),
        .text(fileName), .integer(timestamp),
        .integer(settings.idlePersistIntervalMs), .integer(settings.idlePersistIntervalMs),
        .integer(settings.ridingPersistIntervalMs), .integer(settings.ridingPersistIntervalMs),
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

  public func ensureSavedDevice(
    deviceID: DeviceID,
    advertisedName: String?,
    at: Date
  ) throws -> SavedDeviceRecord {
    lock.lock()
    defer { lock.unlock() }
    return try ensureSavedDeviceLocked(
      deviceID: deviceID,
      advertisedName: advertisedName,
      at: at
    )
  }

  public func create(deviceID: DeviceID, at: Date) throws -> SessionRecord {
    try create(deviceID: deviceID, advertisedName: nil, at: at)
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
            board_state, raw_pack_voltage_v, duty_cycle, controller_temp_c, motor_temp_c,
            footpad_state, extension_json, motor_current_a, battery_current_a, erpm, speed_kph
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          """,
          values: [
            .integer(sample.sequence),
            .integer(Int64(sample.observedAt.timeIntervalSince1970 * 1_000)),
            .integer(sample.elapsedMs), .text(sample.rideState.rawValue),
            sample.boardState.map { .integer(Int64($0)) } ?? .null,
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
    try catalog.execute("BEGIN IMMEDIATE")
    do {
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
      try linkClosedSessionLocked(sessionID: record.sessionID.value)
      try catalog.execute("COMMIT")
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
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
    var conditions: [String] = ["pending_delete = 0"]
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
      FROM session_catalog WHERE session_id = ? AND pending_delete = 0
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
    return try savedDevicesLocked()
  }

  private func savedDevicesLocked() throws -> [SavedDeviceRecord] {
    return try catalog.query(
      """
      SELECT d.saved_device_id, d.device_history_id, d.revision,
             d.custom_display_name, d.advertised_name, d.opaque_platform_id,
             d.last_seen_at_ms, d.last_connected_at_ms,
             COALESCE(p.json, '{}')
      FROM saved_device d
      LEFT JOIN board_profile p ON p.saved_device_id = d.saved_device_id
      ORDER BY COALESCE(d.last_connected_at_ms, d.created_at_ms) DESC
      """
    ) { statement in
      let valuesJSON = Self.columnText(statement, 8) ?? "{}"
      return SavedDeviceRecord(
        savedDeviceID: Self.columnText(statement, 0)!,
        deviceHistoryID: Self.columnText(statement, 1)!,
        revision: Int(sqlite3_column_int64(statement, 2)),
        customDisplayName: Self.columnText(statement, 3),
        advertisedName: Self.columnText(statement, 4),
        bluetoothIdentifier: Self.columnText(statement, 5)!,
        lastSeenAt: Self.columnDate(statement, 6),
        lastConnectedAt: Self.columnDate(statement, 7),
        profileComplete: Self.missingFields(valuesJSON: valuesJSON).isEmpty
      )
    }
  }

  public func updateSavedDevice(
    savedDeviceID: String,
    expectedRevision: Int,
    customDisplayName: String?
  ) throws -> SavedDeviceRecord {
    lock.lock()
    defer { lock.unlock() }
    let normalized = customDisplayName?.trimmingCharacters(in: .whitespacesAndNewlines)
    guard normalized?.utf8.count ?? 0 <= 128 else {
      throw ProductStoreError.invalidSavedDevicePatch
    }
    let value = normalized.flatMap { $0.isEmpty ? nil : $0 }
    try catalog.statement(
      """
      UPDATE saved_device SET custom_display_name = ?, revision = revision + 1
      WHERE saved_device_id = ? AND revision = ?
      """,
      values: [
        value.map(SQLiteValue.text) ?? .null,
        .text(savedDeviceID),
        .integer(Int64(expectedRevision)),
      ]
    )
    guard try catalog.scalarInt("SELECT changes()") == 1 else {
      guard let current = try savedDevicesLocked().first(where: {
        $0.savedDeviceID == savedDeviceID
      }) else {
        throw ProductStoreError.savedDeviceNotFound
      }
      throw ProductStoreError.savedDeviceRevisionConflict(currentRevision: current.revision)
    }
    guard let updated = try savedDevicesLocked().first(where: {
      $0.savedDeviceID == savedDeviceID
    }) else {
      throw ProductStoreError.savedDeviceNotFound
    }
    return updated
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
      let oldPreferredCanID = Self.preferredCanID(values)
      patch.forEach { values[$0.key] = $0.value }
      guard Self.isValidInitialProfileValues(values) else {
        throw ProductStoreError.invalidProfilePatch
      }
      let newPreferredCanID = Self.preferredCanID(values)
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
      if patch.keys.contains("preferredCanId"),
         oldPreferredCanID != newPreferredCanID || newPreferredCanID == nil
      {
        try clearControllerRoutingHintLocked(savedDeviceID: savedDeviceID)
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

  public func selectController(
    savedDeviceID: String,
    canID: UInt8,
    controllerUUID: Data?,
    at: Date
  ) throws -> BoardProfileRecord {
    precondition(canID <= 253)
    guard controllerUUID == nil || controllerUUID?.count == 12 else {
      throw ProductStoreError.invalidProfilePatch
    }
    lock.lock()
    defer { lock.unlock() }
    try catalog.execute("BEGIN IMMEDIATE")
    do {
      let currentProfile = try boardProfileLocked(savedDeviceID: savedDeviceID, at: at)
      guard var values = try JSONSerialization.jsonObject(
        with: Data(currentProfile.valuesJSON.utf8)
      ) as? [String: Any]
      else { throw ProductStoreError.invalidProfilePatch }
      values["preferredCanId"] = Int(canID)
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
          .integer(Int64(currentProfile.revision)),
        ]
      )
      guard try catalog.scalarInt("SELECT changes()") == 1 else {
        throw ProductStoreError.profileRevisionConflict(
          currentRevision: currentProfile.revision
        )
      }
      try storeControllerRoutingHintLocked(
        ControllerRoutingHint(
          savedDeviceID: savedDeviceID,
          canID: canID,
          controllerUUID: controllerUUID
        ),
        at: at
      )
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

  public func controllerRoutingHint(
    savedDeviceID: String
  ) throws -> ControllerRoutingHint? {
    lock.lock()
    defer { lock.unlock() }
    return try controllerRoutingHintLocked(savedDeviceID: savedDeviceID)
  }

  public func storeControllerRoutingHint(
    _ hint: ControllerRoutingHint,
    at: Date
  ) throws {
    guard hint.canID <= 253,
          hint.controllerUUID == nil || hint.controllerUUID?.count == 12
    else { throw ProductStoreError.invalidProfilePatch }
    lock.lock()
    defer { lock.unlock() }
    try storeControllerRoutingHintLocked(hint, at: at)
  }

  public func clearControllerRoutingHint(savedDeviceID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    try clearControllerRoutingHintLocked(savedDeviceID: savedDeviceID)
  }

  public func settings() throws -> SettingsRecord {
    lock.lock()
    defer { lock.unlock() }
    return try settingsLocked()
  }

  public func updateSettings(
    expectedRevision: Int,
    patchJSON: String
  ) throws -> SettingsRecord {
    lock.lock()
    defer { lock.unlock() }
    guard let patch = try? JSONSerialization.jsonObject(with: Data(patchJSON.utf8))
      as? [String: Any],
      !patch.isEmpty,
      patch.keys.allSatisfy(Self.settingKeys.contains)
    else {
      throw ProductStoreError.invalidSettingsPatch
    }
    try catalog.execute("BEGIN IMMEDIATE")
    do {
      let current = try settingsLocked()
      guard current.revision == expectedRevision else {
        throw ProductStoreError.settingsRevisionConflict(currentRevision: current.revision)
      }
      var values = Self.settingsDictionary(current)
      patch.forEach { values[$0.key] = $0.value }
      let updated = try Self.validatedSettings(values, revision: current.revision + 1)
      let json = String(data: try JSONEncoder().encode(updated), encoding: .utf8)!
      try catalog.statement(
        "UPDATE settings SET revision = ?, json = ? WHERE singleton = 1 AND revision = ?",
        values: [
          .integer(Int64(updated.revision)), .text(json), .integer(Int64(expectedRevision)),
        ]
      )
      guard try catalog.scalarInt("SELECT changes()") == 1 else {
        throw ProductStoreError.settingsRevisionConflict(currentRevision: current.revision)
      }
      try catalog.execute("COMMIT")
      return updated
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
  }

  public func deleteSession(sessionID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    if current?.sessionID.value == sessionID { throw SessionStoreError.sessionActive }
    guard let sessionRow = try catalog.query(
      "SELECT file_name, state FROM session_catalog WHERE session_id = ?",
      values: [.text(sessionID)],
      map: { (Self.columnText($0, 0)!, Self.columnText($0, 1)!) }
    ).first else {
      return
    }
    if sessionRow.1 == "active" { throw SessionStoreError.sessionActive }
    let fileName = sessionRow.0
    try catalog.statement(
      "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = ?",
      values: [.text(sessionID)]
    )
    try removeSessionFiles(fileName: fileName)
    try catalog.execute("BEGIN IMMEDIATE")
    do {
      try catalog.statement(
        "DELETE FROM session_catalog WHERE session_id = ?",
        values: [.text(sessionID)]
      )
      try removeUndersizedGroupsLocked()
      try catalog.execute("COMMIT")
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
  }

  public func sessionGroups() throws -> [SessionGroupRecord] {
    lock.lock()
    defer { lock.unlock() }
    let ids = try catalog.query(
      "SELECT session_group_id FROM session_group ORDER BY created_at_ms DESC LIMIT 1000",
      map: { Self.columnText($0, 0)! }
    )
    return try ids.map(sessionGroupLocked)
  }

  public func sessionGroup(sessionGroupID: String) throws -> SessionGroupRecord {
    lock.lock()
    defer { lock.unlock() }
    return try sessionGroupLocked(sessionGroupID)
  }

  public func unlinkSessionGroup(sessionGroupID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    try catalog.statement(
      "DELETE FROM session_group WHERE session_group_id = ?",
      values: [.text(sessionGroupID)]
    )
  }

  public func exportSession(
    sessionID: String,
    format: String
  ) throws -> SessionExportRecord {
    lock.lock()
    defer { lock.unlock() }
    guard format == "float-view-archive" || format == "csv" else {
      throw SessionStoreError.invalidExportFormat
    }
    if current?.sessionID.value == sessionID { throw SessionStoreError.sessionActive }
    guard let sessionRow = try catalog.query(
      """
      SELECT file_name, state FROM session_catalog
      WHERE session_id = ? AND pending_delete = 0
      """,
      values: [.text(sessionID)],
      map: { (Self.columnText($0, 0)!, Self.columnText($0, 1)!) }
    ).first else {
      throw SessionStoreError.sessionNotFound
    }
    if sessionRow.1 == "active" { throw SessionStoreError.sessionActive }
    let fileName = sessionRow.0
    let exports = root.appending(path: "Exports", directoryHint: .isDirectory)
    try FileManager.default.createDirectory(at: exports, withIntermediateDirectories: true)
    let exportID = UUID().uuidString
    let outputName = "float-view-\(exportID).\(format == "csv" ? "csv" : "floatview")"
    let output = exports.appending(path: outputName)
    do {
      let source = root.appending(path: fileName)
      let database = try openSessionLocked(sessionID: sessionID)
      try database.execute("PRAGMA wal_checkpoint(FULL)")
      if format == "float-view-archive" {
        try FileManager.default.copyItem(at: source, to: output)
        let archive = try Database(url: output)
        try archive.execute(
          """
          CREATE TABLE IF NOT EXISTS float_view_archive_metadata(
            key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL
          )
          """
        )
        let manifest = try sessionArchiveManifest(sessionID: sessionID)
        try archive.statement(
          "INSERT OR REPLACE INTO float_view_archive_metadata(key, value) VALUES ('format_version', '1')",
          values: []
        )
        try archive.statement(
          "INSERT OR REPLACE INTO float_view_archive_metadata(key, value) VALUES ('session', ?)",
          values: [.text(try Self.jsonString(manifest))]
        )
        try archive.execute("PRAGMA wal_checkpoint(TRUNCATE)")
      } else {
        FileManager.default.createFile(atPath: output.path, contents: nil)
        let handle = try FileHandle(forWritingTo: output)
        defer { try? handle.close() }
        try handle.write(contentsOf: Data(
          "sequence,observed_at_ms,elapsed_ms,ride_state,board_state,pack_voltage_v,duty_cycle,controller_temp_c,motor_temp_c,footpad_state,motor_current_a,battery_current_a,erpm,speed_kph,extension_json\n".utf8
        ))
        try database.forEachRow(
          """
          SELECT sequence, observed_at_ms, monotonic_elapsed_ms, ride_state,
                 board_state, raw_pack_voltage_v, duty_cycle, controller_temp_c, motor_temp_c,
                 footpad_state, motor_current_a, battery_current_a, erpm, speed_kph,
                 extension_json
          FROM sample ORDER BY sequence
          """
        ) { statement in
          var columns: [String] = []
          for index in 0 ..< 15 {
            if sqlite3_column_type(statement, Int32(index)) == SQLITE_NULL {
              columns.append("")
            } else if index == 3 || index == 14 {
              columns.append(Self.csv(Self.columnText(statement, Int32(index)) ?? ""))
            } else {
              columns.append(String(cString: sqlite3_column_text(statement, Int32(index))))
            }
          }
          try handle.write(contentsOf: Data((columns.joined(separator: ",") + "\n").utf8))
        }
      }
      return SessionExportRecord(exportID: exportID, fileName: outputName, fileURL: output)
    } catch {
      try? FileManager.default.removeItem(at: output)
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

  private func settingsLocked() throws -> SettingsRecord {
    if let json = try catalog.query(
      "SELECT json FROM settings WHERE singleton = 1",
      map: { Self.columnText($0, 0)! }
    ).first,
      let value = try? JSONDecoder().decode(SettingsRecord.self, from: Data(json.utf8))
    {
      return value
    }
    let value = SettingsRecord()
    let json = String(data: try JSONEncoder().encode(value), encoding: .utf8)!
    try catalog.statement(
      "INSERT OR REPLACE INTO settings(singleton, revision, json) VALUES (1, 0, ?)",
      values: [.text(json)]
    )
    return value
  }

  private func sessionArchiveManifest(sessionID: String) throws -> [String: Any] {
    guard let value = try catalog.query(
      """
      SELECT session_id, device_history_id, device_snapshot_json, profile_snapshot_json,
             profile_revision, started_at_ms, ended_at_ms, state, integrity,
             committed_sequence, sample_count, event_count, requested_idle_ms,
             effective_idle_ms, requested_riding_ms, effective_riding_ms, close_reason,
             riding_elapsed_ms, total_elapsed_ms, distance_m, max_speed_mps, max_duty_cycle
      FROM session_catalog WHERE session_id = ? AND pending_delete = 0
      """,
      values: [.text(sessionID)],
      map: { statement -> [String: Any] in
        func json(_ index: Int32) -> Any {
          guard let text = Self.columnText(statement, index),
                let object = try? JSONSerialization.jsonObject(with: Data(text.utf8))
          else { return [:] }
          return object
        }
        func integer(_ index: Int32) -> Any {
          sqlite3_column_type(statement, index) == SQLITE_NULL
            ? NSNull() : sqlite3_column_int64(statement, index)
        }
        func number(_ index: Int32) -> Any {
          sqlite3_column_type(statement, index) == SQLITE_NULL
            ? NSNull() : sqlite3_column_double(statement, index)
        }
        return [
          "sessionId": Self.columnText(statement, 0)!,
          "deviceHistoryId": Self.columnText(statement, 1)!,
          "deviceSnapshot": json(2),
          "boardProfileSnapshot": json(3),
          "boardProfileRevision": sqlite3_column_int64(statement, 4),
          "startedAtMs": sqlite3_column_int64(statement, 5),
          "endedAtMs": integer(6),
          "state": Self.columnText(statement, 7)!,
          "integrity": Self.columnText(statement, 8)!,
          "committedSequence": sqlite3_column_int64(statement, 9),
          "sampleCount": sqlite3_column_int64(statement, 10),
          "eventCount": sqlite3_column_int64(statement, 11),
          "requestedIdleMs": sqlite3_column_int64(statement, 12),
          "effectiveIdleMs": sqlite3_column_int64(statement, 13),
          "requestedRidingMs": sqlite3_column_int64(statement, 14),
          "effectiveRidingMs": sqlite3_column_int64(statement, 15),
          "closeReason": Self.columnText(statement, 16) ?? NSNull(),
          "ridingElapsedMs": sqlite3_column_int64(statement, 17),
          "totalElapsedMs": sqlite3_column_int64(statement, 18),
          "distanceM": number(19),
          "maxSpeedMps": number(20),
          "maxDutyCycle": number(21),
        ]
      }
    ).first else {
      throw SessionStoreError.sessionNotFound
    }
    return value
  }

  private func sessionGroupLocked(_ sessionGroupID: String) throws -> SessionGroupRecord {
    guard let header = try catalog.query(
      """
      SELECT g.device_history_id, COALESCE(SUM(s.total_elapsed_ms), 0),
             CASE WHEN COUNT(s.distance_m) = COUNT(*) THEN SUM(s.distance_m) ELSE NULL END
      FROM session_group g
      JOIN session_group_member m ON m.session_group_id = g.session_group_id
      JOIN session_catalog s ON s.session_id = m.session_id AND s.pending_delete = 0
      WHERE g.session_group_id = ?
      GROUP BY g.session_group_id, g.device_history_id
      """,
      values: [.text(sessionGroupID)],
      map: { statement in
      (
        Self.columnText(statement, 0)!,
        sqlite3_column_int64(statement, 1),
        sqlite3_column_type(statement, 2) == SQLITE_NULL
          ? nil : sqlite3_column_double(statement, 2)
      )
    }).first else {
      throw SessionStoreError.sessionNotFound
    }
    let sessionIDs = try catalog.query(
      """
      SELECT m.session_id FROM session_group_member m
      JOIN session_catalog s ON s.session_id = m.session_id AND s.pending_delete = 0
      WHERE m.session_group_id = ? ORDER BY m.ordinal
      """,
      values: [.text(sessionGroupID)],
      map: { Self.columnText($0, 0)! }
    )
    guard sessionIDs.count >= 2 else { throw SessionStoreError.sessionNotFound }
    return SessionGroupRecord(
      sessionGroupID: sessionGroupID,
      deviceHistoryID: header.0,
      sessionIDs: sessionIDs,
      totalElapsedMs: header.1,
      distanceM: header.2
    )
  }

  private func resumePendingDeletes() throws {
    let pending = try catalog.query(
      "SELECT session_id, file_name FROM session_catalog WHERE pending_delete = 1"
    ) { statement in
      (Self.columnText(statement, 0)!, Self.columnText(statement, 1)!)
    }
    for (sessionID, fileName) in pending {
      try removeSessionFiles(fileName: fileName)
      try catalog.statement(
        "DELETE FROM session_catalog WHERE session_id = ?",
        values: [.text(sessionID)]
      )
    }
    try removeUndersizedGroupsLocked()
  }

  private func removeSessionFiles(fileName: String) throws {
    let file = root.appending(path: fileName)
    for suffix in ["", "-wal", "-shm"] {
      let candidate = URL(fileURLWithPath: file.path + suffix)
      if FileManager.default.fileExists(atPath: candidate.path) {
        try FileManager.default.removeItem(at: candidate)
      }
    }
  }

  private func removeUndersizedGroupsLocked() throws {
    try catalog.execute(
      """
      DELETE FROM session_group
      WHERE session_group_id IN (
        SELECT g.session_group_id FROM session_group g
        LEFT JOIN session_group_member m ON m.session_group_id = g.session_group_id
        GROUP BY g.session_group_id HAVING COUNT(m.session_id) < 2
      )
      """
    )
  }

  private func cleanupStagedExports() throws {
    let directory = root.appending(path: "Exports", directoryHint: .isDirectory)
    guard FileManager.default.fileExists(atPath: directory.path) else { return }
    let files = try FileManager.default.contentsOfDirectory(
      at: directory,
      includingPropertiesForKeys: [.contentModificationDateKey],
      options: [.skipsHiddenFiles]
    ).filter { ["csv", "floatview"].contains($0.pathExtension) }
      .sorted {
        let left = try? $0.resourceValues(forKeys: [.contentModificationDateKey])
          .contentModificationDate
        let right = try? $1.resourceValues(forKeys: [.contentModificationDateKey])
          .contentModificationDate
        return (left ?? .distantPast) > (right ?? .distantPast)
      }
    let cutoff = Date().addingTimeInterval(-86_400)
    for (index, file) in files.enumerated() {
      let modified = try? file.resourceValues(forKeys: [.contentModificationDateKey])
        .contentModificationDate
      if index >= 20 || (modified ?? .distantPast) < cutoff {
        try? FileManager.default.removeItem(at: file)
      }
    }
  }

  private func linkClosedSessionLocked(sessionID: String) throws {
    let settings = try settingsLocked()
    guard settings.automaticSessionLinkingEnabled else { return }
    guard let current = try catalog.query(
      """
      SELECT device_history_id, started_at_ms FROM session_catalog
      WHERE session_id = ? AND pending_delete = 0
      """,
      values: [.text(sessionID)],
      map: { statement in
      (Self.columnText(statement, 0)!, sqlite3_column_int64(statement, 1))
    }).first else {
      return
    }
    guard let previous = try catalog.query(
      """
      SELECT session_id, ended_at_ms FROM session_catalog
      WHERE device_history_id = ? AND session_id != ? AND pending_delete = 0
        AND ended_at_ms IS NOT NULL AND ended_at_ms <= ?
      ORDER BY ended_at_ms DESC, session_id DESC LIMIT 1
      """,
      values: [.text(current.0), .text(sessionID), .integer(current.1)],
      map: { statement in
      (Self.columnText(statement, 0)!, sqlite3_column_int64(statement, 1))
    }).first,
      current.1 - previous.1 <= settings.sessionLinkWindowMs
    else {
      return
    }
    let existingGroup = try catalog.query(
      "SELECT session_group_id FROM session_group_member WHERE session_id = ?",
      values: [.text(previous.0)],
      map: { Self.columnText($0, 0)! }
    ).first
    let groupID = existingGroup ?? UUID().uuidString
    if existingGroup == nil {
      try catalog.statement(
        """
        INSERT INTO session_group(session_group_id, device_history_id, created_at_ms)
        VALUES (?, ?, ?)
        """,
        values: [.text(groupID), .text(current.0), .integer(current.1)]
      )
      try catalog.statement(
        """
        INSERT INTO session_group_member(session_group_id, session_id, ordinal)
        VALUES (?, ?, 0)
        """,
        values: [.text(groupID), .text(previous.0)]
      )
    }
    let ordinal = try catalog.query(
      """
      SELECT COALESCE(MAX(ordinal), -1) + 1 FROM session_group_member
      WHERE session_group_id = ?
      """,
      values: [.text(groupID)],
      map: { Int(sqlite3_column_int64($0, 0)) }
    ).first ?? 0
    try catalog.statement(
      """
      INSERT OR IGNORE INTO session_group_member(session_group_id, session_id, ordinal)
      VALUES (?, ?, ?)
      """,
      values: [.text(groupID), .text(sessionID), .integer(Int64(ordinal))]
    )
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

  private func openSessionLocked(sessionID: String) throws -> Database {
    guard let fileName = try catalog.query(
      "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
      values: [.text(sessionID)],
      map: { Self.columnText($0, 0)! }
    ).first else { throw SessionStoreError.sessionNotFound }
    return try Database(url: root.appending(path: fileName))
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
    "boardState": "board_state",
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

  private static let settingKeys: Set<String> = [
    "idleProbeIntervalMs",
    "idlePersistIntervalMs",
    "ridingPersistIntervalMs",
    "postRideTailMs",
    "reconnectDurationMs",
    "automaticSessionLinkingEnabled",
    "sessionLinkWindowMs",
    "themePreference",
    "unitPreference",
  ]

  private static func settingsDictionary(_ value: SettingsRecord) -> [String: Any] {
    [
      "idleProbeIntervalMs": value.idleProbeIntervalMs,
      "idlePersistIntervalMs": value.idlePersistIntervalMs,
      "ridingPersistIntervalMs": value.ridingPersistIntervalMs,
      "postRideTailMs": value.postRideTailMs,
      "reconnectDurationMs": value.reconnectDurationMs,
      "automaticSessionLinkingEnabled": value.automaticSessionLinkingEnabled,
      "sessionLinkWindowMs": value.sessionLinkWindowMs,
      "themePreference": value.themePreference.rawValue,
      "unitPreference": value.unitPreference.rawValue,
    ]
  }

  private static func validatedSettings(
    _ values: [String: Any],
    revision: Int
  ) throws -> SettingsRecord {
    func integer(_ key: String, _ range: ClosedRange<Int64>) throws -> Int64 {
      guard let number = values[key] as? NSNumber,
            CFGetTypeID(number) != CFBooleanGetTypeID(),
            number.doubleValue.rounded() == number.doubleValue
      else { throw ProductStoreError.invalidSettingsPatch }
      return min(range.upperBound, max(range.lowerBound, number.int64Value))
    }
    func boolean(_ key: String) throws -> Bool {
      guard let number = values[key] as? NSNumber,
            CFGetTypeID(number) == CFBooleanGetTypeID()
      else { throw ProductStoreError.invalidSettingsPatch }
      return number.boolValue
    }
    func themePreference(_ key: String) throws -> ThemePreference {
      guard let rawValue = values[key] as? String,
            let value = ThemePreference(rawValue: rawValue)
      else { throw ProductStoreError.invalidSettingsPatch }
      return value
    }
    func unitPreference(_ key: String) throws -> UnitPreference {
      guard let rawValue = values[key] as? String,
            let value = UnitPreference(rawValue: rawValue)
      else { throw ProductStoreError.invalidSettingsPatch }
      return value
    }
    return try SettingsRecord(
      revision: revision,
      idleProbeIntervalMs: integer("idleProbeIntervalMs", 100 ... 10_000),
      idlePersistIntervalMs: integer("idlePersistIntervalMs", 100 ... 60_000),
      ridingPersistIntervalMs: integer("ridingPersistIntervalMs", 50 ... 10_000),
      postRideTailMs: integer("postRideTailMs", 0 ... 300_000),
      reconnectDurationMs: integer("reconnectDurationMs", 0 ... 3_600_000),
      automaticSessionLinkingEnabled: boolean("automaticSessionLinkingEnabled"),
      sessionLinkWindowMs: integer("sessionLinkWindowMs", 60_000 ... 86_400_000),
      themePreference: themePreference("themePreference"),
      unitPreference: unitPreference("unitPreference")
    )
  }

  private static func csv(_ value: String) -> String {
    "\"\(value.replacingOccurrences(of: "\"", with: "\"\""))\""
  }

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

  private func ensureSavedDeviceLocked(
    deviceID: DeviceID,
    advertisedName: String?,
    at: Date
  ) throws -> SavedDeviceRecord {
    let existing = try catalog.query(
      """
      SELECT saved_device_id, device_history_id, revision, custom_display_name,
             advertised_name, opaque_platform_id, last_seen_at_ms, last_connected_at_ms
      FROM saved_device WHERE opaque_platform_id = ? LIMIT 1
      """,
      values: [.text(deviceID.value)]
    ) { statement in
      SavedDeviceRecord(
        savedDeviceID: Self.columnText(statement, 0)!,
        deviceHistoryID: Self.columnText(statement, 1)!,
        revision: Int(sqlite3_column_int64(statement, 2)),
        customDisplayName: Self.columnText(statement, 3),
        advertisedName: Self.columnText(statement, 4),
        bluetoothIdentifier: Self.columnText(statement, 5)!,
        lastSeenAt: Self.columnDate(statement, 6),
        lastConnectedAt: Self.columnDate(statement, 7),
        profileComplete: false
      )
    }.first
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    if let existing {
      try catalog.statement(
        """
        UPDATE saved_device
        SET advertised_name = COALESCE(?, advertised_name), last_seen_at_ms = ?,
            last_connected_at_ms = ?
        WHERE saved_device_id = ?
        """,
        values: [
          advertisedName.map(SQLiteValue.text) ?? .null,
          .integer(timestamp), .integer(timestamp), .text(existing.savedDeviceID),
        ]
      )
      _ = try boardProfileLocked(savedDeviceID: existing.savedDeviceID, at: at)
      return SavedDeviceRecord(
        savedDeviceID: existing.savedDeviceID,
        deviceHistoryID: existing.deviceHistoryID,
        revision: existing.revision,
        customDisplayName: existing.customDisplayName,
        advertisedName: advertisedName ?? existing.advertisedName,
        bluetoothIdentifier: existing.bluetoothIdentifier,
        lastSeenAt: at,
        lastConnectedAt: at,
        profileComplete: false
      )
    }
    let savedDeviceID = UUID().uuidString
    let deviceHistoryID = UUID().uuidString
    try catalog.statement(
      """
      INSERT INTO saved_device(
        saved_device_id, device_history_id, opaque_platform_id, advertised_name,
        custom_display_name, revision, created_at_ms, last_seen_at_ms, last_connected_at_ms
      ) VALUES (?, ?, ?, ?, NULL, 0, ?, ?, ?)
      """,
      values: [
        .text(savedDeviceID), .text(deviceHistoryID), .text(deviceID.value),
        advertisedName.map(SQLiteValue.text) ?? .null,
        .integer(timestamp), .integer(timestamp), .integer(timestamp),
      ]
    )
    _ = try boardProfileLocked(savedDeviceID: savedDeviceID, at: at)
    return SavedDeviceRecord(
      savedDeviceID: savedDeviceID,
      deviceHistoryID: deviceHistoryID,
      advertisedName: advertisedName,
      bluetoothIdentifier: deviceID.value,
      lastSeenAt: at,
      lastConnectedAt: at,
      profileComplete: false
    )
  }

  private func controllerRoutingHintLocked(
    savedDeviceID: String
  ) throws -> ControllerRoutingHint? {
    try catalog.query(
      """
      SELECT can_id, controller_uuid
      FROM controller_route_hint WHERE saved_device_id = ? LIMIT 1
      """,
      values: [.text(savedDeviceID)]
    ) { statement in
      let rawUUID = Self.columnText(statement, 1).flatMap(Self.dataFromHex)
      return ControllerRoutingHint(
        savedDeviceID: savedDeviceID,
        canID: UInt8(sqlite3_column_int64(statement, 0)),
        controllerUUID: rawUUID
      )
    }.first
  }

  private func storeControllerRoutingHintLocked(
    _ hint: ControllerRoutingHint,
    at: Date
  ) throws {
    try catalog.statement(
      """
      INSERT INTO controller_route_hint(saved_device_id, can_id, controller_uuid, updated_at_ms)
      VALUES (?, ?, ?, ?)
      ON CONFLICT(saved_device_id) DO UPDATE SET
        can_id = excluded.can_id,
        controller_uuid = excluded.controller_uuid,
        updated_at_ms = excluded.updated_at_ms
      """,
      values: [
        .text(hint.savedDeviceID), .integer(Int64(hint.canID)),
        hint.controllerUUID.map { .text(Self.hexString($0)) } ?? .null,
        .integer(Int64(at.timeIntervalSince1970 * 1_000)),
      ]
    )
  }

  private func clearControllerRoutingHintLocked(savedDeviceID: String) throws {
    try catalog.statement(
      "DELETE FROM controller_route_hint WHERE saved_device_id = ?",
      values: [.text(savedDeviceID)]
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
          integer("parallelCount", range: 1 ... 20),
          integer("preferredCanId", range: 0 ... 253)
    else { return false }
    guard let motor = values["motor"], !(motor is NSNull) else { return true }
    return (motor as? String).map { !$0.isEmpty && $0.utf8.count <= 128 } ?? false
  }

  private static func preferredCanID(_ values: [String: Any]) -> Int? {
    guard let raw = values["preferredCanId"], !(raw is NSNull),
          let number = raw as? NSNumber
    else { return nil }
    return number.intValue
  }

  private static func hexString(_ data: Data) -> String {
    data.map { String(format: "%02x", $0) }.joined()
  }

  private static func dataFromHex(_ value: String) -> Data? {
    guard value.count == 24 else { return nil }
    var bytes: [UInt8] = []
    bytes.reserveCapacity(12)
    var index = value.startIndex
    for _ in 0 ..< 12 {
      let next = value.index(index, offsetBy: 2)
      guard let byte = UInt8(value[index ..< next], radix: 16) else { return nil }
      bytes.append(byte)
      index = next
    }
    return Data(bytes)
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
