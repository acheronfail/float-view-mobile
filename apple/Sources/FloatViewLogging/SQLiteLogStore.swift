import CSQLite
import FloatViewCore
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
    guard sqlite3_step(statement) == SQLITE_DONE else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
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

public final class SQLiteLogStore: LogStorePort, @unchecked Sendable {
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
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    try catalog.statement(
      """
      INSERT OR IGNORE INTO saved_device(
        saved_device_id, device_history_id, opaque_platform_id, display_name, created_at_ms
      ) VALUES (?, ?, ?, ?, ?)
      """,
      values: [
        .text("saved-\(deviceID.value)"), .text("history-\(deviceID.value)"),
        .text(deviceID.value), .text("Board"), .integer(timestamp),
      ]
    )
    try catalog.statement(
      """
      INSERT INTO session_catalog(
        session_id, job_id, device_history_id, device_snapshot_json,
        profile_snapshot_json, profile_revision, file_name, started_at_ms,
        state, integrity, requested_idle_ms, effective_idle_ms,
        requested_riding_ms, effective_riding_ms
      ) VALUES (?, ?, ?, '{}', '{}', 0, ?, ?, 'active', 'complete', 1000, 1000, 100, 100)
      """,
      values: [
        .text(sessionID.value), .text(jobID.value), .text("history-\(deviceID.value)"),
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
        try session.statement(
          """
          INSERT OR IGNORE INTO sample(
            sequence, observed_at_ms, monotonic_elapsed_ms, ride_state,
            raw_pack_voltage_v, duty_cycle, controller_temp_c, motor_temp_c,
            footpad_state, extension_json
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, '{}')
          """,
          values: [
            .integer(sample.sequence),
            .integer(Int64(sample.observedAt.timeIntervalSince1970 * 1_000)),
            .integer(sample.elapsedMs), .text(sample.rideState.rawValue),
            optional(sample.packVoltageV), optional(sample.dutyCycle),
            optional(sample.controllerTempC), optional(sample.motorTempC),
            sample.footpadState.map { .integer(Int64($0)) } ?? .null,
          ]
        )
      }
      let last = samples.map(\.sequence).max()!
      try session.statement(
        "INSERT OR REPLACE INTO commit_marker(singleton, committed_sequence, committed_at_ms) VALUES (1, ?, ?)",
        values: [.integer(last), .integer(Int64(samples.last!.observedAt.timeIntervalSince1970 * 1_000))]
      )
      try session.execute("COMMIT")
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

  public func recoverInterrupted() throws {
    try catalog.execute(
      """
      UPDATE session_catalog SET state = 'interrupted', integrity = 'recovered',
      close_reason = 'process_interruption' WHERE state = 'active'
      """
    )
  }
}
