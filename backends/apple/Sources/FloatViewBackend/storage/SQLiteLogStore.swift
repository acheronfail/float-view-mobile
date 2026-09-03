#if SWIFT_PACKAGE
  import CSQLite
#else
  import SQLite3
#endif
import CoreFoundation
import CryptoKit
import Foundation

public enum SQLiteLogStoreError: Error {
  case sqlite(String)
  case cancelled
  case noActiveSession
  case wrongJob
  case storageFull
}

private final class Database: @unchecked Sendable {
  private var handle: OpaquePointer?
  private var cancellation: SQLiteCancellation?

  init(url: URL, readOnly: Bool = false) throws {
    let flags = (readOnly ? SQLITE_OPEN_READONLY : SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE)
      | SQLITE_OPEN_FULLMUTEX
    if sqlite3_open_v2(
      url.path,
      &handle,
      flags,
      nil
    ) != SQLITE_OK {
      throw SQLiteLogStoreError.sqlite("Unable to open \(url.lastPathComponent)")
    }
  }

  func close() {
    guard let handle else { return }
    sqlite3_close(handle)
    self.handle = nil
  }

  deinit { close() }

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
      if cancellation?.isCancelled == true { throw SQLiteLogStoreError.cancelled }
      switch sqlite3_step(statement) {
      case SQLITE_ROW: try result.append(map(statement))
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
      if cancellation?.isCancelled == true { throw SQLiteLogStoreError.cancelled }
      switch sqlite3_step(statement) {
      case SQLITE_ROW: try body(statement)
      case SQLITE_DONE: return
      default:
        throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
      }
    }
  }

  func scalarInt(_ sql: String, values: [SQLiteValue] = []) throws -> Int {
    var statement: OpaquePointer?
    guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    defer { sqlite3_finalize(statement) }
    bind(values, to: statement)
    guard sqlite3_step(statement) == SQLITE_ROW else {
      throw SQLiteLogStoreError.sqlite(String(cString: sqlite3_errmsg(handle)))
    }
    return Int(sqlite3_column_int64(statement, 0))
  }

  func interrupt() {
    cancellation?.cancel()
    sqlite3_interrupt(handle)
  }

  func installCancellation(_ value: SQLiteCancellation) {
    cancellation = value
  }

  private func bind(_ values: [SQLiteValue], to statement: OpaquePointer?) {
    for (index, value) in values.enumerated() {
      let position = Int32(index + 1)
      switch value {
      case let .integer(value): sqlite3_bind_int64(statement, position, value)
      case let .real(value): sqlite3_bind_double(statement, position, value)
      case let .text(value):
        sqlite3_bind_text(statement, position, value, -1, SQLITE_TRANSIENT)
      case .null: sqlite3_bind_null(statement, position)
      }
    }
  }
}

private final class SQLiteCancellation: @unchecked Sendable {
  private let lock = NSLock()
  private var value = false

  var isCancelled: Bool {
    lock.withLock { value }
  }

  func cancel() {
    lock.withLock { value = true }
  }
}

private let SQLITE_TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

private enum SQLiteValue {
  case integer(Int64)
  case real(Double)
  case text(String)
  case null
}

private enum SessionReadCancellationContext {
  @TaskLocal static var current: SessionReadCancellation?
}

public final class SQLiteLogStore: LogStorePort, ProductStorePort, SessionStorePort, @unchecked Sendable {
  private struct RefloatSchemaCacheEnvelope: Codable {
    let identity: RefloatSchemaCacheIdentity
    let digest: String
    let data: Data
  }

  private static let csvLocationToleranceMs: Int64 = 10_000
  private static let initialMissingFields = ["seriesCount", "parallelCount", "motor"]
  private let root: URL
  private let schemaDirectory: URL
  private let catalog: Database
  private let availableBytes: @Sendable () -> Int64
  private var current: SessionRecord?
  private var session: Database?
  private let lock = NSLock()
  private var activeReadLeases: [String: Int] = [:]
  private var maintenanceSessions: Set<String> = []
  private var historicalConnections: [String: [Database]] = [:]
  private var historicalCancellations: [String: [SQLiteCancellation]] = [:]
  private let historicalReadSlots = DispatchSemaphore(value: 2)
  var onHistoricalReadRegisteredForTesting: (@Sendable (SessionReadCancellation?) -> Void)?
  /// A deletion is marked in the catalog before touching the session file. The
  /// in-memory set closes the small window between the file removal and the
  /// catalog transaction so cancellation cannot resurrect a session after the
  /// irreversible part has started.
  private var deletingSessions: Set<String> = []

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
    // The session marker is authoritative when a process died after a session
    // commit but before the catalog summary converged. Reconcile those rows
    // before any session list/read can expose stale boundaries or aggregates.
    try reconcileCatalogLocked()
    // Process death releases native leases. Preserve the bounded cursor and
    // counters, but expose an interrupted job as resumable on the next request.
    try catalog
      .execute(
        "UPDATE session_maintenance_job SET state = 'pending' WHERE singleton = 1 AND state IN ('running', 'cancel_requested')"
      )
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
      stableIdentity: nil,
      at: at
    )
    let profile = try boardProfileLocked(savedDeviceID: savedDevice.savedDeviceID, at: at)
    let settings = try settingsLocked()
    try session.statement(
      """
      INSERT INTO session_metadata(
        singleton, expected_riding_persistence_ms, expected_idle_persistence_ms
      ) VALUES (1, ?, ?)
      """,
      values: [.integer(settings.ridingPersistIntervalMs), .integer(settings.idlePersistIntervalMs)]
    )
    try session.statement(
      "INSERT INTO commit_marker(singleton, committed_at_ms) VALUES (1, ?)",
      values: [.integer(timestamp)]
    )
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
      stableIdentity: nil,
      at: at
    )
  }

  public func ensureSavedDevice(
    deviceID: DeviceID,
    advertisedName: String?,
    stableIdentity: String?,
    at: Date
  ) throws -> SavedDeviceRecord {
    lock.lock()
    defer { lock.unlock() }
    return try ensureSavedDeviceLocked(
      deviceID: deviceID,
      advertisedName: advertisedName,
      stableIdentity: stableIdentity,
      at: at
    )
  }

  public func create(deviceID: DeviceID, at: Date) throws -> SessionRecord {
    try create(deviceID: deviceID, advertisedName: nil, at: at)
  }

  public func activeSessionURL() -> URL? {
    lock.withLock {
      current.map { root.appending(path: "session-\($0.sessionID.value).sqlite") }
    }
  }

  public func append(jobID: JobID, samples: [TelemetrySample]) throws {
    lock.lock()
    defer { lock.unlock() }
    guard var record = current, let session else { throw SQLiteLogStoreError.noActiveSession }
    guard record.jobID == jobID else { throw SQLiteLogStoreError.wrongJob }
    try reconcileDirtyCatalogLocked(jobID: jobID.value)
    guard StoragePolicy.level(availableBytes: availableBytes()) != .stop else {
      throw SQLiteLogStoreError.storageFull
    }
    guard !samples.isEmpty else { return }
    try session.execute("BEGIN IMMEDIATE")
    do {
      var previousTransitions = try Self.latestTransitionValues(in: session)
      var eventSequence = try session.query(
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM session_event",
        map: { sqlite3_column_int64($0, 0) }
      ).first ?? 1
      for sample in samples {
        let optional: (Double?) -> SQLiteValue = { $0.map(SQLiteValue.real) ?? .null }
        let extensionJSON = try Self.telemetryExtensionJSON(sample.extensionFields)
        let values: [SQLiteValue] = [
          .integer(sample.sequence),
          .integer(Int64(sample.observedAt.timeIntervalSince1970 * 1_000)),
          sample.slowObservedAt.map {
            .integer(Int64($0.timeIntervalSince1970 * 1_000))
          } ?? .null,
          .integer(sample.elapsedMs),
          .text(sample.rideState.rawValue),
          sample.packageState.map { .text($0.rawValue) } ?? .null,
          sample.packageMode.map { .text($0.rawValue) } ?? .null,
          sample.stopCondition.map { .text($0.rawValue) } ?? .null,
          sample.setpointAdjustment.map { .text($0.rawValue) } ?? .null,
          sample.wheelslip.map { .integer($0 ? 1 : 0) } ?? .null,
          sample.darkride.map { .integer($0 ? 1 : 0) } ?? .null,
          sample.charging.map { .integer($0 ? 1 : 0) } ?? .null,
          sample.alertReason.map { .text($0.rawValue) } ?? .null,
          sample.firmwareFault.map { .text($0.rawValue) } ?? .null,
          optional(sample.packVoltageV),
          optional(sample.dutyCycle),
          optional(sample.controllerTempC),
          optional(sample.motorTempC),
          sample.footpadState.map { .text($0.rawValue) } ?? .null,
          .text(extensionJSON),
          optional(sample.motorCurrentA),
          optional(sample.batteryCurrentA),
          optional(sample.erpm),
          optional(sample.speedKph),
          optional(sample.pitchDeg),
          optional(sample.balancePitchDeg),
          optional(sample.rollDeg),
          optional(sample.directionalMotorCurrentA),
          optional(sample.filteredMotorCurrentA),
          optional(sample.requestedBalanceCurrentA),
          optional(sample.boosterCurrentA),
          optional(sample.fieldWeakeningCurrentA),
          optional(sample.footpadAdc1V),
          optional(sample.footpadAdc2V),
          optional(sample.remoteInput),
          optional(sample.setpointDeg),
          optional(sample.atrSetpointDeg),
          optional(sample.brakeTiltSetpointDeg),
          optional(sample.torqueTiltSetpointDeg),
          optional(sample.turnTiltSetpointDeg),
          optional(sample.remoteSetpointDeg),
          optional(sample.atrAccelerationDifferenceErpmPerSecond),
          optional(sample.atrSpeedBoost),
          optional(sample.distanceM),
          optional(sample.odometerM),
          optional(sample.ampHoursDrawnAh),
          optional(sample.ampHoursChargedAh),
          optional(sample.wattHoursDrawnWh),
          optional(sample.wattHoursChargedWh),
          optional(sample.batteryLevel),
          optional(sample.chargingCurrentA),
          optional(sample.chargingVoltageV),
        ]
        try session.statement(
          """
          INSERT OR IGNORE INTO sample(
            sequence, observed_at_ms, slow_observed_at_ms, monotonic_elapsed_ms, ride_state,
            package_state, package_mode, stop_condition, setpoint_adjustment,
            wheelslip, darkride, charging, alert_reason, firmware_fault,
            raw_pack_voltage_v, duty_cycle, controller_temp_c, motor_temp_c,
            footpad_state, extension_json,
            motor_current_a, battery_current_a, erpm, speed_kph,
            pitch_deg, balance_pitch_deg, roll_deg, directional_motor_current_a,
            filtered_motor_current_a, requested_balance_current_a, booster_current_a,
            field_weakening_current_a, footpad_adc_1_v, footpad_adc_2_v, remote_input,
            setpoint_deg, atr_setpoint_deg, brake_tilt_setpoint_deg, torque_tilt_setpoint_deg,
            turn_tilt_setpoint_deg, remote_setpoint_deg,
            atr_acceleration_difference_erpm_per_second, atr_speed_boost, distance_m,
            odometer_m, amp_hours_drawn_ah, amp_hours_charged_ah, watt_hours_drawn_wh,
            watt_hours_charged_wh, battery_level, charging_current_a, charging_voltage_v
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          """,
          values: values
        )
        let inserted = try session.query(
          "SELECT changes()",
          map: { sqlite3_column_int64($0, 0) }
        ).first == 1
        guard inserted else { continue }
        for (field, value) in Self.numericValues(sample) where value != nil {
          try session.statement(
            "INSERT OR IGNORE INTO session_numeric_field(field_name, source, first_sequence) VALUES (?, 'stable', ?)",
            values: [.text(field), .integer(sample.sequence)]
          )
        }
        for field in sample.extensionFields.keys {
          try session.statement(
            "INSERT OR IGNORE INTO session_numeric_field(field_name, source, first_sequence) VALUES (?, 'extension', ?)",
            values: [.text(field), .integer(sample.sequence)]
          )
        }
        for (kind, value) in Self.transitionValues(sample) {
          if previousTransitions[kind] == value { continue }
          let detailsJSON = try Self.jsonString(["value": value])
          try session.statement(
            """
            INSERT INTO session_event(
              sequence, source_sample_sequence, observed_at_ms, monotonic_elapsed_ms, kind, details_json
            ) VALUES (?, ?, ?, ?, ?, ?)
            """,
            values: [
              .integer(eventSequence),
              .integer(sample.sequence),
              .integer(Int64(sample.observedAt.timeIntervalSince1970 * 1_000)),
              .integer(sample.elapsedMs), .text(kind), .text(detailsJSON),
            ]
          )
          previousTransitions[kind] = value
          eventSequence += 1
        }
      }
      let last = samples.map(\.sequence).max()!
      try session.statement(
        """
        WITH ordered AS (
          SELECT sequence, monotonic_elapsed_ms, ride_state, speed_kph, duty_cycle,
                 LAG(monotonic_elapsed_ms) OVER (
                   ORDER BY monotonic_elapsed_ms, sequence
                 ) AS previous_elapsed_ms,
                 LAG(speed_kph) OVER (
                   ORDER BY monotonic_elapsed_ms, sequence
                 ) AS previous_speed_kph
          FROM sample
        )
        UPDATE commit_marker SET
          committed_sample_sequence = MAX(committed_sample_sequence, ?),
          committed_event_sequence = COALESCE((SELECT MAX(sequence) FROM session_event), 0),
          recorded_sample_count = recorded_sample_count + ?,
          stored_sample_count = (SELECT COUNT(*) FROM sample),
          committed_event_count = (SELECT COUNT(*) FROM session_event),
          committed_elapsed_ms = COALESCE((SELECT MAX(monotonic_elapsed_ms) FROM sample), 0),
          committed_at_ms = ?,
          riding_elapsed_ms = COALESCE((SELECT SUM(
            CASE WHEN ride_state = 'riding' AND previous_elapsed_ms IS NOT NULL
              THEN MAX(0, monotonic_elapsed_ms - previous_elapsed_ms) ELSE 0 END
          ) FROM ordered), 0),
          distance_m = (SELECT COALESCE(SUM(
            CASE WHEN speed_kph IS NOT NULL AND previous_elapsed_ms IS NOT NULL
              THEN ((ABS(previous_speed_kph) + ABS(speed_kph)) / 2.0)
                * MAX(0, monotonic_elapsed_ms - previous_elapsed_ms) / 3600.0
              ELSE 0 END
          ), 0) FROM ordered),
          maximum_speed_kph = (SELECT MAX(ABS(speed_kph)) FROM sample),
          maximum_speed_source_sequence = (
            SELECT sequence FROM sample
            WHERE speed_kph IS NOT NULL
            ORDER BY ABS(speed_kph) DESC, monotonic_elapsed_ms, sequence LIMIT 1
          ),
          maximum_duty_cycle = (SELECT MAX(ABS(duty_cycle)) FROM sample),
          maximum_duty_source_sequence = (
            SELECT sequence FROM sample
            WHERE duty_cycle IS NOT NULL
            ORDER BY ABS(duty_cycle) DESC, monotonic_elapsed_ms, sequence LIMIT 1
          ),
          last_speed_kph = (
            SELECT speed_kph FROM sample
            ORDER BY monotonic_elapsed_ms DESC, sequence DESC LIMIT 1
          ),
          last_ride_state = (
            SELECT ride_state FROM sample
            ORDER BY monotonic_elapsed_ms DESC, sequence DESC LIMIT 1
          )
        WHERE singleton = 1
        """,
        values: [
          .integer(last), .integer(Int64(samples.count)),
          .integer(Int64(samples.last!.observedAt.timeIntervalSince1970 * 1_000)),
        ]
      )
      try session.execute("COMMIT")
      do {
        // Catalog convergence is deliberately outside the writer transaction.
        // A catalog failure must not make a committed batch appear rolled back;
        // mark it dirty so the marker-driven reconciler repairs it before the
        // next append/read/publication.
        try updateCatalogSummaryLocked(record: record, samples: samples)
      } catch {
        try? markCatalogDirtyLocked(jobID: record.jobID.value)
        throw error
      }
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

  public func appendLocations(jobID: JobID, observations: [LocationObservation]) throws {
    lock.lock()
    defer { lock.unlock() }
    guard let record = current, record.jobID == jobID, let session else {
      throw SQLiteLogStoreError.noActiveSession
    }
    if observations.isEmpty { return }
    let ordered = observations.sorted { left, right in
      left.elapsedMs < right.elapsedMs || (left.elapsedMs == right.elapsedMs && left.observedAt < right.observedAt)
    }
    try session.execute("BEGIN IMMEDIATE")
    do {
      var sequence = try session.scalarInt("SELECT COALESCE(MAX(location_sequence), 0) FROM location")
      var accepted = 0
      for observation in ordered {
        guard observation.elapsedMs >= 0,
              (-90 ... 90).contains(observation.latitude),
              (-180 ... 180).contains(observation.longitude),
              observation.accuracyM.map({ $0.isFinite && $0 >= 0 }) ?? true
        else { throw SessionStoreError.invalidRange }
        let duplicate = try session.scalarInt(
          "SELECT EXISTS(SELECT 1 FROM location WHERE monotonic_elapsed_ms = ? AND latitude = ? AND longitude = ?)",
          values: [.integer(observation.elapsedMs), .real(observation.latitude), .real(observation.longitude)]
        ) == 1
        if duplicate { continue }
        sequence += 1
        try session.statement(
          "INSERT INTO location(location_sequence, observed_at_ms, monotonic_elapsed_ms, latitude, longitude, accuracy_m) VALUES (?, ?, ?, ?, ?, ?)",
          values: [
            .integer(Int64(sequence)),
            .integer(Int64(observation.observedAt.timeIntervalSince1970 * 1_000)),
            .integer(observation.elapsedMs),
            .real(observation.latitude),
            .real(observation.longitude),
            observation.accuracyM.map(SQLiteValue.real) ?? .null,
          ]
        )
        accepted += 1
      }
      if accepted > 0 {
        let elapsed = try session.scalarInt(
          "SELECT monotonic_elapsed_ms FROM location WHERE location_sequence = ?",
          values: [.integer(Int64(sequence))]
        )
        try session.statement(
          "UPDATE commit_marker SET committed_location_sequence = ?, committed_location_elapsed_ms = ?, recorded_location_count = recorded_location_count + ?, stored_location_count = (SELECT COUNT(*) FROM location), last_location_status = 'available' WHERE singleton = 1",
          values: [.integer(Int64(sequence)), .integer(Int64(elapsed)), .integer(Int64(accepted))]
        )
      }
      try session.execute("COMMIT")
      let marker = try session.query(
        "SELECT committed_location_sequence, committed_location_elapsed_ms, recorded_location_count, stored_location_count, last_location_status FROM commit_marker WHERE singleton = 1",
        map: { statement in
          (
            sqlite3_column_int64(statement, 0),
            sqlite3_column_int64(statement, 1),
            sqlite3_column_int64(statement, 2),
            sqlite3_column_int64(statement, 3),
            Self.columnText(statement, 4) ?? "disabled"
          )
        }
      ).first!
      try catalog.statement(
        "UPDATE session_catalog SET committed_location_sequence = ?, committed_location_elapsed_ms = ?, location_count = ?, recorded_location_count = ?, stored_location_count = ?, location_status = ? WHERE job_id = ?",
        values: [
          .integer(marker.0),
          .integer(marker.1),
          .integer(marker.2),
          .integer(marker.2),
          .integer(marker.3),
          .text(marker.4),
          .text(jobID.value),
        ]
      )
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
    try reconcileDirtyCatalogLocked(jobID: jobID.value)
    let enteredRiding = try session?.scalarInt(
      "SELECT EXISTS(SELECT 1 FROM sample WHERE ride_state = 'riding')"
    ) == 1
    let distance = try catalog.query(
      "SELECT COALESCE(distance_m, 0) FROM session_catalog WHERE job_id = ?",
      values: [.text(jobID.value)],
      map: { sqlite3_column_double($0, 0) }
    ).first ?? 0
    let minimumSessionDistance = try settingsLocked().minimumSessionDistance
    let retainSession = enteredRiding && distance >= minimumSessionDistance
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
      if retainSession {}
      try catalog.execute("COMMIT")
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
    session = nil
    current = nil
    if !retainSession {
      try deleteSessionLocked(sessionID: record.sessionID.value)
    }
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
    try reconcileDirtyCatalogLocked()
    guard (1 ... 100).contains(query.limit) else { throw SessionStoreError.invalidRange }
    guard let sortColumn = Self.sessionSortColumns[query.sortField] else {
      throw SessionStoreError.invalidRange
    }
    let cursor = try query.cursor.map(Self.decodeCursor)
    if let cursor, cursor.field != query.sortField { throw SessionStoreError.invalidCursor }
    var conditions = ["pending_delete = 0"]
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
    try reconcileDirtyCatalogLocked()
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

  public func sessionSnapshot(sessionID: String) throws -> SessionPublishedSnapshotRecord {
    lock.lock()
    defer { lock.unlock() }
    try reconcileDirtyCatalogLocked()
    guard let value = try catalog.query(
      """
      SELECT history_revision, committed_sequence, committed_event_sequence,
             committed_elapsed_ms, sample_count, stored_sample_count,
             committed_location_sequence, committed_location_elapsed_ms,
             location_count, recorded_location_count, stored_location_count, location_status
      FROM session_catalog WHERE session_id = ? AND pending_delete = 0
      """,
      values: [.text(sessionID)],
      map: { statement in
        SessionPublishedSnapshotRecord(
          historyRevision: sqlite3_column_int64(statement, 0),
          committedSampleSequence: sqlite3_column_int64(statement, 1),
          committedEventSequence: sqlite3_column_int64(statement, 2),
          committedElapsedMs: sqlite3_column_int64(statement, 3),
          recordedSampleCount: sqlite3_column_int64(statement, 4),
          storedSampleCount: sqlite3_column_int64(statement, 5),
          committedLocationSequence: sqlite3_column_int64(statement, 6),
          committedLocationElapsedMs: sqlite3_column_int64(statement, 7),
          recordedLocationCount: sqlite3_column_int64(statement, 9),
          storedLocationCount: sqlite3_column_int64(statement, 10),
          locationStatus: Self.columnText(statement, 11) ?? "disabled"
        )
      }
    ).first else { throw SessionStoreError.sessionNotFound }
    return value
  }

  public func sessionURL(sessionID: String) throws -> URL {
    lock.lock()
    defer { lock.unlock() }
    guard let fileName = try catalog.query(
      "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
      values: [.text(sessionID)], map: { Self.columnText($0, 0)! }
    ).first else { throw SessionStoreError.sessionNotFound }
    return root.appending(path: fileName)
  }

  public func reconcileSession(sessionID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    guard let row = try catalog.query(
      "SELECT job_id, file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
      values: [.text(sessionID)], map: { (Self.columnText($0, 0)!, Self.columnText($0, 1)!) }
    ).first else { throw SessionStoreError.sessionNotFound }
    try reconcileSessionCatalogLocked(sessionID: sessionID, jobID: row.0, fileName: row.1)
  }

  public func applySessionCommitMarker(
    sessionID: String,
    marker: SessionCommitMarkerRecord
  ) throws {
    lock.lock()
    defer { lock.unlock() }
    guard let record = current, record.sessionID.value == sessionID else {
      throw SessionStoreError.sessionNotFound
    }
    let snapshot = marker.snapshot
    try catalog.statement(
      """
      UPDATE session_catalog SET integrity = 'complete', committed_sequence = ?,
        committed_event_sequence = ?, sample_count = ?, stored_sample_count = ?,
        event_count = ?, committed_elapsed_ms = ?, history_revision = ?,
        riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?, max_speed_mps = ?,
        max_duty_cycle = ?, max_speed_source_sequence = ?, max_duty_source_sequence = ?,
        last_sample_elapsed_ms = ?, last_speed_mps = ?, committed_location_sequence = ?,
        committed_location_elapsed_ms = ?, location_count = ?, recorded_location_count = ?,
        stored_location_count = ?, location_status = ?
      WHERE session_id = ? AND job_id = ? AND state = 'active' AND pending_delete = 0
      """,
      values: [
        .integer(snapshot.committedSampleSequence), .integer(snapshot.committedEventSequence),
        .integer(snapshot.recordedSampleCount), .integer(snapshot.storedSampleCount),
        .integer(marker.committedEventCount), .integer(snapshot.committedElapsedMs),
        .integer(snapshot.historyRevision), .integer(marker.ridingElapsedMs),
        .integer(snapshot.committedElapsedMs), marker.distanceM.map(SQLiteValue.real) ?? .null,
        marker.maximumSpeedKph.map { .real(abs($0) / 3.6) } ?? .null,
        marker.maximumDutyCycle.map { .real(abs($0)) } ?? .null,
        marker.maximumSpeedSourceSequence.map(SQLiteValue.integer) ?? .null,
        marker.maximumDutySourceSequence.map(SQLiteValue.integer) ?? .null,
        .integer(snapshot.committedElapsedMs),
        marker.lastSpeedKph.map { .real($0 / 3.6) } ?? .null,
        .integer(snapshot.committedLocationSequence),
        .integer(snapshot.committedLocationElapsedMs),
        .integer(snapshot.recordedLocationCount), .integer(snapshot.recordedLocationCount),
        .integer(snapshot.storedLocationCount), .text(snapshot.locationStatus),
        .text(sessionID), .text(record.jobID.value),
      ]
    )
    current = SessionRecord(
      sessionID: record.sessionID, jobID: record.jobID, deviceID: record.deviceID,
      startedAt: record.startedAt, committedSequence: snapshot.committedSampleSequence,
      active: true
    )
  }

  public func maintenanceJob() throws -> SessionMaintenanceJobRecord? {
    lock.lock()
    defer { lock.unlock() }
    return try catalog.query(
      """
      SELECT operation_id, kind, scope, state, session_id, target_resolution_seconds,
             upper_session_id, last_session_id, completed_sessions, skipped_sessions,
             retained_samples, reclaimable_bytes, error
      FROM session_maintenance_job WHERE singleton = 1
      """,
      map: { statement in
        SessionMaintenanceJobRecord(
          operationID: Self.columnText(statement, 0)!,
          kind: Self.columnText(statement, 1)!,
          scope: Self.columnText(statement, 2)!,
          state: Self.columnText(statement, 3)!,
          sessionID: Self.columnText(statement, 4),
          targetResolutionSeconds: sqlite3_column_int64(statement, 5),
          upperSessionID: Self.columnText(statement, 6),
          lastSessionID: Self.columnText(statement, 7),
          completedSessions: sqlite3_column_int64(statement, 8),
          skippedSessions: sqlite3_column_int64(statement, 9),
          retainedSamples: sqlite3_column_int64(statement, 10),
          reclaimableBytes: sqlite3_column_int64(statement, 11),
          error: Self.columnText(statement, 12)
        )
      }
    ).first
  }

  public func saveMaintenanceJob(_ job: SessionMaintenanceJobRecord) throws {
    lock.lock()
    defer { lock.unlock() }
    try catalog.statement(
      """
      INSERT OR REPLACE INTO session_maintenance_job(
        singleton, operation_id, kind, scope, session_id, state,
        target_resolution_seconds, upper_session_id, last_session_id,
        completed_sessions, skipped_sessions, retained_samples,
        reclaimable_bytes, error, updated_at_ms
      ) VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      """,
      values: [
        .text(job.operationID), .text(job.kind), .text(job.scope),
        job.sessionID.map(SQLiteValue.text) ?? .null, .text(job.state),
        .integer(job.targetResolutionSeconds), job.upperSessionID.map(SQLiteValue.text) ?? .null,
        job.lastSessionID.map(SQLiteValue.text) ?? .null, .integer(job.completedSessions),
        .integer(job.skippedSessions), .integer(job.retainedSamples), .integer(job.reclaimableBytes),
        job.error.map(SQLiteValue.text) ?? .null,
        .integer(Int64(Date().timeIntervalSince1970 * 1_000)),
      ]
    )
  }

  public func cancelMaintenanceJob(operationID: String) throws -> SessionMaintenanceJobRecord? {
    guard let job = try maintenanceJob(), job.operationID == operationID else { return nil }
    guard !["completed", "cancelled", "failed"].contains(job.state) else { return job }
    let updated = SessionMaintenanceJobRecord(
      operationID: job.operationID, kind: job.kind, scope: job.scope,
      state: "cancel_requested", sessionID: job.sessionID,
      targetResolutionSeconds: job.targetResolutionSeconds, upperSessionID: job.upperSessionID,
      lastSessionID: job.lastSessionID, completedSessions: job.completedSessions,
      skippedSessions: job.skippedSessions, retainedSamples: job.retainedSamples,
      reclaimableBytes: job.reclaimableBytes, error: job.error
    )
    try saveMaintenanceJob(updated)
    if let sessionID = job.sessionID { cancelHistoricalReads(sessionID: sessionID) }
    return updated
  }

  public func savedDevices() throws -> [SavedDeviceRecord] {
    lock.lock()
    defer { lock.unlock() }
    return try savedDevicesLocked()
  }

  public func deleteSavedDevice(savedDeviceID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    guard current == nil else { throw ProductStoreError.savedDeviceActive }
    guard let deviceHistoryID = try catalog.query(
      "SELECT device_history_id FROM saved_device WHERE saved_device_id = ?",
      values: [.text(savedDeviceID)],
      map: { Self.columnText($0, 0)! }
    ).first else { throw ProductStoreError.savedDeviceNotFound }
    let files = try catalog.query(
      "SELECT file_name FROM session_catalog WHERE device_history_id = ?",
      values: [.text(deviceHistoryID)],
      map: { Self.columnText($0, 0)! }
    )
    try catalog.execute("BEGIN IMMEDIATE")
    do {
      try catalog.statement(
        "DELETE FROM board_profile WHERE saved_device_id = ?",
        values: [.text(savedDeviceID)]
      )
      try catalog.statement(
        "DELETE FROM controller_route_hint WHERE saved_device_id = ?",
        values: [.text(savedDeviceID)]
      )
      try catalog.statement(
        "DELETE FROM session_catalog WHERE device_history_id = ?",
        values: [.text(deviceHistoryID)]
      )
      try catalog.statement(
        "DELETE FROM saved_device WHERE saved_device_id = ?",
        values: [.text(savedDeviceID)]
      )
      guard try catalog.scalarInt("SELECT changes()") == 1 else {
        throw ProductStoreError.savedDeviceNotFound
      }
      try catalog.execute("COMMIT")
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
    for fileName in files {
      let file = root.appending(path: fileName)
      try? FileManager.default.removeItem(at: file)
      try? FileManager.default.removeItem(atPath: file.path + "-wal")
      try? FileManager.default.removeItem(atPath: file.path + "-shm")
    }
  }

  public func ensureImportedSavedDevice(
    deviceHistoryID: String,
    displayName: String?,
    profileSnapshotJSON: String,
    at: Date
  ) throws -> SavedDeviceRecord {
    lock.lock()
    defer { lock.unlock() }
    return try ensureImportedSavedDeviceLocked(
      deviceHistoryID: deviceHistoryID,
      displayName: displayName,
      profileSnapshotJSON: profileSnapshotJSON,
      at: at
    )
  }

  private func savedDevicesLocked() throws -> [SavedDeviceRecord] {
    return try catalog.query(
      """
      SELECT d.saved_device_id, d.device_history_id, d.revision,
             d.custom_display_name, d.advertised_name, d.opaque_platform_id,
             d.last_seen_at_ms, d.last_connected_at_ms,
             COALESCE(p.json, '{}'),
             COUNT(s.session_id),
             COALESCE(SUM(s.riding_elapsed_ms), 0),
             SUM(s.distance_m),
             MAX(s.started_at_ms)
      FROM saved_device d
      LEFT JOIN board_profile p ON p.saved_device_id = d.saved_device_id
      LEFT JOIN session_catalog s
        ON s.device_history_id = d.device_history_id AND s.pending_delete = 0
      GROUP BY d.saved_device_id, d.device_history_id, d.revision,
               d.custom_display_name, d.advertised_name, d.opaque_platform_id,
               d.last_seen_at_ms, d.last_connected_at_ms, p.json, d.created_at_ms
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
        bluetoothIdentifier: Self.columnText(statement, 5),
        lastSeenAt: Self.columnDate(statement, 6),
        lastConnectedAt: Self.columnDate(statement, 7),
        profileComplete: Self.missingFields(valuesJSON: valuesJSON).isEmpty,
        summary: BoardHistorySummaryRecord(
          sessionCount: Int(sqlite3_column_int64(statement, 9)),
          totalRidingElapsedMs: sqlite3_column_int64(statement, 10),
          totalDistanceM: sqlite3_column_type(statement, 11) == SQLITE_NULL
            ? nil : sqlite3_column_double(statement, 11),
          mostRecentSessionAt: Self.columnDate(statement, 12)
        )
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
      let json = try String(data: JSONEncoder().encode(updated), encoding: .utf8)!
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

  public func refloatConfigurations(
    identity: RefloatConfigurationIdentityRecord, limit: Int, cursor: String?
  ) throws -> RefloatConfigurationListPage {
    guard Self.validRefloatIdentity(identity), (1 ... 50).contains(limit) else {
      throw ProductStoreError.invalidRefloatConfiguration
    }
    let cursorValue: (Int64, String)?
    if let cursor {
      guard cursor.utf8.count <= 512, let data = Data(base64Encoded: cursor),
            let value = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let updated = value["updatedAtMs"] as? NSNumber,
            let identifier = value["configurationId"] as? String,
            value["fingerprint"] as? String == identity.schemaFingerprint
      else { throw ProductStoreError.invalidRefloatConfiguration }
      cursorValue = (updated.int64Value, identifier)
    } else { cursorValue = nil }
    lock.lock()
    defer { lock.unlock() }
    var sql = """
    SELECT configuration_id, revision, name, description, package_name, package_version,
           schema_fingerprint, wire_signature, values_json, created_at_ms, updated_at_ms
    FROM refloat_configuration
    WHERE package_name = ? AND package_version = ? AND schema_fingerprint = ? AND wire_signature = ?
    """
    var values: [SQLiteValue] = [
      .text(identity.packageName), .text(identity.packageVersion), .text(identity.schemaFingerprint),
      .integer(Int64(identity.wireSignature)),
    ]
    if let cursorValue {
      sql += " AND (updated_at_ms < ? OR (updated_at_ms = ? AND configuration_id < ?))"
      values += [.integer(cursorValue.0), .integer(cursorValue.0), .text(cursorValue.1)]
    }
    sql += " ORDER BY updated_at_ms DESC, configuration_id DESC LIMIT ?"
    values.append(.integer(Int64(limit + 1)))
    var records = try catalog.query(sql, values: values, map: Self.refloatConfigurationRecord)
    let hasMore = records.count > limit
    if hasMore { records.removeLast(records.count - limit) }
    let next = hasMore ? records.last.flatMap { record -> String? in
      let value: [String: Any] = [
        "updatedAtMs": Int64(record.updatedAt.timeIntervalSince1970 * 1_000),
        "configurationId": record.configurationID, "fingerprint": identity.schemaFingerprint,
      ]
      return (try? JSONSerialization.data(withJSONObject: value))?.base64EncodedString()
    } : nil
    return .init(items: records, nextCursor: next)
  }

  public func cachedRefloatSchema(identity: RefloatSchemaCacheIdentity) -> Data? {
    guard identity.hardwareConfigurationCRC != 0 else { return nil }
    lock.lock()
    defer { lock.unlock() }
    let url = refloatSchemaCacheURL(identity)
    do {
      let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
      guard (attributes[.size] as? NSNumber)?.intValue ?? 0 <= 64 * 1024 else { throw CocoaError(.fileReadCorruptFile) }
      let envelope = try JSONDecoder().decode(
        RefloatSchemaCacheEnvelope.self, from: Data(contentsOf: url, options: .mappedIfSafe)
      )
      guard envelope.identity == identity, envelope.data.count <= RefloatConfigurationLimits.compressedBytes,
            envelope.digest == Self.sha256Hex(envelope.data)
      else { throw CocoaError(.fileReadCorruptFile) }
      return envelope.data
    } catch {
      try? FileManager.default.removeItem(at: url)
      return nil
    }
  }

  public func storeRefloatSchema(_ data: Data, identity: RefloatSchemaCacheIdentity) {
    guard identity.hardwareConfigurationCRC != 0,
          data.count <= RefloatConfigurationLimits.compressedBytes
    else { return }
    lock.lock()
    defer { lock.unlock() }
    do {
      let directory = root.appending(path: "refloat-schema-cache", directoryHint: .isDirectory)
      try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
      let envelope = RefloatSchemaCacheEnvelope(
        identity: identity, digest: Self.sha256Hex(data), data: data
      )
      try JSONEncoder().encode(envelope).write(to: refloatSchemaCacheURL(identity), options: .atomic)
      let files = try FileManager.default.contentsOfDirectory(
        at: directory, includingPropertiesForKeys: [.contentModificationDateKey]
      ).sorted {
        (try? $0.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast <
          ((try? $1.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast)
      }
      for url in files.dropLast(8) {
        try? FileManager.default.removeItem(at: url)
      }
    } catch {}
  }

  public func removeCachedRefloatSchema(identity: RefloatSchemaCacheIdentity) {
    lock.lock()
    defer { lock.unlock() }
    try? FileManager.default.removeItem(at: refloatSchemaCacheURL(identity))
  }

  private func refloatSchemaCacheURL(_ identity: RefloatSchemaCacheIdentity) -> URL {
    let key = "\(identity.firmwareVersion)\u{0}\(identity.hardwareName)\u{0}\(identity.packageVersion)\u{0}\(identity.customIndex)\u{0}\(identity.hardwareConfigurationCRC)"
    return root.appending(path: "refloat-schema-cache/\(Self.sha256Hex(Data(key.utf8))).json")
  }

  private static func sha256Hex(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
  }

  public func refloatConfiguration(configurationID: String) throws -> RefloatConfigurationRecord {
    lock.lock()
    defer { lock.unlock() }
    return try refloatConfigurationLocked(configurationID: configurationID)
  }

  public func createRefloatConfiguration(
    name: String, description: String?, identity: RefloatConfigurationIdentityRecord,
    valuesJSON: String, at: Date
  ) throws -> RefloatConfigurationRecord {
    guard Self.validRefloatConfiguration(
      name: name,
      description: description,
      identity: identity,
      valuesJSON: valuesJSON
    )
    else { throw ProductStoreError.invalidRefloatConfiguration }
    lock.lock()
    defer { lock.unlock() }
    let identifier = UUID().uuidString
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    try catalog.statement(
      """
      INSERT INTO refloat_configuration(
        configuration_id, revision, name, description, package_name, package_version,
        schema_fingerprint, wire_signature, values_json, created_at_ms, updated_at_ms
      ) VALUES (?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      """,
      values: [
        .text(identifier), .text(name), description.map(SQLiteValue.text) ?? .null,
        .text(identity.packageName), .text(identity.packageVersion), .text(identity.schemaFingerprint),
        .integer(Int64(identity.wireSignature)), .text(valuesJSON), .integer(timestamp), .integer(timestamp),
      ]
    )
    return try refloatConfigurationLocked(configurationID: identifier)
  }

  public func updateRefloatConfiguration(
    configurationID: String, expectedRevision: Int, name: String,
    description: String?, valuesJSON: String, at: Date
  ) throws -> RefloatConfigurationRecord {
    lock.lock()
    defer { lock.unlock() }
    let current = try refloatConfigurationLocked(configurationID: configurationID)
    guard current.revision == expectedRevision else {
      throw ProductStoreError.refloatConfigurationRevisionConflict(currentRevision: current.revision)
    }
    guard Self.validRefloatConfiguration(
      name: name, description: description, identity: current.identity, valuesJSON: valuesJSON
    ) else { throw ProductStoreError.invalidRefloatConfiguration }
    try catalog.statement(
      """
      UPDATE refloat_configuration
      SET revision = revision + 1, name = ?, description = ?, values_json = ?, updated_at_ms = ?
      WHERE configuration_id = ? AND revision = ?
      """,
      values: [
        .text(name), description.map(SQLiteValue.text) ?? .null, .text(valuesJSON),
        .integer(Int64(at.timeIntervalSince1970 * 1_000)), .text(configurationID),
        .integer(Int64(expectedRevision)),
      ]
    )
    guard try catalog.scalarInt("SELECT changes()") == 1 else {
      throw ProductStoreError.refloatConfigurationRevisionConflict(currentRevision: current.revision)
    }
    return try refloatConfigurationLocked(configurationID: configurationID)
  }

  public func deleteRefloatConfiguration(configurationID: String, expectedRevision: Int) throws {
    lock.lock()
    defer { lock.unlock() }
    let current = try refloatConfigurationLocked(configurationID: configurationID)
    guard current.revision == expectedRevision else {
      throw ProductStoreError.refloatConfigurationRevisionConflict(currentRevision: current.revision)
    }
    try catalog.statement(
      "DELETE FROM refloat_configuration WHERE configuration_id = ? AND revision = ?",
      values: [.text(configurationID), .integer(Int64(expectedRevision))]
    )
  }

  private func refloatConfigurationLocked(configurationID: String) throws -> RefloatConfigurationRecord {
    guard !configurationID.isEmpty, configurationID.utf8.count <= 128,
          let record = try catalog.query(
            """
            SELECT configuration_id, revision, name, description, package_name, package_version,
                   schema_fingerprint, wire_signature, values_json, created_at_ms, updated_at_ms
            FROM refloat_configuration WHERE configuration_id = ?
            """,
            values: [.text(configurationID)], map: Self.refloatConfigurationRecord
          ).first
    else { throw ProductStoreError.refloatConfigurationNotFound }
    return record
  }

  private static func refloatConfigurationRecord(_ statement: OpaquePointer) -> RefloatConfigurationRecord {
    .init(
      configurationID: columnText(statement, 0)!, revision: Int(sqlite3_column_int64(statement, 1)),
      name: columnText(statement, 2)!, description: columnText(statement, 3),
      identity: .init(
        packageName: columnText(statement, 4)!, packageVersion: columnText(statement, 5)!,
        schemaFingerprint: columnText(statement, 6)!, wireSignature: UInt32(sqlite3_column_int64(statement, 7))
      ),
      valuesJSON: columnText(statement, 8)!,
      createdAt: Date(timeIntervalSince1970: Double(sqlite3_column_int64(statement, 9)) / 1_000),
      updatedAt: Date(timeIntervalSince1970: Double(sqlite3_column_int64(statement, 10)) / 1_000)
    )
  }

  private static func validRefloatIdentity(_ identity: RefloatConfigurationIdentityRecord) -> Bool {
    identity.packageName == "refloat" && !identity.packageVersion.isEmpty && identity.packageVersion.utf8
      .count <= 128 &&
      identity.schemaFingerprint.range(of: "^[0-9a-f]{64}$", options: .regularExpression) != nil
  }

  private static func validRefloatConfiguration(
    name: String, description: String?, identity: RefloatConfigurationIdentityRecord, valuesJSON: String
  ) -> Bool {
    guard !name.isEmpty, name.utf8.count <= 128, description?.utf8.count ?? 0 <= 1_024,
          validRefloatIdentity(identity), valuesJSON.utf8.count <= 48 * 1_024,
          let object = try? JSONSerialization.jsonObject(with: Data(valuesJSON.utf8)) as? [String: Any],
          !object.isEmpty, object.count <= RefloatConfigurationLimits.fields
    else { return false }
    return true
  }

  public func deleteSession(sessionID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    try deleteSessionLocked(sessionID: sessionID)
  }

  private func deleteSessionLocked(sessionID: String) throws {
    if current?.sessionID.value == sessionID { throw SessionStoreError.sessionActive }
    guard let sessionRow = try catalog.query(
      "SELECT file_name, state, pending_delete FROM session_catalog WHERE session_id = ?",
      values: [.text(sessionID)],
      map: {
        (Self.columnText($0, 0)!, Self.columnText($0, 1)!, sqlite3_column_int($0, 2) != 0)
      }
    ).first else {
      return
    }
    if sessionRow.1 == "active" { throw SessionStoreError.sessionActive }
    if sessionRow.2 { return }
    try catalog.statement(
      "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = ?",
      values: [.text(sessionID)]
    )
    // Existing readers retain their WAL snapshot. New readers observe the
    // pending flag under the same lock and are rejected until cancellation or
    // the final release completes the deletion.
    if activeReadLeases[sessionID, default: 0] > 0 { return }
    try finalizePendingDeleteLocked(sessionID: sessionID, fileName: sessionRow.0)
  }

  private func finalizePendingDeleteLocked(sessionID: String, fileName: String) throws {
    guard activeReadLeases[sessionID, default: 0] == 0 else { return }
    guard deletingSessions.insert(sessionID).inserted else { return }
    defer { deletingSessions.remove(sessionID) }
    try removeSessionFiles(fileName: fileName)
    try catalog.execute("BEGIN IMMEDIATE")
    do {
      try catalog.statement(
        "DELETE FROM session_catalog WHERE session_id = ?",
        values: [.text(sessionID)]
      )
      try catalog.execute("COMMIT")
    } catch {
      try? catalog.execute("ROLLBACK")
      throw error
    }
  }

  /// Restores a pending deletion while its file still exists. The operation
  /// identifier is the session identifier for the native synchronous delete
  /// API; no token or path is exposed to the WebView.
  public func cancelPendingDelete(operationID: String) throws -> Bool {
    lock.lock()
    defer { lock.unlock() }
    guard !deletingSessions.contains(operationID) else { return false }
    guard let row = try catalog.query(
      "SELECT file_name, pending_delete FROM session_catalog WHERE session_id = ?",
      values: [.text(operationID)],
      map: { (Self.columnText($0, 0)!, sqlite3_column_int($0, 1) != 0) }
    ).first, row.1 else { return false }
    let file = root.appending(path: row.0)
    guard FileManager.default.fileExists(atPath: file.path) else { return false }
    try catalog.statement(
      "UPDATE session_catalog SET pending_delete = 0 WHERE session_id = ? AND pending_delete = 1",
      values: [.text(operationID)]
    )
    return try catalog.scalarInt("SELECT changes()") == 1
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
      let includeGpsLocation = try settingsLocked().includeGpsLocationWhenExporting
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
          values: [.text(Self.jsonString(manifest))]
        )
        try archive.execute("PRAGMA wal_checkpoint(TRUNCATE)")
      } else {
        FileManager.default.createFile(atPath: output.path, contents: nil)
        let handle = try FileHandle(forWritingTo: output)
        defer { try? handle.close() }
        try handle.write(contentsOf: Data((
          """
          sequence,observed_at_ms,slow_observed_at_ms,elapsed_ms,ride_state,package_state,package_mode,footpad_state,stop_condition,setpoint_adjustment,wheelslip,darkride,charging,alert_reason,firmware_fault,pack_voltage_v,duty_cycle,controller_temp_c,motor_temp_c,motor_current_a,battery_current_a,erpm,speed_kph,pitchDeg,balancePitchDeg,rollDeg,directionalMotorCurrentA,filteredMotorCurrentA,requestedBalanceCurrentA,boosterCurrentA,fieldWeakeningCurrentA,footpadAdc1V,footpadAdc2V,remoteInput,setpointDeg,atrSetpointDeg,brakeTiltSetpointDeg,torqueTiltSetpointDeg,turnTiltSetpointDeg,remoteSetpointDeg,atrAccelerationDifferenceErpmPerSecond,atrSpeedBoost,distanceM,odometerM,ampHoursDrawnAh,ampHoursChargedAh,wattHoursDrawnWh,wattHoursChargedWh,batteryLevel,chargingCurrentA,chargingVoltageV,extension_json\(includeGpsLocation ?
            ",gpsAccuracy,gpsLongitude,gpsLatitude" : "")
          """ + "\n"
        ).utf8))
        try database.forEachRow(
          """
          SELECT sequence, observed_at_ms, slow_observed_at_ms, monotonic_elapsed_ms,
                 ride_state, package_state, package_mode, footpad_state, stop_condition,
                 setpoint_adjustment, wheelslip, darkride, charging, alert_reason,
                 firmware_fault, raw_pack_voltage_v, duty_cycle, controller_temp_c,
                 motor_temp_c, motor_current_a, battery_current_a, erpm, speed_kph,
                 pitch_deg, balance_pitch_deg, roll_deg, directional_motor_current_a,
                 filtered_motor_current_a, requested_balance_current_a, booster_current_a,
                 field_weakening_current_a, footpad_adc_1_v, footpad_adc_2_v, remote_input,
                 setpoint_deg, atr_setpoint_deg, brake_tilt_setpoint_deg, torque_tilt_setpoint_deg,
                 turn_tilt_setpoint_deg, remote_setpoint_deg,
                 atr_acceleration_difference_erpm_per_second, atr_speed_boost, distance_m,
                 odometer_m, amp_hours_drawn_ah, amp_hours_charged_ah, watt_hours_drawn_wh,
                 watt_hours_charged_wh, battery_level, charging_current_a, charging_voltage_v,
                 extension_json
          FROM sample ORDER BY sequence
          """
        ) { statement in
          var columns: [String] = []
          let textColumns: Set = [4, 5, 6, 7, 8, 9, 13, 14, 51]
          for index in 0 ..< 52 {
            if sqlite3_column_type(statement, Int32(index)) == SQLITE_NULL {
              columns.append("")
            } else if textColumns.contains(index) {
              columns.append(Self.csv(Self.columnText(statement, Int32(index)) ?? ""))
            } else {
              columns.append(String(cString: sqlite3_column_text(statement, Int32(index))))
            }
          }
          if includeGpsLocation {
            try columns.append(contentsOf: nearestLocationCSVColumns(
              database: database,
              elapsedMs: sqlite3_column_int64(statement, 3)
            ))
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

  private func nearestLocationCSVColumns(database: Database, elapsedMs: Int64) throws -> [String] {
    let tolerance = Self.csvLocationToleranceMs
    return try database.query(
      """
      SELECT accuracy_m, longitude, latitude
      FROM location
      WHERE monotonic_elapsed_ms BETWEEN ? AND ?
      ORDER BY ABS(monotonic_elapsed_ms - ?) ASC, location_sequence ASC
      LIMIT 1
      """,
      values: [.integer(elapsedMs - tolerance), .integer(elapsedMs + tolerance), .integer(elapsedMs)],
      map: { statement in
        [
          sqlite3_column_type(statement, 0) == SQLITE_NULL ? "" : String(sqlite3_column_double(statement, 0)),
          String(sqlite3_column_double(statement, 1)),
          String(sqlite3_column_double(statement, 2)),
        ]
      }
    ).first ?? ["", "", ""]
  }

  @discardableResult
  public func importSessionArchive(data: Data) throws -> String {
    lock.lock()
    defer { lock.unlock() }
    guard data.count <= 128 * 1024 * 1024 else { throw SessionStoreError.invalidArchive }
    if current != nil { throw SessionStoreError.sessionActive }

    let importID = UUID().uuidString
    let sessionID = importID
    let jobID = UUID().uuidString
    let fileName = "session-\(sessionID).sqlite"
    let temporary = root.appending(path: "import-\(importID).floatview.tmp")
    let destination = root.appending(path: fileName)
    func removeSQLiteSidecars(_ file: URL) {
      for suffix in ["-wal", "-shm"] {
        try? FileManager.default.removeItem(at: URL(fileURLWithPath: file.path + suffix))
      }
    }
    func removeSQLiteFiles(_ file: URL) {
      removeSQLiteSidecars(file)
      try? FileManager.default.removeItem(at: file)
    }
    do {
      try data.write(to: temporary, options: .atomic)
      let archive = try Database(url: temporary)
      defer { archive.close() }
      let formatVersion = try archive.query(
        "SELECT value FROM float_view_archive_metadata WHERE key = 'format_version'",
        map: { Self.columnText($0, 0) }
      ).first ?? nil
      let manifestText = try archive.query(
        "SELECT value FROM float_view_archive_metadata WHERE key = 'session'",
        map: { Self.columnText($0, 0) }
      ).first ?? nil
      guard formatVersion == "1",
            let manifestText,
            let manifest = try JSONSerialization.jsonObject(with: Data(manifestText.utf8)) as? [String: Any],
            try archive.query("PRAGMA integrity_check", map: { Self.columnText($0, 0) ?? "" }).first == "ok",
            try archive.scalarInt("SELECT EXISTS(SELECT 1 FROM commit_marker WHERE singleton = 1)") == 1,
            try archive.scalarInt("SELECT COUNT(*) FROM sample") > 0
      else { throw SessionStoreError.invalidArchive }

      func text(_ key: String, fallback: String) -> String {
        guard let value = manifest[key] as? String, !value.isEmpty else { return fallback }
        return value
      }
      func integer(_ key: String, fallback: Int64) -> Int64 {
        guard let value = manifest[key] as? NSNumber else { return fallback }
        return value.int64Value
      }
      func optionalInteger(_ key: String) -> SQLiteValue {
        guard let value = manifest[key] as? NSNumber else { return .null }
        return .integer(value.int64Value)
      }
      func json(_ key: String) throws -> String {
        guard let value = manifest[key] as? [String: Any] else { return "{}" }
        return try Self.jsonString(value)
      }

      guard let rawDeviceHistoryID = manifest["deviceHistoryId"] as? String,
            let deviceHistoryID = Self.normalizedDeviceHistoryID(rawDeviceHistoryID)
      else { throw SessionStoreError.invalidArchive }
      let deviceSnapshot = try json("deviceSnapshot")
      let profileSnapshot = try json("boardProfileSnapshot")
      let displayName = (manifest["deviceSnapshot"] as? [String: Any])?["displayName"] as? String
      let startedAtMs = integer("startedAtMs", fallback: Int64(Date().timeIntervalSince1970 * 1_000))
      let requestedIdleMs = max(1, integer("requestedIdleMs", fallback: 1_000))
      let effectiveIdleMs = max(1, integer("effectiveIdleMs", fallback: requestedIdleMs))
      let requestedRidingMs = max(1, integer("requestedRidingMs", fallback: 1_000))
      let effectiveRidingMs = max(1, integer("effectiveRidingMs", fallback: requestedRidingMs))
      let closeReason = (manifest["closeReason"] as? String).map(SQLiteValue.text) ?? .null

      archive.close()
      removeSQLiteSidecars(temporary)
      try FileManager.default.moveItem(at: temporary, to: destination)
      try catalog.execute("BEGIN IMMEDIATE")
      do {
        _ = try ensureImportedSavedDeviceLocked(
          deviceHistoryID: deviceHistoryID,
          displayName: displayName,
          profileSnapshotJSON: profileSnapshot,
          at: Date(timeIntervalSince1970: Double(startedAtMs) / 1_000)
        )
        try catalog.statement(
          """
          INSERT INTO session_catalog(
            session_id, job_id, device_history_id, device_snapshot_json,
            profile_snapshot_json, profile_revision, file_name, started_at_ms,
            ended_at_ms, state, integrity, requested_idle_ms, effective_idle_ms,
            requested_riding_ms, effective_riding_ms, close_reason
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'completed', 'complete', ?, ?, ?, ?, ?)
          """,
          values: [
            .text(sessionID), .text(jobID), .text(deviceHistoryID), .text(deviceSnapshot),
            .text(profileSnapshot), .integer(integer("boardProfileRevision", fallback: 0)),
            .text(fileName), .integer(startedAtMs), optionalInteger("endedAtMs"),
            .integer(requestedIdleMs), .integer(effectiveIdleMs),
            .integer(requestedRidingMs), .integer(effectiveRidingMs), closeReason,
          ]
        )
        try catalog.execute("COMMIT")
      } catch {
        try? catalog.execute("ROLLBACK")
        throw error
      }
      try reconcileSessionCatalogLocked(sessionID: sessionID, jobID: jobID, fileName: fileName)
      return sessionID
    } catch let error as SessionStoreError {
      removeSQLiteFiles(temporary)
      removeSQLiteFiles(destination)
      throw error
    } catch {
      removeSQLiteFiles(temporary)
      removeSQLiteFiles(destination)
      throw SessionStoreError.invalidArchive
    }
  }

  @discardableResult
  public func recoverInterrupted() throws -> Int {
    lock.lock()
    defer { lock.unlock() }
    let minimumDistance = try settingsLocked().minimumSessionDistance
    let activeSessions = try catalog.query(
      "SELECT session_id, file_name, COALESCE(distance_m, 0) FROM session_catalog WHERE state = 'active'",
      map: {
        (Self.columnText($0, 0)!, Self.columnText($0, 1)!, sqlite3_column_double($0, 2))
      }
    )
    var recovered = 0
    for (sessionID, fileName, distance) in activeSessions {
      let enteredRiding = (try? sessionEnteredRiding(fileName: fileName)) ?? false
      try catalog.statement(
        """
        UPDATE session_catalog SET state = 'interrupted', integrity = 'recovered',
        close_reason = 'process_interruption' WHERE session_id = ? AND state = 'active'
        """,
        values: [.text(sessionID)]
      )
      if enteredRiding && distance >= minimumDistance {
        recovered += 1
      } else {
        try deleteSessionLocked(sessionID: sessionID)
      }
    }
    return recovered
  }

  private func sessionEnteredRiding(fileName: String) throws -> Bool {
    let database = try Database(url: root.appending(path: fileName))
    return try database.scalarInt(
      "SELECT EXISTS(SELECT 1 FROM sample WHERE ride_state = 'riding')"
    ) == 1
  }

  private func settingsLocked() throws -> SettingsRecord {
    if let stored = try catalog.query(
      "SELECT revision, json FROM settings WHERE singleton = 1",
      map: { (Int(sqlite3_column_int64($0, 0)), Self.columnText($0, 1)!) }
    ).first,
      var values = (try? JSONSerialization.jsonObject(with: Data(stored.1.utf8)))
      as? [String: Any]
    {
      if values["minimumSessionDistance"] == nil {
        values["minimumSessionDistance"] = SettingsRecord().minimumSessionDistance
      }
      if values["historyGapThresholdSeconds"] == nil {
        values["historyGapThresholdSeconds"] = NSNull()
      }
      if values["locationRecordingEnabled"] == nil {
        values["locationRecordingEnabled"] = false
      }
      if values["includeGpsLocationWhenExporting"] == nil {
        values["includeGpsLocationWhenExporting"] = false
      }
      if let value = try? Self.validatedSettings(values, revision: stored.0) {
        return value
      }
    }
    let value = SettingsRecord()
    let json = try String(data: JSONEncoder().encode(value), encoding: .utf8)!
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

  private func updateCatalogSummaryLocked(
    record: SessionRecord,
    samples _: [TelemetrySample]
  ) throws {
    guard let session else { return }
    let marker = try session.query(
      """
      SELECT committed_sample_sequence, committed_event_sequence,
             recorded_sample_count, stored_sample_count, committed_event_count,
             committed_elapsed_ms, riding_elapsed_ms, distance_m,
             maximum_speed_kph, maximum_duty_cycle,
             maximum_speed_source_sequence, maximum_duty_source_sequence,
             last_speed_kph, committed_location_sequence,
             committed_location_elapsed_ms, recorded_location_count, stored_location_count, last_location_status
      FROM commit_marker WHERE singleton = 1
      """,
      map: { statement in
        (
          committedSequence: sqlite3_column_int64(statement, 0),
          eventSequence: sqlite3_column_int64(statement, 1),
          recordedCount: sqlite3_column_int64(statement, 2),
          storedCount: sqlite3_column_int64(statement, 3),
          eventCount: sqlite3_column_int64(statement, 4),
          elapsed: sqlite3_column_int64(statement, 5),
          riding: sqlite3_column_int64(statement, 6),
          distance: Self.columnDouble(statement, 7),
          maxSpeed: Self.columnDouble(statement, 8),
          maxDuty: Self.columnDouble(statement, 9),
          maxSpeedSequence: sqlite3_column_type(statement, 10) == SQLITE_NULL ? nil : sqlite3_column_int64(
            statement,
            10
          ),
          maxDutySequence: sqlite3_column_type(statement, 11) == SQLITE_NULL ? nil : sqlite3_column_int64(
            statement,
            11
          ),
          lastSpeed: Self.columnDouble(statement, 12),
          locationSequence: sqlite3_column_int64(statement, 13),
          locationElapsed: sqlite3_column_int64(statement, 14),
          locationCount: sqlite3_column_int64(statement, 15),
          storedLocationCount: sqlite3_column_int64(statement, 16),
          locationStatus: Self.columnText(statement, 17) ?? "disabled"
        )
      }
    ).first
    guard let marker else { return }
    try catalog.statement(
      """
      UPDATE session_catalog SET committed_sequence = ?, committed_event_sequence = ?,
        committed_elapsed_ms = ?, stored_sample_count = ?, sample_count = ?,
        riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
        max_speed_mps = ?, max_duty_cycle = ?, max_speed_source_sequence = ?,
        max_duty_source_sequence = ?, last_sample_elapsed_ms = ?,
        last_speed_mps = ?, committed_location_sequence = ?,
        committed_location_elapsed_ms = ?, location_count = ?, recorded_location_count = ?, stored_location_count = ?, location_status = ? WHERE job_id = ?
      """,
      values: [
        .integer(marker.committedSequence), .integer(marker.eventSequence),
        .integer(marker.elapsed), .integer(marker.storedCount), .integer(marker.recordedCount),
        .integer(marker.riding), .integer(marker.elapsed), marker.distance.map(SQLiteValue.real) ?? .null,
        marker.maxSpeed.map { .real(abs($0) / 3.6) } ?? .null,
        marker.maxDuty.map { .real(abs($0)) } ?? .null,
        marker.maxSpeedSequence.map(SQLiteValue.integer) ?? .null,
        marker.maxDutySequence.map(SQLiteValue.integer) ?? .null,
        .integer(marker.elapsed), marker.lastSpeed.map { .real(abs($0) / 3.6) } ?? .null,
        .integer(marker.locationSequence), .integer(marker.locationElapsed), .integer(marker.locationCount),
        .integer(marker.locationCount), .integer(marker.storedLocationCount), .text(marker.locationStatus),
        .text(record.jobID.value),
      ]
    )
  }

  private func markCatalogDirtyLocked(jobID: String) throws {
    try catalog.statement(
      "UPDATE session_catalog SET integrity = 'dirty' WHERE job_id = ?",
      values: [.text(jobID)]
    )
  }

  /// Rebuild a catalog row from the session's durable marker and canonical
  /// rows. This is intentionally idempotent: it may run during startup, before
  /// an append, or after a catalog write failed following a committed batch.
  private func reconcileSessionCatalogLocked(
    sessionID: String,
    jobID: String,
    fileName: String
  ) throws {
    let file = root.appending(path: fileName)
    guard FileManager.default.fileExists(atPath: file.path) else {
      try? markCatalogDirtyLocked(jobID: jobID)
      return
    }
    let database = try Database(url: file)
    let check = try database.query("PRAGMA integrity_check", map: { Self.columnText($0, 0) ?? "" }).first
    guard check == "ok" else {
      try? markCatalogDirtyLocked(jobID: jobID)
      return
    }
    let marker = try database.query(
      """
      SELECT committed_sample_sequence, committed_event_sequence,
             recorded_sample_count, stored_sample_count, committed_event_count,
             committed_elapsed_ms, history_revision, committed_at_ms,
             committed_location_sequence, committed_location_elapsed_ms,
             recorded_location_count, stored_location_count, last_location_status
      FROM commit_marker WHERE singleton = 1
      """,
      map: { statement in
        (
          sampleSequence: sqlite3_column_int64(statement, 0),
          eventSequence: sqlite3_column_int64(statement, 1),
          recordedCount: sqlite3_column_int64(statement, 2),
          storedCount: sqlite3_column_int64(statement, 3),
          eventCount: sqlite3_column_int64(statement, 4),
          elapsed: sqlite3_column_int64(statement, 5),
          revision: sqlite3_column_int64(statement, 6),
          committedAt: sqlite3_column_int64(statement, 7),
          locationSequence: sqlite3_column_int64(statement, 8),
          locationElapsed: sqlite3_column_int64(statement, 9),
          locationCount: sqlite3_column_int64(statement, 10),
          storedLocationCount: sqlite3_column_int64(statement, 11),
          locationStatus: Self.columnText(statement, 12) ?? "disabled"
        )
      }
    ).first
    guard let marker else { throw SQLiteLogStoreError.sqlite("Missing session commit marker") }

    let rows: [(sequence: Int64, elapsed: Int64, riding: Bool, speed: Double?, duty: Double?)] = try database.query(
      """
      SELECT sequence, monotonic_elapsed_ms, ride_state, speed_kph, duty_cycle
      FROM sample ORDER BY monotonic_elapsed_ms ASC, sequence ASC
      """
    ) { statement in
      (
        sequence: sqlite3_column_int64(statement, 0),
        elapsed: sqlite3_column_int64(statement, 1),
        riding: Self.columnText(statement, 2) == RideState.riding.rawValue,
        speed: Self.columnDouble(statement, 3).map { abs($0) / 3.6 },
        duty: Self.columnDouble(statement, 4).map(abs)
      )
    }
    var ridingElapsed: Int64 = 0
    var distance = 0.0
    var maxSpeed: Double?
    var maxSpeedSequence: Int64?
    var maxDuty: Double?
    var maxDutySequence: Int64?
    var previousElapsed: Int64?
    var previousSpeed: Double?
    for row in rows {
      if let previousElapsed {
        let delta = max(0, row.elapsed - previousElapsed)
        if row.riding { ridingElapsed += delta }
        if let speed = row.speed {
          distance += ((previousSpeed ?? speed) + speed) * 0.5 * Double(delta) / 1_000
        }
      }
      if let speed = row.speed, maxSpeed == nil || speed > maxSpeed! {
        maxSpeed = speed
        maxSpeedSequence = row.sequence
      }
      if let duty = row.duty, maxDuty == nil || duty > maxDuty! {
        maxDuty = duty
        maxDutySequence = row.sequence
      }
      previousElapsed = row.elapsed
      previousSpeed = row.speed
    }
    let storedCount = Int64(rows.count)
    let actualEventCount = try Int64(database.scalarInt("SELECT COUNT(*) FROM session_event"))
    let actualEventSequence = try Int64(database.scalarInt("SELECT COALESCE(MAX(sequence), 0) FROM session_event"))
    let actualElapsed = rows.last?.elapsed ?? 0
    let lastSpeed = rows.last?.speed
    let lastElapsed = rows.last?.elapsed
    try catalog.statement(
      """
      UPDATE session_catalog SET integrity = CASE WHEN integrity = 'dirty' THEN 'complete' ELSE integrity END,
        committed_sequence = ?, committed_event_sequence = ?,
        sample_count = ?, stored_sample_count = ?, event_count = ?,
        committed_elapsed_ms = ?, history_revision = ?,
        riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
        max_speed_mps = ?, max_duty_cycle = ?,
        max_speed_source_sequence = ?, max_duty_source_sequence = ?,
        last_sample_elapsed_ms = ?, last_speed_mps = ?
        , committed_location_sequence = ?, committed_location_elapsed_ms = ?,
        location_count = ?, recorded_location_count = ?, stored_location_count = ?, location_status = ?
      WHERE session_id = ? AND job_id = ?
      """,
      values: [
        .integer(marker.sampleSequence), .integer(max(marker.eventSequence, actualEventSequence)),
        .integer(marker.recordedCount), .integer(max(marker.storedCount, storedCount)),
        .integer(max(marker.eventCount, actualEventCount)), .integer(max(marker.elapsed, actualElapsed)),
        .integer(marker.revision), .integer(ridingElapsed), .integer(actualElapsed), .real(distance),
        maxSpeed.map(SQLiteValue.real) ?? .null, maxDuty.map(SQLiteValue.real) ?? .null,
        maxSpeedSequence.map(SQLiteValue.integer) ?? .null, maxDutySequence.map(SQLiteValue.integer) ?? .null,
        lastElapsed.map(SQLiteValue.integer) ?? .null, lastSpeed.map(SQLiteValue.real) ?? .null,
        .integer(marker.locationSequence), .integer(marker.locationElapsed), .integer(marker.locationCount),
        .integer(marker.locationCount), .integer(marker.storedLocationCount), .text(marker.locationStatus),
        .text(sessionID), .text(jobID),
      ]
    )
  }

  private func reconcileCatalogLocked() throws {
    let rows = try catalog.query(
      "SELECT session_id, job_id, file_name FROM session_catalog WHERE pending_delete = 0",
      map: { (Self.columnText($0, 0)!, Self.columnText($0, 1)!, Self.columnText($0, 2)!) }
    )
    for (sessionID, jobID, fileName) in rows {
      do {
        try reconcileSessionCatalogLocked(sessionID: sessionID, jobID: jobID, fileName: fileName)
      } catch {
        try? markCatalogDirtyLocked(jobID: jobID)
      }
    }
  }

  private func reconcileDirtyCatalogLocked(jobID: String) throws {
    let rows = try catalog.query(
      """
      SELECT session_id, file_name FROM session_catalog
      WHERE job_id = ? AND pending_delete = 0 AND integrity = 'dirty'
      """,
      values: [.text(jobID)],
      map: { (Self.columnText($0, 0)!, Self.columnText($0, 1)!) }
    )
    for (sessionID, fileName) in rows {
      try reconcileSessionCatalogLocked(sessionID: sessionID, jobID: jobID, fileName: fileName)
    }
  }

  private func reconcileDirtyCatalogLocked() throws {
    let rows = try catalog.query(
      """
      SELECT session_id, job_id, file_name FROM session_catalog
      WHERE pending_delete = 0 AND integrity = 'dirty'
      """,
      map: { (Self.columnText($0, 0)!, Self.columnText($0, 1)!, Self.columnText($0, 2)!) }
    )
    for (sessionID, jobID, fileName) in rows {
      try reconcileSessionCatalogLocked(sessionID: sessionID, jobID: jobID, fileName: fileName)
    }
  }

  private func openSessionLocked(sessionID: String) throws -> Database {
    if let jobID = try catalog.query(
      "SELECT job_id FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
      values: [.text(sessionID)],
      map: { Self.columnText($0, 0)! }
    ).first {
      try reconcileDirtyCatalogLocked(jobID: jobID)
    }
    guard let fileName = try catalog.query(
      "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
      values: [.text(sessionID)],
      map: { Self.columnText($0, 0)! }
    ).first else { throw SessionStoreError.sessionNotFound }
    return try Database(url: root.appending(path: fileName))
  }

  private struct ReadLease {
    let sessionID: String
    let fileURL: URL
  }

  /// Resolve a historical file under the short store lock, then let the
  /// caller scan it on a dedicated query-only WAL connection. The lease keeps
  /// deletion from removing a file while a reader is still opening or using
  /// it, and the deferred transaction gives the request one stable snapshot.
  private func acquireReadLeaseLocked(sessionID: String) throws -> ReadLease {
    guard !maintenanceSessions.contains(sessionID) else {
      throw SessionStoreError.maintenanceInProgress
    }
    try reconcileDirtyCatalogLocked()
    guard let fileName = try catalog.query(
      """
      SELECT file_name FROM session_catalog
      WHERE session_id = ? AND pending_delete = 0
      """,
      values: [.text(sessionID)],
      map: { Self.columnText($0, 0)! }
    ).first else { throw SessionStoreError.sessionNotFound }
    activeReadLeases[sessionID, default: 0] += 1
    return ReadLease(sessionID: sessionID, fileURL: root.appending(path: fileName))
  }

  private func releaseReadLease(_ lease: ReadLease) {
    lock.lock()
    defer { lock.unlock() }
    guard let count = activeReadLeases[lease.sessionID] else { return }
    if count <= 1 {
      activeReadLeases.removeValue(forKey: lease.sessionID)
      // Deletion is deliberately best-effort here: if the file operation is
      // interrupted the durable pending marker is retried at the next store
      // open, and the session remains hidden from new readers meanwhile.
      if let fileName = try? catalog.query(
        "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 1",
        values: [.text(lease.sessionID)],
        map: { Self.columnText($0, 0)! }
      ).first {
        try? finalizePendingDeleteLocked(sessionID: lease.sessionID, fileName: fileName)
      }
    } else {
      activeReadLeases[lease.sessionID] = count - 1
    }
  }

  private func withReadLease<T>(
    sessionID: String,
    body: (Database, SQLiteCancellation) throws -> T
  ) throws -> T {
    // Keep historical work bounded independently from the loopback request
    // executor. The slot is acquired before the lease so a queued request
    // cannot make deletion appear blocked while it waits for a reader slot.
    historicalReadSlots.wait()
    defer { historicalReadSlots.signal() }
    lock.lock()
    let lease: ReadLease
    do {
      lease = try acquireReadLeaseLocked(sessionID: sessionID)
    } catch {
      lock.unlock()
      throw error
    }
    lock.unlock()
    defer { releaseReadLease(lease) }

    let database = try Database(url: lease.fileURL, readOnly: true)
    let cancellation = SQLiteCancellation()
    database.installCancellation(cancellation)
    let requestCancellation = SessionReadCancellationContext.current
    lock.lock()
    historicalConnections[sessionID, default: []].append(database)
    historicalCancellations[sessionID, default: []].append(cancellation)
    lock.unlock()
    let requestRegistration = requestCancellation?.register { database.interrupt() }
    defer {
      requestCancellation?.unregister(requestRegistration)
      lock.lock()
      historicalConnections[sessionID]?.removeAll { $0 === database }
      if historicalConnections[sessionID]?.isEmpty == true {
        historicalConnections.removeValue(forKey: sessionID)
      }
      historicalCancellations[sessionID]?.removeAll { $0 === cancellation }
      if historicalCancellations[sessionID]?.isEmpty == true {
        historicalCancellations.removeValue(forKey: sessionID)
      }
      lock.unlock()
    }
    onHistoricalReadRegisteredForTesting?(requestCancellation)
    try database.execute("PRAGMA query_only = ON; PRAGMA busy_timeout = 1000; BEGIN DEFERRED")
    defer { try? database.execute("ROLLBACK") }
    return try body(database, cancellation)
  }

  private func readSeries(
    sessionID: String,
    start: Int64,
    end: Int64,
    fields: [String],
    maxPoints: Int,
    lossless: Bool,
    maximumSequence: Int64?
  ) throws -> [SessionSeriesPoint] {
    let uniqueFields = Array(NSOrderedSet(array: fields)) as? [String] ?? fields
    guard !uniqueFields.isEmpty else { throw SessionStoreError.invalidRange }
    let columns = try uniqueFields.map { field -> String in
      if let column = Self.seriesColumns[field] { return column }
      guard !field.isEmpty, field.lengthOfBytes(using: .utf8) <= 64 else {
        throw SessionStoreError.unsupportedField(field)
      }
      let encoded = try JSONSerialization.data(
        withJSONObject: field,
        options: [.fragmentsAllowed]
      )
      let jsonKey = String(decoding: encoded, as: UTF8.self)
      let path = ("$." + jsonKey).replacingOccurrences(of: "'", with: "''")
      return "json_extract(extension_json, '\(path)')"
    }
    return try withReadLease(sessionID: sessionID) { database, _ in
      let selected = columns.enumerated().map { index, column in
        "\(column) AS value_\(index)"
      }.joined(separator: ", ")
      let sql: String
      var values: [SQLiteValue] = [.integer(start), .integer(end)]
      let sequencePredicate: String
      if let maximumSequence {
        sequencePredicate = " AND sequence <= ?"
        values.append(.integer(maximumSequence))
      } else {
        sequencePredicate = ""
      }
      if lossless {
        sql = """
        SELECT sequence, monotonic_elapsed_ms, ride_state, \(selected) FROM sample
        WHERE monotonic_elapsed_ms BETWEEN ? AND ?\(sequencePredicate)
        ORDER BY monotonic_elapsed_ms, sequence LIMIT ?
        """
        values.append(.integer(Int64(maxPoints)))
      } else {
        sql = """
        SELECT sequence, monotonic_elapsed_ms, ride_state, \(selected) FROM sample
        WHERE monotonic_elapsed_ms BETWEEN ? AND ?\(sequencePredicate)
        ORDER BY monotonic_elapsed_ms, sequence LIMIT ?
        """
        values.append(.integer(Int64(maxPoints)))
      }
      return try database.query(sql, values: values) { statement in
        var result: [String: Double?] = [:]
        for (index, field) in uniqueFields.enumerated() {
          result[field] = .some(Self.columnDouble(statement, Int32(index + 3)))
        }
        return SessionSeriesPoint(
          elapsedMs: sqlite3_column_int64(statement, 1),
          values: result,
          sequence: sqlite3_column_int64(statement, 0),
          rideState: Self.columnText(statement, 2)
        )
      }
    }
  }

  private static let seriesColumns = [
    "packVoltageV": "raw_pack_voltage_v",
    "dutyCycle": "duty_cycle",
    "controllerTempC": "controller_temp_c",
    "motorTempC": "motor_temp_c",
    "motorCurrentA": "motor_current_a",
    "batteryCurrentA": "battery_current_a",
    "erpm": "erpm",
    "speedKph": "speed_kph",
    "pitchDeg": "pitch_deg",
    "balancePitchDeg": "balance_pitch_deg",
    "rollDeg": "roll_deg",
    "directionalMotorCurrentA": "directional_motor_current_a",
    "filteredMotorCurrentA": "filtered_motor_current_a",
    "requestedBalanceCurrentA": "requested_balance_current_a",
    "boosterCurrentA": "booster_current_a",
    "fieldWeakeningCurrentA": "field_weakening_current_a",
    "footpadAdc1V": "footpad_adc_1_v",
    "footpadAdc2V": "footpad_adc_2_v",
    "remoteInput": "remote_input",
    "setpointDeg": "setpoint_deg",
    "atrSetpointDeg": "atr_setpoint_deg",
    "brakeTiltSetpointDeg": "brake_tilt_setpoint_deg",
    "torqueTiltSetpointDeg": "torque_tilt_setpoint_deg",
    "turnTiltSetpointDeg": "turn_tilt_setpoint_deg",
    "remoteSetpointDeg": "remote_setpoint_deg",
    "atrAccelerationDifferenceErpmPerSecond":
      "atr_acceleration_difference_erpm_per_second",
    "atrSpeedBoost": "atr_speed_boost",
    "distanceM": "distance_m",
    "odometerM": "odometer_m",
    "ampHoursDrawnAh": "amp_hours_drawn_ah",
    "ampHoursChargedAh": "amp_hours_charged_ah",
    "wattHoursDrawnWh": "watt_hours_drawn_wh",
    "wattHoursChargedWh": "watt_hours_charged_wh",
    "batteryLevel": "battery_level",
    "chargingCurrentA": "charging_current_a",
    "chargingVoltageV": "charging_voltage_v",
  ]

  private static func numericValues(_ sample: TelemetrySample) -> [String: Double?] {
    [
      "packVoltageV": sample.packVoltageV, "dutyCycle": sample.dutyCycle,
      "controllerTempC": sample.controllerTempC, "motorTempC": sample.motorTempC,
      "motorCurrentA": sample.motorCurrentA, "batteryCurrentA": sample.batteryCurrentA,
      "erpm": sample.erpm, "speedKph": sample.speedKph, "pitchDeg": sample.pitchDeg,
      "balancePitchDeg": sample.balancePitchDeg, "rollDeg": sample.rollDeg,
      "directionalMotorCurrentA": sample.directionalMotorCurrentA,
      "filteredMotorCurrentA": sample.filteredMotorCurrentA,
      "requestedBalanceCurrentA": sample.requestedBalanceCurrentA,
      "boosterCurrentA": sample.boosterCurrentA, "fieldWeakeningCurrentA": sample.fieldWeakeningCurrentA,
      "footpadAdc1V": sample.footpadAdc1V, "footpadAdc2V": sample.footpadAdc2V,
      "remoteInput": sample.remoteInput, "setpointDeg": sample.setpointDeg,
      "atrSetpointDeg": sample.atrSetpointDeg, "brakeTiltSetpointDeg": sample.brakeTiltSetpointDeg,
      "torqueTiltSetpointDeg": sample.torqueTiltSetpointDeg, "turnTiltSetpointDeg": sample.turnTiltSetpointDeg,
      "remoteSetpointDeg": sample.remoteSetpointDeg,
      "atrAccelerationDifferenceErpmPerSecond": sample.atrAccelerationDifferenceErpmPerSecond,
      "atrSpeedBoost": sample.atrSpeedBoost, "distanceM": sample.distanceM,
      "odometerM": sample.odometerM, "ampHoursDrawnAh": sample.ampHoursDrawnAh,
      "ampHoursChargedAh": sample.ampHoursChargedAh, "wattHoursDrawnWh": sample.wattHoursDrawnWh,
      "wattHoursChargedWh": sample.wattHoursChargedWh, "batteryLevel": sample.batteryLevel,
      "chargingCurrentA": sample.chargingCurrentA, "chargingVoltageV": sample.chargingVoltageV,
    ]
  }

  private static func telemetryExtensionJSON(_ values: [String: Double]) throws -> String {
    guard values.count <= 32,
          values.allSatisfy({
            !$0.key.isEmpty && $0.key.lengthOfBytes(using: .utf8) <= 64 && $0.value.isFinite
          })
    else {
      throw SQLiteLogStoreError.sqlite("Telemetry extension fields exceed their bound")
    }
    let data = try JSONSerialization.data(withJSONObject: values, options: [.sortedKeys])
    guard data.count <= 16_384, let result = String(data: data, encoding: .utf8) else {
      throw SQLiteLogStoreError.sqlite("Telemetry extension JSON exceeds its bound")
    }
    return result
  }

  private static func transitionValues(_ sample: TelemetrySample) -> [(String, String)] {
    var values: [(String, String)] = []
    if let value = sample.packageState { values.append(("packageState", value.rawValue)) }
    if let value = sample.packageMode { values.append(("packageMode", value.rawValue)) }
    if let value = sample.footpadState { values.append(("footpadState", value.rawValue)) }
    if let value = sample.stopCondition { values.append(("stopCondition", value.rawValue)) }
    if let value = sample.setpointAdjustment {
      values.append(("setpointAdjustment", value.rawValue))
    }
    if let value = sample.wheelslip { values.append(("wheelslip", value ? "active" : "cleared")) }
    if let value = sample.darkride { values.append(("darkride", value ? "active" : "cleared")) }
    if let value = sample.charging { values.append(("charging", value ? "active" : "cleared")) }
    values.append(("alertReason", sample.alertReason?.rawValue ?? "none"))
    values.append(("firmwareFault", sample.firmwareFault?.rawValue ?? "none"))
    return values
  }

  private static func latestTransitionValues(in database: Database) throws -> [String: String] {
    try database.query(
      """
      SELECT package_state, package_mode, footpad_state, stop_condition,
             setpoint_adjustment, wheelslip, darkride, charging, alert_reason, firmware_fault
      FROM sample ORDER BY sequence DESC LIMIT 1
      """
    ) { statement in
      var values: [String: String] = [:]
      let textFields = [
        (0, "packageState"), (1, "packageMode"), (2, "footpadState"),
        (3, "stopCondition"), (4, "setpointAdjustment"),
      ]
      for (index, key) in textFields {
        if let value = columnText(statement, Int32(index)) { values[key] = value }
      }
      for (index, key) in [(5, "wheelslip"), (6, "darkride"), (7, "charging")] {
        if sqlite3_column_type(statement, Int32(index)) != SQLITE_NULL {
          values[key] = sqlite3_column_int64(statement, Int32(index)) == 0 ? "cleared" : "active"
        }
      }
      values["alertReason"] = columnText(statement, 8) ?? "none"
      values["firmwareFault"] = columnText(statement, 9) ?? "none"
      return values
    }.first ?? [:]
  }

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
    "minimumSessionDistance",
    "themePreference",
    "unitPreference",
    "historyGapThresholdSeconds",
    "locationRecordingEnabled",
    "includeGpsLocationWhenExporting",
  ]

  private static func settingsDictionary(_ value: SettingsRecord) -> [String: Any] {
    [
      "idleProbeIntervalMs": value.idleProbeIntervalMs,
      "idlePersistIntervalMs": value.idlePersistIntervalMs,
      "ridingPersistIntervalMs": value.ridingPersistIntervalMs,
      "postRideTailMs": value.postRideTailMs,
      "reconnectDurationMs": value.reconnectDurationMs,
      "minimumSessionDistance": value.minimumSessionDistance,
      "themePreference": value.themePreference.rawValue,
      "unitPreference": value.unitPreference.rawValue,
      "historyGapThresholdSeconds": value.historyGapThresholdSeconds ?? NSNull(),
      "locationRecordingEnabled": value.locationRecordingEnabled,
      "includeGpsLocationWhenExporting": value.includeGpsLocationWhenExporting,
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
    func double(_ key: String, _ range: ClosedRange<Double>) throws -> Double {
      guard let number = values[key] as? NSNumber,
            CFGetTypeID(number) != CFBooleanGetTypeID(),
            number.doubleValue.isFinite
      else { throw ProductStoreError.invalidSettingsPatch }
      return min(range.upperBound, max(range.lowerBound, number.doubleValue))
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
    func historyGapThresholdSeconds(_ key: String) throws -> Double? {
      guard let raw = values[key] else { throw ProductStoreError.invalidSettingsPatch }
      if raw is NSNull { return nil }
      guard let number = raw as? NSNumber,
            CFGetTypeID(number) != CFBooleanGetTypeID(),
            number.doubleValue.isFinite
      else { throw ProductStoreError.invalidSettingsPatch }
      return min(3_600, max(0.1, number.doubleValue))
    }
    func bool(_ key: String) throws -> Bool {
      guard let number = values[key] as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else {
        throw ProductStoreError.invalidSettingsPatch
      }
      return number.boolValue
    }
    return try SettingsRecord(
      revision: revision,
      idleProbeIntervalMs: integer("idleProbeIntervalMs", 100 ... 10_000),
      idlePersistIntervalMs: integer("idlePersistIntervalMs", 100 ... 60_000),
      ridingPersistIntervalMs: integer("ridingPersistIntervalMs", 50 ... 10_000),
      postRideTailMs: integer("postRideTailMs", 0 ... 300_000),
      reconnectDurationMs: integer("reconnectDurationMs", 0 ... 3_600_000),
      minimumSessionDistance: double("minimumSessionDistance", 0 ... 1_000_000),
      themePreference: themePreference("themePreference"),
      unitPreference: unitPreference("unitPreference"),
      historyGapThresholdSeconds: historyGapThresholdSeconds("historyGapThresholdSeconds"),
      locationRecordingEnabled: bool("locationRecordingEnabled"),
      includeGpsLocationWhenExporting: bool("includeGpsLocationWhenExporting")
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
          let number = Double(raw[raw.index(after: first) ..< second])
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
    stableIdentity: String?,
    at: Date
  ) throws -> SavedDeviceRecord {
    let stableIdentity = stableIdentity.flatMap(Self.normalizedDeviceHistoryID)
    let stableMatch = stableIdentity.flatMap { identity in
      try? catalog.query(
        "SELECT saved_device_id FROM saved_device WHERE device_history_id = ? LIMIT 1",
        values: [.text(identity)],
        map: { Self.columnText($0, 0)! }
      ).first
    } ?? nil
    let localMatch = try catalog.query(
      "SELECT saved_device_id FROM saved_device WHERE opaque_platform_id = ? LIMIT 1",
      values: [.text(deviceID.value)],
      map: { Self.columnText($0, 0)! }
    ).first
    let existingID = stableMatch ?? localMatch
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    if let existingID {
      try catalog.statement(
        """
        UPDATE saved_device
        SET opaque_platform_id = ?, advertised_name = COALESCE(?, advertised_name), last_seen_at_ms = ?,
            last_connected_at_ms = ?
        WHERE saved_device_id = ?
        """,
        values: [
          .text(deviceID.value), advertisedName.map(SQLiteValue.text) ?? .null,
          .integer(timestamp), .integer(timestamp), .text(existingID),
        ]
      )
      _ = try boardProfileLocked(savedDeviceID: existingID, at: at)
      guard let value = try savedDevicesLocked().first(where: { $0.savedDeviceID == existingID }) else {
        throw ProductStoreError.savedDeviceNotFound
      }
      return value
    }
    let savedDeviceID = UUID().uuidString
    let deviceHistoryID = stableIdentity ?? UUID().uuidString
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
    guard let value = try savedDevicesLocked().first(where: { $0.savedDeviceID == savedDeviceID }) else {
      throw ProductStoreError.savedDeviceNotFound
    }
    return value
  }

  private func ensureImportedSavedDeviceLocked(
    deviceHistoryID: String,
    displayName: String?,
    profileSnapshotJSON: String,
    at: Date
  ) throws -> SavedDeviceRecord {
    guard let historyID = Self.normalizedDeviceHistoryID(deviceHistoryID) else {
      throw SessionStoreError.invalidArchive
    }
    guard profileSnapshotJSON.utf8.count <= 16_384,
          let profileData = profileSnapshotJSON.data(using: .utf8),
          let profileObject = try? JSONSerialization.jsonObject(with: profileData),
          profileObject is [String: Any]
    else { throw SessionStoreError.invalidArchive }
    if let existing = try catalog.query(
      "SELECT saved_device_id FROM saved_device WHERE device_history_id = ? LIMIT 1",
      values: [.text(historyID)],
      map: { Self.columnText($0, 0)! }
    ).first {
      _ = try boardProfileLocked(savedDeviceID: existing, at: at)
      guard let value = try savedDevicesLocked().first(where: { $0.savedDeviceID == existing }) else {
        throw ProductStoreError.savedDeviceNotFound
      }
      return value
    }
    let trimmedName = displayName?.trimmingCharacters(in: .whitespacesAndNewlines)
    let normalizedName = trimmedName?.isEmpty == true ? nil : trimmedName
    guard normalizedName?.utf8.count ?? 0 <= 128 else {
      throw SessionStoreError.invalidArchive
    }
    let savedDeviceID = UUID().uuidString
    let timestamp = Int64(at.timeIntervalSince1970 * 1_000)
    try catalog.statement(
      """
      INSERT INTO saved_device(
        saved_device_id, device_history_id, opaque_platform_id, advertised_name,
        custom_display_name, revision, created_at_ms, last_seen_at_ms, last_connected_at_ms
      ) VALUES (?, ?, NULL, ?, NULL, 0, ?, NULL, NULL)
      """,
      values: [
        .text(savedDeviceID), .text(historyID), normalizedName.map(SQLiteValue.text) ?? .null,
        .integer(timestamp),
      ]
    )
    try catalog.statement(
      "INSERT INTO board_profile(saved_device_id, revision, json, updated_at_ms) VALUES (?, 0, ?, ?)",
      values: [.text(savedDeviceID), .text(profileSnapshotJSON), .integer(timestamp)]
    )
    guard let value = try savedDevicesLocked().first(where: { $0.savedDeviceID == savedDeviceID }) else {
      throw ProductStoreError.savedDeviceNotFound
    }
    return value
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
      VALUES (?, 0, '{"preferredBmsCanId":null}', ?)
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
      "values": JSONSerialization.jsonObject(with: Data(profile.valuesJSON.utf8)),
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
          integer("preferredCanId", range: 0 ... 253),
          integer("preferredBmsCanId", range: 0 ... 253)
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

  private static func normalizedDeviceHistoryID(_ value: String) -> String? {
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty, trimmed.utf8.count <= 128,
          trimmed.utf8.allSatisfy({ byte in
            (byte >= 48 && byte <= 57) ||
              (byte >= 65 && byte <= 90) ||
              (byte >= 97 && byte <= 122) ||
              byte == 45 || byte == 46 || byte == 95
          })
    else { return nil }
    return trimmed
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
