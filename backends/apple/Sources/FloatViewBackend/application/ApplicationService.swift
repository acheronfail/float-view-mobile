import CryptoKit
import Foundation

public final class ApplicationService: @unchecked Sendable {
  private let bluetooth: BluetoothPort
  private let clock: ClockPort
  private let logStore: LogStorePort
  private let productStore: (any ProductStorePort)?
  private let sessionStore: (any SessionStorePort)?
  private let scheduler: (any SchedulerPort)?
  private let locationProvider: (any LocationProviderPort)?
  private let events: EventSink
  private let rustRuntime: RustApplicationRuntime
  private var rustGeneration: UInt64 = 0
  private var rustDeadlines: [UInt64: any CancellationPort] = [:]
  private var rustTimers: [UInt64: any CancellationPort] = [:]
  private var rustTimerIntervals: [UInt64: Int64] = [:]
  private var rustSessionAttached = false
  private var rustControllerRoute: ControllerRoute?
  private var rustControllerCandidates: [ControllerCandidate] = []
  private var rustConnectionIssue: ConnectionIssue?
  private var rustFirmware: FirmwareRecord?
  private var rustRefloatPackageVersion: String?
  private var rustControls: RefloatControls?
  private var rustControllerActions = ControllerActions()
  private var rustBmsState = BmsState()
  private var rustConfigurationSummary = RefloatConfigurationSummary(
    availability: .unavailable, writable: false, packageVersion: nil,
    schemaFingerprint: nil, wireSignature: nil, currentLoaded: false,
    defaultLoaded: false, pendingOperation: nil
  )
  private var rustConfigurationSchema: RefloatConfigurationSchema?
  private var rustConfigurationCurrent: RefloatConfigurationValues?
  private var rustConfigurationDefaults: RefloatConfigurationValues?
  private var rustSessionResult: [String: Any]?
  private var reconnectDurationMs: Int64
  private var idleProbeIntervalMs: Int64
  private let lock = NSRecursiveLock()
  private var samplingSettings: SamplingSettings
  private var explicitDisconnect = false
  private var scanSequence = 0
  private var knownDevicesByID: [DeviceID: KnownDevice] = [:]
  private var discoveryPublishTimer: (any CancellationPort)?
  private var discoveryUpdatePending = false
  private var discoveryExpiryTimer: (any CancellationPort)?
  private var discoveryExpiryGeneration = 0
  private var deviceKind: DeviceKind?
  private var trackingMode: TrackingMode = .saved
  private var currentSavedDevice: SavedDeviceRecord?
  private var sessionControllerRoute: ControllerRoute?
  private var sessionControllerUUID: Data?
  private var sessionFirmware: FirmwareRecord?
  private var sessionRefloatPackageVersion: String?
  private var latestPersistedSample: TelemetrySample?
  private var latestLocation: LiveLocation?
  private var locationCaptureStartedMonotonicMs: Int64?
  private var locationCaptureBaseElapsedMs: Int64 = 0
  private var locationCaptureActive = false
  private var currentLimits: ControllerCurrentLimits?
  private var refloatWaiter: RefloatMutationWaiter?
  private var refloatConfigurationWaiter: RefloatConfigurationRequestWaiter?
  private var refloatConfigurationObservedPending = false
  private var reconnectTimer: (any CancellationPort)?
  private var reconnectStartedAtMs: Int64?
  private var reconnectAttempt = 0

  public private(set) var activeDeviceID: DeviceID?
  private var activeDeviceName: String?
  public private(set) var connectionState: ConnectionState = .disconnected
  public private(set) var bluetoothState: BluetoothState
  public private(set) var activeScanID: String?
  public var knownDevices: [KnownDevice] {
    knownDevicesByID.values.sorted { $0.deviceID.value < $1.deviceID.value }
  }

  public init(
    bluetooth: BluetoothPort,
    clock: ClockPort,
    logStore: LogStorePort,
    productStore: (any ProductStorePort)? = nil,
    sessionStore: (any SessionStorePort)? = nil,
    scheduler: (any SchedulerPort)? = nil,
    events: EventSink,
    samplingSettings: SamplingSettings = .init(),
    locationProvider: (any LocationProviderPort)? = nil
  ) {
    do {
      rustRuntime = try RustApplicationRuntime()
    } catch {
      preconditionFailure("Unified Rust application runtime is unavailable: \(error)")
    }
    self.bluetooth = bluetooth
    self.clock = clock
    self.logStore = logStore
    self.productStore = productStore
    self.sessionStore = sessionStore
    self.scheduler = scheduler
    self.locationProvider = locationProvider
    self.events = events
    reconnectDurationMs = samplingSettings.reconnectDurationMs
    idleProbeIntervalMs = samplingSettings.idleProbeIntervalMs
    self.samplingSettings = samplingSettings
    bluetoothState = bluetooth.currentState()
    let savedIDs = ((try? productStore?.savedDevices()) ?? [])
      .compactMap { $0.bluetoothIdentifier.map(DeviceID.init) }
    bluetooth.configureSavedDeviceIDs(Set(savedIDs))
  }

  @discardableResult
  public func requestBluetoothPermission() -> BluetoothState {
    lock.lock()
    defer { lock.unlock() }
    bluetooth.requestPermission()
    bluetoothState = bluetooth.currentState()
    return bluetoothState
  }

  public func savedDevices() throws -> [SavedDeviceRecord] {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    return try productStore.savedDevices()
  }

  public func updateSavedDevice(
    savedDeviceID: String,
    expectedRevision: Int,
    customDisplayName: String?
  ) throws -> SavedDeviceRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    let value = try productStore.updateSavedDevice(
      savedDeviceID: savedDeviceID,
      expectedRevision: expectedRevision,
      customDisplayName: customDisplayName
    )
    events.publish(value)
    return value
  }

  public func deleteSavedDevice(savedDeviceID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    if activeConnectionSnapshot()?.savedDevice?.savedDeviceID == savedDeviceID {
      throw ProductStoreError.savedDeviceActive
    }
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    try productStore.deleteSavedDevice(savedDeviceID: savedDeviceID)
    events.publish(String("saved device deleted"))
  }

  public func boardProfile(savedDeviceID: String) throws -> BoardProfileRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    return try productStore.boardProfile(savedDeviceID: savedDeviceID)
  }

  public func updateBoardProfile(
    savedDeviceID: String,
    expectedRevision: Int,
    patchJSON: String
  ) throws -> BoardProfileRecord {
    lock.lock()
    defer { lock.unlock() }
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    let profile = try productStore.updateBoardProfile(
      savedDeviceID: savedDeviceID,
      expectedRevision: expectedRevision,
      patchJSON: patchJSON,
      at: clock.now()
    )
    events.publish(profile)
    return profile
  }

  public func selectController(deviceID: DeviceID, canID: UInt8) throws {
    lock.lock()
    defer { lock.unlock() }
    guard activeDeviceID == deviceID, connectionState == .selectingController,
          rustControllerCandidates.contains(where: { $0.canID == canID })
    else {
      throw ApplicationError.invalidControllerSelection
    }
    if trackingMode == .saved {
      guard let productStore, let savedDevice = currentSavedDevice else {
        throw ApplicationError.invalidControllerSelection
      }
      let profile = try productStore.selectController(
        savedDeviceID: savedDevice.savedDeviceID,
        canID: canID,
        controllerUUID: nil,
        at: clock.now()
      )
      events.publish(profile)
    }
    transition(.identifying)
    try performRustCommand([
      "command": "select_controller", "generation": rustGeneration, "can_id": canID,
      "now_ms": clock.monotonicMilliseconds(),
    ])
  }

  public func settings() throws -> SettingsRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    return try productStore.settings()
  }

  public func locationState() -> LocationProviderState {
    let enabled = (try? productStore?.settings().locationRecordingEnabled) ?? false
    guard enabled else {
      return LocationProviderState(
        authorization: .unavailable,
        backgroundCapture: .unavailable,
        reason: nil
      )
    }
    return locationProvider?.state() ?? LocationProviderState(
      authorization: .unavailable,
      backgroundCapture: .unavailable,
      reason: "This host does not provide location capture."
    )
  }

  @discardableResult
  public func requestLocationPermission() -> LocationProviderState {
    locationProvider?.requestPermission()
    return locationState()
  }

  @discardableResult
  public func openLocationSettings() -> LocationProviderState {
    locationProvider?.openSettings()
    return locationState()
  }

  public func updateSettings(
    expectedRevision: Int,
    patchJSON: String
  ) throws -> SettingsRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    let value = try productStore.updateSettings(
      expectedRevision: expectedRevision,
      patchJSON: patchJSON
    )
    lock.withLock {
      idleProbeIntervalMs = value.idleProbeIntervalMs
      reconnectDurationMs = value.reconnectDurationMs
      samplingSettings = SamplingSettings(
        idleProbeIntervalMs: value.idleProbeIntervalMs,
        idlePersistIntervalMs: value.idlePersistIntervalMs,
        ridingPersistIntervalMs: value.ridingPersistIntervalMs,
        postRideTailMs: value.postRideTailMs,
        reconnectDurationMs: value.reconnectDurationMs
      )
      try? performRustCommand([
        "command": "update_settings", "generation": rustGeneration,
        "idle_interval_ms": value.idleProbeIntervalMs,
        "post_ride_tail_ms": value.postRideTailMs,
        "now_ms": clock.monotonicMilliseconds(),
      ])
      if !value.locationRecordingEnabled {
        stopLocationCapture()
      } else {
        locationProvider?.requestPermission()
      }
    }
    events.publish(value)
    return value
  }

  public func sessions(_ query: SessionListQuery) throws -> SessionPage {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessions(query)
  }

  public func session(sessionID: String) throws -> SessionSummaryRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.session(sessionID: sessionID)
  }

  public func sessionSnapshot(sessionID: String) throws -> SessionPublishedSnapshotRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionSnapshot(sessionID: sessionID)
  }

  public func sessionHistoryConfiguration(sessionID: String) throws -> SessionHistoryConfiguration {
    let value = try rustSessionQuery([
      "command": "session_metadata", "path": sessionPath(sessionID),
    ])
    guard let riding = Self.int64(value["expected_riding_persistence_ms"]),
          let idle = Self.int64(value["expected_idle_persistence_ms"]),
          let resolution = Self.int64(value["retained_resolution_seconds"])
    else { throw SessionStoreError.sessionNotFound }
    return SessionHistoryConfiguration(
      expectedRidingPersistenceMs: riding, expectedIdlePersistenceMs: idle,
      retainedResolutionSeconds: resolution
    )
  }

  public func querySessionSeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int,
    publishedSampleSequence: Int64? = nil
  ) throws -> [SessionSeriesPoint] {
    try rustSessionSamples(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs,
      fields: fields, maxRows: maxPoints, publishedSampleSequence: publishedSampleSequence
    )
  }

  public func querySessionSeriesSource(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxRows: Int,
    publishedSampleSequence: Int64? = nil
  ) throws -> [SessionSeriesPoint] {
    try rustSessionSamples(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs,
      fields: fields, maxRows: maxRows, publishedSampleSequence: publishedSampleSequence
    )
  }

  public func reduceSessionSeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    buckets: Int,
    gapPolicy: SessionGapPolicyRecord,
    expectedPersistenceMs _: Int64,
    expectedRidingPersistenceMs _: Int64?,
    expectedIdlePersistenceMs _: Int64?,
    retainedResolutionSeconds _: Int64,
    publishedSampleSequence: Int64? = nil
  ) throws -> SessionSeriesReductionRecord {
    let snapshot = try sessionSnapshot(sessionID: sessionID)
    let started = DispatchTime.now().uptimeNanoseconds
    let value = try rustSessionQuery([
      "command": "session_reduce", "path": sessionPath(sessionID), "fields": fields,
      "start_elapsed_ms": startElapsedMs, "end_elapsed_ms": endElapsedMs, "buckets": buckets,
      "gap_seconds": gapPolicy.thresholdSeconds ?? NSNull(),
      "boundary": Self.boundary(snapshot, sampleSequence: publishedSampleSequence),
    ])
    let columns = (value["columns"] as? [[String: Any]] ?? []).reduce(into: [String: SessionSeriesColumnRecord]()) {
      guard let field = $1["field"] as? String,
            let elapsed = Self.int64s($1["elapsed_ms"]),
            let sequences = Self.int64s($1["sequences"]),
            let values = Self.optionalDoubles($1["values"]),
            let generations = Self.ints($1["generations"])
      else { return }
      $0[field] = SessionSeriesColumnRecord(
        elapsedMs: elapsed, sequence: sequences, values: values, generations: generations
      )
    }
    let elapsed = Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000
    return SessionSeriesReductionRecord(
      columns: columns, sourceRows: Int(Self.int64(value["source_rows"]) ?? 0),
      sourceMs: elapsed, reducerMs: 0, elapsedMs: elapsed
    )
  }

  public func withSessionReadCancellation<T>(
    _ cancellation: SessionReadCancellation,
    body: () throws -> T
  ) rethrows -> T {
    let registration = cancellation.register { [rustRuntime] in rustRuntime.interrupt() }
    defer { cancellation.unregister(registration) }
    return try body()
  }

  public func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    publishedEventSequence: Int64? = nil
  ) throws -> [SessionEventRecord] {
    let snapshot = try sessionSnapshot(sessionID: sessionID)
    let value = try rustSessionQuery([
      "command": "session_events", "path": sessionPath(sessionID),
      "start_elapsed_ms": startElapsedMs, "end_elapsed_ms": endElapsedMs, "limit": 500,
      "after_elapsed_ms": NSNull(), "after_sequence": NSNull(),
      "boundary": Self.boundary(snapshot, eventSequence: publishedEventSequence),
    ])
    return (value["rows"] as? [[String: Any]] ?? []).compactMap {
      guard let sequence = Self.int64($0["sequence"]), let elapsed = Self.int64($0["elapsed_ms"]),
            let kind = $0["kind"] as? String, let details = $0["details_json"] as? String
      else { return nil }
      return SessionEventRecord(
        eventID: String(sequence), elapsedMs: elapsed, kind: kind, detailsJSON: details,
        sourceSampleSequence: Self.int64($0["source_sample_sequence"])
      )
    }
  }

  public func querySessionLocations(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    limit: Int,
    cursor: String?,
    publishedLocationSequence: Int64?
  ) throws -> SessionLocationPageRecord {
    let snapshot = try sessionSnapshot(sessionID: sessionID)
    let after = cursor.flatMap { Data(base64Encoded: $0) }
      .flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String: Any] }
      .flatMap { Self.int64($0["locationSequence"]) }
    let value = try rustSessionQuery([
      "command": "session_locations", "path": sessionPath(sessionID),
      "start_elapsed_ms": startElapsedMs, "end_elapsed_ms": endElapsedMs, "limit": limit,
      "after_location_sequence": after ?? NSNull(),
      "boundary": Self.boundary(snapshot, locationSequence: publishedLocationSequence),
    ])
    let points: [SessionLocationPointRecord] = (value["rows"] as? [[String: Any]] ?? []).compactMap {
      guard let sequence = Self.int64($0["location_sequence"]), let elapsed = Self.int64($0["elapsed_ms"]),
            let observed = Self.int64($0["observed_at_epoch_ms"]),
            let latitude = Self.double($0["latitude"]), let longitude = Self.double($0["longitude"])
      else { return nil }
      return SessionLocationPointRecord(
        locationSequence: sequence, elapsedMs: elapsed, latitude: latitude, longitude: longitude,
        accuracyM: Self.double($0["accuracy_m"]),
        observedAt: Date(timeIntervalSince1970: Double(observed) / 1_000)
      )
    }
    let gaps: [SessionLocationGapRecord] = (value["gaps"] as? [[String: Any]] ?? []).compactMap {
      guard let start = Self.int64($0["start_elapsed_ms"]), let end = Self.int64($0["end_elapsed_ms"])
      else { return nil }
      return SessionLocationGapRecord(startElapsedMs: start, endElapsedMs: end, reason: "noFix")
    }
    let next = Self.int64(value["next_location_sequence"]).flatMap { sequence in
      try? JSONSerialization.data(withJSONObject: [
        "sessionId": sessionID, "startElapsedMs": startElapsedMs, "endElapsedMs": endElapsedMs,
        "historyRevision": snapshot.historyRevision, "locationSequence": sequence,
      ]).base64EncodedString()
    }
    return SessionLocationPageRecord(points: points, gaps: gaps, nextCursor: next, snapshot: snapshot)
  }

  public func sessionExtensionFields(sessionID: String) throws -> [String] {
    let value = try rustSessionQuery(["command": "session_fields", "path": sessionPath(sessionID)])
    return (value["rows"] as? [[String: Any]] ?? []).compactMap {
      ($0["source"] as? String) == "extension" ? $0["name"] as? String : nil
    }
  }

  public func sessionPruneMetadata(sessionID: String) throws -> SessionPruneMetadataRecord {
    let value = try rustSessionQuery(["command": "session_metadata", "path": sessionPath(sessionID)])
    return SessionPruneMetadataRecord(
      targetResolutionSeconds: Self.int64(value["retained_resolution_seconds"]).flatMap { $0 == 0 ? nil : $0 },
      recordedSampleCount: Self.int64(value["recorded_sample_count"]) ?? 0,
      storedSampleCount: Self.int64(value["stored_sample_count"]) ?? 0,
      policy: value["prune_policy"] as? String
    )
  }

  public func prunePreview(sessionID: String, targetResolutionSeconds: Int64) throws -> SessionPrunePreviewRecord {
    try rustPrune(sessionID: sessionID, targetResolutionSeconds: targetResolutionSeconds, commit: false)
  }

  public func pruneSession(sessionID: String, targetResolutionSeconds: Int64) throws -> SessionPrunePreviewRecord {
    sessionStore?.cancelHistoricalReads(sessionID: sessionID)
    let value = try rustPrune(sessionID: sessionID, targetResolutionSeconds: targetResolutionSeconds, commit: true)
    try sessionStore?.reconcileSession(sessionID: sessionID)
    return value
  }

  public func maintenanceJob() throws -> SessionMaintenanceJobRecord? {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.maintenanceJob()
  }

  public func saveMaintenanceJob(_ job: SessionMaintenanceJobRecord) throws {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    try sessionStore.saveMaintenanceJob(job)
  }

  public func cancelMaintenanceJob(operationID: String) throws -> SessionMaintenanceJobRecord? {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.cancelMaintenanceJob(operationID: operationID)
  }

  public func querySampleWindow(
    sessionID: String,
    centerElapsedMs: Int64,
    beforeMs: Int64,
    afterMs: Int64,
    fields: [String],
    publishedSampleSequence: Int64? = nil
  ) throws -> [SessionSeriesPoint] {
    try rustSessionSamples(
      sessionID: sessionID, startElapsedMs: max(0, centerElapsedMs - beforeMs),
      endElapsedMs: centerElapsedMs + afterMs, fields: fields, maxRows: 2_000,
      publishedSampleSequence: publishedSampleSequence
    )
  }

  public func deleteSession(sessionID: String) throws {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    try sessionStore.deleteSession(sessionID: sessionID)
  }

  public func cancelPendingDelete(operationID: String) throws -> Bool {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.cancelPendingDelete(operationID: operationID)
  }

  public func exportSession(
    sessionID: String,
    format: String
  ) throws -> SessionExportRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.exportSession(sessionID: sessionID, format: format)
  }

  public func importSessionArchive(data: Data) throws -> String {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.importSessionArchive(data: data)
  }

  public func startDiscovery() throws -> String {
    lock.lock()
    defer { lock.unlock() }
    guard bluetoothState.scanPermission == .granted ||
      bluetoothState.scanPermission == .notRequired
    else {
      throw ApplicationError.permissionRequired
    }
    guard bluetoothState.radio == .poweredOn else {
      throw ApplicationError.bluetoothUnavailable
    }
    if let activeScanID { return activeScanID }
    scanSequence += 1
    let scanID = "scan-\(scanSequence)"
    activeScanID = scanID
    bluetooth.startDiscovery()
    events.publish(DiscoverySnapshot(
      bluetoothState: bluetoothState,
      activeScanID: activeScanID,
      knownDevices: knownDevices
    ))
    return scanID
  }

  public func stopDiscovery(scanID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    guard let activeScanID else { return }
    guard scanID == activeScanID else { throw ApplicationError.invalidScan }
    bluetooth.stopDiscovery()
    self.activeScanID = nil
    cancelPendingDiscoveryPublish()
    events.publish(DiscoverySnapshot(
      bluetoothState: bluetoothState,
      activeScanID: nil,
      knownDevices: knownDevices
    ))
  }

  public func connect(_ deviceID: DeviceID, trackingMode: TrackingMode = .saved) throws {
    lock.lock()
    defer { lock.unlock() }
    guard knownDevicesByID[deviceID] != nil else {
      throw ApplicationError.unknownDevice
    }
    guard activeDeviceID == nil || activeDeviceID == deviceID else {
      throw ApplicationError.activeDeviceConflict
    }
    if activeDeviceID == deviceID, connectionState == .failed, self.trackingMode != trackingMode {
      throw ApplicationError.activeDeviceConflict
    }
    if activeScanID != nil {
      bluetooth.stopDiscovery()
      activeScanID = nil
      cancelPendingDiscoveryPublish()
    }
    activeDeviceID = deviceID
    self.trackingMode = trackingMode
    currentSavedDevice = nil
    activeDeviceName = knownDevicesByID[deviceID]?.name
    if logStore.active() == nil {
      sessionControllerRoute = nil
      sessionControllerUUID = nil
      sessionFirmware = nil
      sessionRefloatPackageVersion = nil
    }
    explicitDisconnect = false
    cancelReconnect()
    latestPersistedSample = nil
    latestLocation = nil
    try performRustCommand(["command": "reset", "now_ms": clock.monotonicMilliseconds()])
    transition(.connecting)
    bluetooth.connect(deviceID)
  }

  public func receive(_ event: BluetoothEvent) throws {
    lock.lock()
    defer { lock.unlock() }
    switch event {
    case let .stateChanged(state):
      bluetoothState = state
      if state.radio != .poweredOn || !Self.permissionAllowsUse(state.scanPermission),
         activeScanID != nil
      {
        bluetooth.stopDiscovery()
        activeScanID = nil
        cancelPendingDiscoveryPublish()
      }
      if Self.permissionIsTerminal(state.connectPermission),
         activeDeviceID != nil,
         !explicitDisconnect
      {
        finishReconnect(reason: "bluetooth_permission_denied")
      } else if state.radio != .poweredOn,
                activeDeviceID != nil,
                trackingMode == .saved || trackingMode == .guest,
                !explicitDisconnect
      {
        beginReconnect()
      }
      events.publish(state)
    case let .deviceFound(deviceID, name, rssi):
      let device = KnownDevice(
        deviceID: deviceID,
        name: name,
        rssi: rssi,
        lastSeenAt: clock.now(),
        savedDeviceID: savedDeviceID(for: deviceID)
      )
      knownDevicesByID[deviceID] = device
      scheduleDiscoveryExpiry()
      publishDiscoveryUpdate(device)
    case let .scanFailed(reason):
      activeScanID = nil
      cancelPendingDiscoveryPublish()
      events.publish(reason)
    case let .connectionChanged(deviceID, state):
      guard deviceID == activeDeviceID else { return }
      if state == .failed || state == .disconnected,
         rustControllerActions.pendingAction == .shutdown
      {
        finishReconnect(reason: "controller_shutdown")
        return
      }
      if state == .ready {
        transition(.identifying)
        try beginRustApplication()
        return
      }
      if state == .failed || state == .disconnected,
         !explicitDisconnect,
         trackingMode == .saved || trackingMode == .guest
      {
        beginReconnect()
        return
      }
      if state == .failed, trackingMode == .saved || trackingMode == .guest, !explicitDisconnect,
         logStore.active() == nil
      {
        rustConnectionIssue = ConnectionIssue(
          code: .transportSetupFailed,
          message: "The VESC Bluetooth transport could not be prepared."
        )
        transition(.failed)
        return
      }
      if state == .disconnected, connectionState == .failed, !explicitDisconnect {
        return
      }
      if state == .disconnected {
        cancelReconnect()
        activeDeviceID = nil
        activeDeviceName = nil
        latestPersistedSample = nil
        latestLocation = nil
      }
      transition(state)
    case let .notification(deviceID, bytes):
      guard deviceID == activeDeviceID else { return }
      try performRustCommand([
        "command": "receive_bytes", "generation": rustGeneration, "bytes": Array(bytes),
        "now_ms": clock.monotonicMilliseconds(),
        "observed_at_epoch_ms": Int64(clock.now().timeIntervalSince1970 * 1_000),
      ])
      return
    }
  }

  public func disconnect() throws {
    lock.lock()
    defer { lock.unlock() }
    guard let deviceID = activeDeviceID else { return }
    transition(.disconnecting)
    explicitDisconnect = true
    cancelReconnect()
    stopLocationCapture()
    if trackingMode == .saved, let active = logStore.active() {
      try performRustCommand(["command": "detach_session"])
      rustSessionAttached = false
      try events.publish(
        logStore.close(jobID: active.jobID, at: clock.now(), reason: "explicit_disconnect")
      )
    }
    rustDeadlines.values.forEach { $0.cancel() }
    rustDeadlines.removeAll()
    rustTimers.values.forEach { $0.cancel() }
    rustTimers.removeAll()
    rustTimerIntervals.removeAll()
    bluetooth.disconnect(deviceID)
  }

  public func ingest(_ sample: TelemetrySample) throws {
    lock.lock()
    defer { lock.unlock() }
    guard connectionState == .ready else { return }
    if trackingMode == .guest {
      latestPersistedSample = sample
      events.publish(sample)
      return
    }
    guard let active = logStore.active() else { return }
    updateLocationCapture(rideState: sample.rideState)
    try logStore.append(jobID: active.jobID, samples: [sample])
    latestPersistedSample = sample
    events.publish(sample)
  }

  private func updateLocationCapture(rideState: RideState) {
    guard trackingMode == .saved,
          let locationProvider,
          let productStore,
          let settings = try? productStore.settings(),
          settings.locationRecordingEnabled,
          rideState == .riding,
          let active = logStore.active()
    else {
      stopLocationCapture()
      return
    }
    let providerState = locationProvider.state()
    guard providerState.authorization == .authorized || providerState.authorization == .foregroundOnly else {
      stopLocationCapture()
      return
    }
    guard !locationCaptureActive else { return }
    locationCaptureActive = true
    locationCaptureStartedMonotonicMs = clock.monotonicMilliseconds()
    locationCaptureBaseElapsedMs = latestPersistedSample?.elapsedMs ?? 0
    locationProvider.startUpdates { [weak self] observation in
      self?.ingestLocation(observation, jobID: active.jobID)
    }
  }

  private func stopLocationCapture() {
    guard locationCaptureActive else { return }
    locationCaptureActive = false
    locationCaptureStartedMonotonicMs = nil
    locationProvider?.stopUpdates()
  }

  private func ingestLocation(_ observation: LocationProviderObservation, jobID: JobID) {
    lock.lock()
    defer { lock.unlock() }
    guard locationCaptureActive,
          let started = locationCaptureStartedMonotonicMs,
          logStore.active()?.jobID == jobID
    else { return }
    let elapsedMs = locationCaptureBaseElapsedMs + max(0, clock.monotonicMilliseconds() - started)
    do {
      try performRustCommand([
        "command": "append_locations",
        "locations": [[
          "observed_at_epoch_ms": Int64(observation.observedAt.timeIntervalSince1970 * 1_000),
          "elapsed_ms": elapsedMs, "latitude": observation.latitude,
          "longitude": observation.longitude,
          "accuracy_m": observation.accuracyM.map { $0 as Any } ?? NSNull(),
        ]],
      ])
      latestLocation = LiveLocation(
        latitude: observation.latitude,
        longitude: observation.longitude,
        accuracyM: observation.accuracyM,
        observedAt: observation.observedAt
      )
      events.publish(LocationUpdated())
    } catch {
      events.publish(String(describing: error))
    }
  }

  public func discoverySnapshot() -> DiscoverySnapshot {
    lock.lock()
    defer { lock.unlock() }
    return DiscoverySnapshot(
      bluetoothState: bluetoothState,
      activeScanID: activeScanID,
      knownDevices: knownDevices
    )
  }

  public func activeConnectionSnapshot() -> ActiveConnectionSnapshot? {
    lock.lock()
    defer { lock.unlock() }
    guard let activeDeviceID else { return nil }
    let activeSession = trackingMode == .saved ? logStore.active().flatMap { record in
      try? sessionStore?.session(sessionID: record.sessionID.value)
    } : nil
    return ActiveConnectionSnapshot(
      deviceID: activeDeviceID,
      name: activeDeviceName,
      state: connectionState,
      deviceKind: deviceKind,
      trackingMode: trackingMode,
      savedDevice: trackingMode == .saved ? currentSavedDevice : nil,
      activeSession: activeSession ?? nil,
      latestSample: latestPersistedSample,
      latestLocation: latestLocation,

      currentLimits: nil,
      controllerRoute: rustControllerRoute,
      controllerCandidates: rustControllerCandidates,
      connectionIssue: rustConnectionIssue,
      bms: rustBmsState,
      refloatControls: rustControls,
      controllerActions: rustControllerActions
    )
  }

  public func shutdownController() throws {
    try lock.withLock {
      try performRustCommand([
        "command": "shutdown_controller", "generation": rustGeneration,
        "now_ms": clock.monotonicMilliseconds(),
      ])
    }
  }

  public func setRefloatControl(_ control: RefloatBooleanControl, enabled: Bool) throws -> RefloatControls {
    let waiter = try lock.withLock { () throws -> RefloatMutationWaiter in
      let waiter = RefloatMutationWaiter()
      refloatWaiter = waiter
      let controlName = switch control {
      case .package: "package"
      case .flywheel: "flywheel"
      case .leds: "leds"
      case .headlights: "headlights"
      }
      try performRustCommand([
        "command": "set_control", "generation": rustGeneration, "control": controlName,
        "enabled": enabled, "now_ms": clock.monotonicMilliseconds(),
      ])
      return waiter
    }
    if let controls = lock.withLock({ rustControls }), controls.pendingAction == nil {
      return controls
    }
    return try waiter.wait(timeout: 3.25)
  }

  public func clearRefloatFatalError() throws -> RefloatControls {
    let waiter = try lock.withLock { () throws -> RefloatMutationWaiter in
      let waiter = RefloatMutationWaiter()
      refloatWaiter = waiter
      try performRustCommand([
        "command": "clear_fatal_error", "generation": rustGeneration,
        "now_ms": clock.monotonicMilliseconds(),
      ])
      return waiter
    }
    if let controls = lock.withLock({ rustControls }), controls.pendingAction == nil {
      return controls
    }
    return try waiter.wait(timeout: 3.25)
  }

  public func firmwareSnapshot() -> FirmwareRecord? {
    lock.lock()
    defer { lock.unlock() }
    return rustFirmware
  }

  public func refloatPackageVersionSnapshot() -> String? {
    lock.lock()
    defer { lock.unlock() }
    return rustRefloatPackageVersion
  }

  public func refloatConfigurationSummarySnapshot() -> RefloatConfigurationSummary {
    lock.withLock { rustConfigurationSummary }
  }

  public func refloatConfigurationSchemaSnapshot() -> RefloatConfigurationSchema? {
    lock.withLock { rustConfigurationSchema }
  }

  public func refloatConfigurationValuesSnapshot(defaults: Bool) -> RefloatConfigurationValues? {
    lock.withLock { defaults ? rustConfigurationDefaults : rustConfigurationCurrent }
  }

  public func readRefloatConfiguration(defaults: Bool) throws -> RefloatConfigurationValues {
    let waiter = try lock.withLock { () throws -> RefloatConfigurationRequestWaiter in
      let waiter = RefloatConfigurationRequestWaiter()
      refloatConfigurationWaiter = waiter
      refloatConfigurationObservedPending = false
      try performRustCommand([
        "command": "read_configuration", "generation": rustGeneration, "defaults": defaults,
        "now_ms": clock.monotonicMilliseconds(),
      ])
      return waiter
    }
    try waiter.wait(timeout: Self.configurationReadHostWaitTimeout)
    guard let values = lock.withLock({ defaults ? rustConfigurationDefaults : rustConfigurationCurrent })
    else { throw RefloatConfigurationServiceError.invalidState }
    return values
  }

  public func writeRefloatConfiguration(
    packageVersion: String, fingerprint: String, wireSignature: UInt32,
    baseRevision: UInt64, values: [String: RefloatConfigurationValue]
  ) throws -> RefloatConfigurationValues {
    try writeRefloatConfiguration(
      mutation: "write", packageVersion: packageVersion, fingerprint: fingerprint,
      wireSignature: wireSignature, baseRevision: baseRevision, values: values
    )
  }

  private func writeRefloatConfiguration(
    mutation: String, packageVersion: String, fingerprint: String, wireSignature: UInt32,
    baseRevision: UInt64, values: [String: RefloatConfigurationValue]
  ) throws -> RefloatConfigurationValues {
    let waiter = try lock.withLock { () throws -> RefloatConfigurationRequestWaiter in
      let waiter = RefloatConfigurationRequestWaiter()
      refloatConfigurationWaiter = waiter
      refloatConfigurationObservedPending = false
      try performRustCommand([
        "command": "write_configuration", "generation": rustGeneration, "mutation": mutation,
        "package_version": packageVersion, "fingerprint": fingerprint,
        "wire_signature": wireSignature, "base_revision": baseRevision,
        "values": Self.rustConfigurationValues(values), "now_ms": clock.monotonicMilliseconds(),
      ])
      return waiter
    }
    try waiter.wait(timeout: Self.configurationWriteHostWaitTimeout)
    guard let values = lock.withLock({ rustConfigurationCurrent })
    else { throw RefloatConfigurationServiceError.invalidState }
    return values
  }

  public func refloatTunes(
    identity: RefloatConfigurationIdentityRecord, limit: Int, cursor: String?
  ) throws -> RefloatConfigurationListPage {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    return try productStore.refloatConfigurations(identity: identity, limit: limit, cursor: cursor)
  }

  public func refloatTune(configurationID: String) throws -> RefloatConfigurationRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    return try productStore.refloatConfiguration(configurationID: configurationID)
  }

  public func createRefloatTune(
    name: String, description: String?, identity: RefloatConfigurationIdentityRecord,
    sourceRevision: UInt64, values: [String: RefloatConfigurationValue]
  ) throws -> RefloatConfigurationRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    try validateRefloatTuneValues(identity: identity, sourceRevision: sourceRevision, values: values)
    return try productStore.createRefloatConfiguration(
      name: name, description: description, identity: identity,
      valuesJSON: Self.refloatConfigurationValuesJSON(values), at: clock.now()
    )
  }

  public func updateRefloatTune(
    configurationID: String, expectedRevision: Int, name: String,
    description: String?, values: [String: RefloatConfigurationValue]
  ) throws -> RefloatConfigurationRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    let record = try productStore.refloatConfiguration(configurationID: configurationID)
    let currentRevision = rustConfigurationCurrent?.revision
    try validateRefloatTuneValues(identity: record.identity, sourceRevision: currentRevision, values: values)
    return try productStore.updateRefloatConfiguration(
      configurationID: configurationID, expectedRevision: expectedRevision,
      name: name, description: description,
      valuesJSON: Self.refloatConfigurationValuesJSON(values), at: clock.now()
    )
  }

  public func deleteRefloatTune(configurationID: String, expectedRevision: Int) throws {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    try productStore.deleteRefloatConfiguration(
      configurationID: configurationID, expectedRevision: expectedRevision
    )
  }

  public func applyRefloatTune(
    configurationID: String, expectedRevision: Int, baseRevision: UInt64
  ) throws -> RefloatConfigurationValues {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    let record = try productStore.refloatConfiguration(configurationID: configurationID)
    guard record.revision == expectedRevision else {
      throw ProductStoreError.refloatConfigurationRevisionConflict(currentRevision: record.revision)
    }
    let values = try Self.refloatConfigurationValues(json: record.valuesJSON)
    return try writeRefloatConfiguration(
      mutation: "tune_apply", packageVersion: record.identity.packageVersion,
      fingerprint: record.identity.schemaFingerprint,
      wireSignature: record.identity.wireSignature,
      baseRevision: baseRevision, values: values
    )
  }

  private func validateRefloatTuneValues(
    identity: RefloatConfigurationIdentityRecord, sourceRevision: UInt64?,
    values: [String: RefloatConfigurationValue]
  ) throws {
    let summary = rustConfigurationSummary
    guard identity.packageName == "refloat", summary.packageVersion == identity.packageVersion,
          summary.schemaFingerprint == identity.schemaFingerprint,
          summary.wireSignature == identity.wireSignature,
          sourceRevision != nil, rustConfigurationCurrent?.revision == sourceRevision,
          rustConfigurationSchema != nil,
          values.values.allSatisfy({
            if case let .double(value) = $0 { return value.isFinite }
            return true
          })
    else { throw RefloatConfigurationServiceError.invalidState }
  }

  private static func refloatConfigurationValuesJSON(
    _ values: [String: RefloatConfigurationValue]
  ) throws -> String {
    let object = values.mapValues { value -> [String: Any] in
      switch value {
      case let .double(value): ["type": "double", "value": value]
      case let .integer(value): ["type": "integer", "value": value]
      case let .boolean(value): ["type": "boolean", "value": value]
      case let .string(value): ["type": "string", "value": value]
      }
    }
    return try String(decoding: JSONSerialization.data(withJSONObject: object, options: [.sortedKeys]), as: UTF8.self)
  }

  private static func refloatConfigurationValues(
    json: String
  ) throws -> [String: RefloatConfigurationValue] {
    guard let object = try JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: [String: Any]]
    else { throw ProductStoreError.invalidRefloatConfiguration }
    var result: [String: RefloatConfigurationValue] = [:]
    for (name, typed) in object {
      switch typed["type"] as? String {
      case "double":
        guard let value = typed["value"] as? NSNumber,
              CFGetTypeID(value) != CFBooleanGetTypeID(), value.doubleValue.isFinite
        else { throw ProductStoreError.invalidRefloatConfiguration }
        result[name] = .double(value.doubleValue)
      case "integer":
        guard let value = typed["value"] as? NSNumber,
              CFGetTypeID(value) != CFBooleanGetTypeID(),
              value.doubleValue.rounded() == value.doubleValue
        else { throw ProductStoreError.invalidRefloatConfiguration }
        result[name] = .integer(value.int64Value)
      case "boolean":
        guard let value = typed["value"] as? Bool else { throw ProductStoreError.invalidRefloatConfiguration }
        result[name] = .boolean(value)
      case "string":
        guard let value = typed["value"] as? String else { throw ProductStoreError.invalidRefloatConfiguration }
        result[name] = .string(value)
      default: throw ProductStoreError.invalidRefloatConfiguration
      }
    }
    return result
  }

  private func finishRefloatConfigurationRequest(_ result: Result<Void, any Error>) {
    let waiter = refloatConfigurationWaiter
    refloatConfigurationWaiter = nil
    refloatConfigurationObservedPending = false
    waiter?.finish(result)
  }

  private func beginReconnect() {
    guard reconnectDurationMs > 0 else {
      finishReconnect(reason: "unexpected_disconnect")
      return
    }
    if reconnectStartedAtMs == nil {
      reconnectStartedAtMs = clock.monotonicMilliseconds()
      reconnectAttempt = 0
    }
    transition(.reconnecting)
    scheduleReconnectAttempt()
  }

  private func scheduleReconnectAttempt() {
    guard let scheduler,
          reconnectTimer == nil,
          reconnectStartedAtMs != nil
    else {
      if scheduler == nil { finishReconnect(reason: "reconnect_unavailable") }
      return
    }
    let elapsed = clock.monotonicMilliseconds() - (reconnectStartedAtMs ?? 0)
    let remaining = reconnectDurationMs - elapsed
    guard remaining > 0 else {
      finishReconnect(reason: "reconnect_timeout")
      return
    }
    let exponent = min(reconnectAttempt, 5)
    let delay = min(Int64(1_000) << exponent, Self.maxReconnectBackoffMs, remaining)
    reconnectTimer = scheduler.schedule(afterMilliseconds: delay) { [weak self] in
      self?.performReconnectAttempt()
    }
  }

  private func performReconnectAttempt() {
    lock.lock()
    defer { lock.unlock() }
    reconnectTimer = nil
    guard connectionState == .reconnecting, let deviceID = activeDeviceID else { return }
    let elapsed = clock.monotonicMilliseconds() - (reconnectStartedAtMs ?? 0)
    guard elapsed < reconnectDurationMs else {
      finishReconnect(reason: "reconnect_timeout")
      return
    }
    guard bluetoothState.radio == .poweredOn else {
      scheduleReconnectAttempt()
      return
    }
    reconnectAttempt += 1
    bluetooth.connect(deviceID)
  }

  private func finishReconnect(reason: String) {
    let deviceID = activeDeviceID
    cancelReconnect()
    if rustSessionAttached {
      try? performRustCommand(["command": "detach_session"])
      rustSessionAttached = false
    }
    if trackingMode == .saved, let active = logStore.active() {
      do {
        try events.publish(
          logStore.close(jobID: active.jobID, at: clock.now(), reason: reason)
        )
      } catch {
        events.publish(String(describing: error))
      }
    }
    explicitDisconnect = true
    if let deviceID { bluetooth.disconnect(deviceID) }
    activeDeviceID = nil
    activeDeviceName = nil
    latestPersistedSample = nil
    latestLocation = nil
    try? performRustCommand(["command": "reset", "now_ms": clock.monotonicMilliseconds()])
    transition(.disconnected)
    events.publish(reason)
  }

  private func cancelReconnect() {
    reconnectTimer?.cancel()
    reconnectTimer = nil
    reconnectStartedAtMs = nil
    reconnectAttempt = 0
  }

  private static func stableBoardIdentity(_ value: Data?) -> String? {
    guard let value, value.count == 12, value.contains(where: { $0 != 0 }) else { return nil }
    var input = Data("float-view-board-v1\0".utf8)
    input.append(value)
    let digest = SHA256.hash(data: input)
    return "vesc-v1-" + digest.map { String(format: "%02x", $0) }.joined()
  }

  private static func preferredCANID(_ json: String, key: String) -> UInt8? {
    guard let object = try? JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any],
          let number = object[key] as? NSNumber,
          CFGetTypeID(number) != CFBooleanGetTypeID(),
          number.intValue >= 0, number.intValue <= Int(UInt8.max)
    else { return nil }
    return UInt8(number.intValue)
  }

  private static let discoveryPublishIntervalMs: Int64 = 250
  private static let discoveryExpiryMs: Int64 = 120_000
  private static let maxReconnectBackoffMs: Int64 = 30_000
  private static let configurationReadHostWaitTimeout: TimeInterval = 3.25
  private static let configurationWriteHostWaitTimeout: TimeInterval = 3.25

  private func transition(_ state: ConnectionState) {
    connectionState = state
    events.publish(state)
  }

  private func publishDiscoveryUpdate(_ device: KnownDevice) {
    guard activeScanID != nil, let scheduler else {
      events.publish(device)
      return
    }
    discoveryUpdatePending = true
    guard discoveryPublishTimer == nil else { return }
    discoveryPublishTimer = scheduler.schedule(
      afterMilliseconds: Self.discoveryPublishIntervalMs
    ) { [weak self] in
      self?.lock.withLock {
        guard let self else { return }
        self.discoveryPublishTimer = nil
        guard self.discoveryUpdatePending else { return }
        self.discoveryUpdatePending = false
        self.events.publish(self.discoverySnapshot())
      }
    }
  }

  private func savedDeviceID(for deviceID: DeviceID) -> String? {
    guard let productStore else { return nil }
    return (try? productStore.savedDevices())?.first {
      $0.bluetoothIdentifier == deviceID.value
    }?.savedDeviceID
  }

  private func cancelPendingDiscoveryPublish() {
    discoveryPublishTimer?.cancel()
    discoveryPublishTimer = nil
    discoveryUpdatePending = false
  }

  private func scheduleDiscoveryExpiry() {
    guard activeScanID != nil, let scheduler else { return }
    discoveryExpiryTimer?.cancel()
    discoveryExpiryGeneration += 1
    let generation = discoveryExpiryGeneration
    discoveryExpiryTimer = scheduler.schedule(
      afterMilliseconds: Self.discoveryExpiryMs
    ) { [weak self] in
      self?.lock.withLock {
        guard let self, self.discoveryExpiryGeneration == generation else { return }
        self.discoveryExpiryTimer = nil
        self.clearDiscoveredDevices(publish: true)
      }
    }
  }

  private func clearDiscoveredDevices(publish: Bool) {
    discoveryExpiryGeneration += 1
    discoveryExpiryTimer?.cancel()
    discoveryExpiryTimer = nil
    cancelPendingDiscoveryPublish()
    guard !knownDevicesByID.isEmpty else { return }
    knownDevicesByID.removeAll()
    if publish { events.publish(discoverySnapshot()) }
  }

  private func sessionPath(_ sessionID: String) throws -> String {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionURL(sessionID: sessionID).path
  }

  private func rustSessionQuery(_ command: [String: Any]) throws -> [String: Any] {
    try lock.withLock {
      rustSessionResult = nil
      try performRustCommand(command)
      guard let result = rustSessionResult else { throw RustApplicationRuntimeError.internalFailure }
      rustSessionResult = nil
      return result
    }
  }

  private static func boundary(
    _ snapshot: SessionPublishedSnapshotRecord,
    sampleSequence: Int64? = nil,
    eventSequence: Int64? = nil,
    locationSequence: Int64? = nil
  ) -> [String: Any] {
    [
      "history_revision": snapshot.historyRevision,
      "sample_sequence": sampleSequence ?? snapshot.committedSampleSequence,
      "event_sequence": eventSequence ?? snapshot.committedEventSequence,
      "location_sequence": locationSequence ?? snapshot.committedLocationSequence,
    ]
  }

  private func rustSessionSamples(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxRows: Int,
    publishedSampleSequence: Int64?
  ) throws -> [SessionSeriesPoint] {
    guard maxRows > 0 else { throw SessionStoreError.invalidRange }
    let snapshot = try sessionSnapshot(sessionID: sessionID)
    let path = try sessionPath(sessionID)
    var rows: [SessionSeriesPoint] = []
    var afterElapsed: Any = NSNull()
    var afterSequence: Any = NSNull()
    while rows.count < maxRows {
      let value = try rustSessionQuery([
        "command": "session_samples", "path": path, "fields": fields,
        "start_elapsed_ms": startElapsedMs, "end_elapsed_ms": endElapsedMs,
        "limit": min(2_000, maxRows - rows.count),
        "after_elapsed_ms": afterElapsed, "after_sequence": afterSequence,
        "boundary": Self.boundary(snapshot, sampleSequence: publishedSampleSequence),
      ])
      let page = (value["rows"] as? [[String: Any]] ?? []).compactMap { row -> SessionSeriesPoint? in
        guard let elapsed = Self.int64(row["elapsed_ms"]), let sequence = Self.int64(row["sequence"]),
              let rawValues = row["values"] as? [Any], rawValues.count == fields.count
        else { return nil }
        var values: [String: Double?] = [:]
        for (field, raw) in zip(fields, rawValues) {
          values[field] = raw is NSNull ? nil : Self.double(raw)
        }
        return SessionSeriesPoint(
          elapsedMs: elapsed, values: values, sequence: sequence,
          rideState: row["ride_state"] as? String
        )
      }
      rows.append(contentsOf: page)
      guard let next = value["next"] as? [String: Any],
            let elapsed = Self.int64(next["elapsed_ms"]),
            let sequence = Self.int64(next["sequence"])
      else { break }
      afterElapsed = elapsed
      afterSequence = sequence
    }
    return rows
  }

  private func rustPrune(
    sessionID: String, targetResolutionSeconds: Int64, commit: Bool
  ) throws -> SessionPrunePreviewRecord {
    let value = try rustSessionQuery([
      "command": "session_prune", "path": sessionPath(sessionID),
      "target_resolution_seconds": targetResolutionSeconds, "commit": commit,
      "committed_at_epoch_ms": Int64(clock.now().timeIntervalSince1970 * 1_000),
    ])
    guard let recorded = Self.int64(value["recorded_sample_count"]),
          let retained = Self.int64(value["retained_sample_count"]),
          let reclaimable = Self.int64(value["estimated_reclaimable_bytes"]),
          let skipped = value["skipped"] as? Bool
    else { throw RustApplicationRuntimeError.internalFailure }
    return SessionPrunePreviewRecord(
      recordedSampleCount: recorded, estimatedStoredSampleCount: retained,
      estimatedReclaimableBytes: reclaimable, skipped: skipped,
      skipReason: skipped ? "already_at_or_below_target" : nil
    )
  }

  private static func int64s(_ value: Any?) -> [Int64]? {
    (value as? [Any])?.compactMap(int64)
  }

  private static func ints(_ value: Any?) -> [Int]? {
    int64s(value).map { $0.map(Int.init) }
  }

  private static func optionalDoubles(_ value: Any?) -> [Double?]? {
    guard let values = value as? [Any] else { return nil }
    return values.map { $0 is NSNull ? nil : Self.double($0) }
  }

  private func beginRustApplication() throws {
    let saved = try? productStore?.savedDevices().first {
      $0.bluetoothIdentifier == activeDeviceID?.value
    }
    currentSavedDevice = saved ?? nil
    let profile: BoardProfileRecord? = (try? saved.flatMap {
      try productStore?.boardProfile(savedDeviceID: $0.savedDeviceID)
    }) ?? nil
    let hint: ControllerRoutingHint? = (try? saved.flatMap {
      try productStore?.controllerRoutingHint(savedDeviceID: $0.savedDeviceID)
    }) ?? nil
    try performRustCommand([
      "command": "begin",
      "now_ms": clock.monotonicMilliseconds(),
      "idle_interval_ms": idleProbeIntervalMs,
      "post_ride_tail_ms": samplingSettings.postRideTailMs,
      "preferred_controller_can_id": profile.flatMap {
        Self.preferredCANID($0.valuesJSON, key: "preferredCanId")
      } ?? NSNull(),
      "preferred_bms_can_id": profile.flatMap {
        Self.preferredCANID($0.valuesJSON, key: "preferredBmsCanId")
      } ?? NSNull(),
      "routing_hint_can_id": hint?.canID ?? NSNull(),
      "routing_hint_controller_uuid": hint?.controllerUUID.map(Array.init) ?? NSNull(),
      "has_active_session": trackingMode == .saved && logStore.active() != nil,
      "continuity_can_id": sessionControllerRoute?.canID ?? NSNull(),
      "continuity_controller_uuid": sessionControllerUUID.map(Array.init) ?? NSNull(),
    ])
  }

  private func performRustCommand(_ command: [String: Any]) throws {
    let result: RustApplicationRuntimeResult
    do {
      result = try rustRuntime.apply(command)
    } catch {
      let name = command["command"] as? String ?? "invalid"
      events.publish("rust_runtime_apply_failure_\(name)_\(String(describing: error))")
      if let runtimeError = error as? RustApplicationRuntimeError {
        switch (name, runtimeError) {
        case let (command, .invalidState)
          where ["set_control", "clear_fatal_error", "shutdown_controller"].contains(command):
          throw RefloatMutationError.invalidState
        case let (command, .invalidState)
          where ["read_configuration", "write_configuration"].contains(command):
          throw RefloatConfigurationServiceError.invalidState
        case let (command, .conflict)
          where ["read_configuration", "write_configuration"].contains(command):
          throw RefloatConfigurationServiceError.conflict
        case let (command, .unavailable)
          where ["read_configuration", "write_configuration"].contains(command):
          throw RefloatConfigurationServiceError.unavailable
        default: break
        }
      }
      throw error
    }
    rustGeneration = result.generation
    for effect in result.effects {
      do {
        try performRustEffect(effect)
      } catch {
        let kind = effect["effect"] as? String ?? "invalid"
        events.publish("rust_runtime_effect_failure_\(kind)")
        throw error
      }
    }
  }

  private func performRustEffect(_ effect: [String: Any]) throws {
    guard let kind = effect["effect"] as? String else {
      throw RustApplicationRuntimeError.internalFailure
    }
    switch kind {
    case "write_bytes":
      guard let deviceID = activeDeviceID, let bytes = Self.bytes(effect["bytes"]) else {
        throw RustApplicationRuntimeError.internalFailure
      }
      bluetooth.write(Data(bytes), to: deviceID)
    case "arm_deadline":
      try armRustTimer(effect, repeating: false, deadline: true)
    case "replace_timer":
      try armRustTimer(effect, repeating: true, deadline: false)
    case "cancel_deadline":
      guard let token = Self.uint64(effect["token"]) else { throw RustApplicationRuntimeError.internalFailure }
      rustDeadlines.removeValue(forKey: token)?.cancel()
    case "cancel_timer":
      guard let token = Self.uint64(effect["token"]) else { throw RustApplicationRuntimeError.internalFailure }
      rustTimers.removeValue(forKey: token)?.cancel()
      rustTimerIntervals.removeValue(forKey: token)
    case "clear_routing_hint":
      if let savedDeviceID = currentSavedDevice?.savedDeviceID {
        try? productStore?.clearControllerRoutingHint(savedDeviceID: savedDeviceID)
      }
    case "store_routing_hint":
      try storeRustRoutingHint(effect)
    case "prepare_activation":
      try prepareRustActivation(effect)
    case "publish_state":
      try publishRustState(effect)
    case "publish_telemetry":
      try publishRustTelemetry(effect)
    case "publish_bms_state":
      try publishRustBmsState(effect)
    case "publish_bms_observation":
      try publishRustBmsObservation(effect)
    case "publish_session_marker":
      try applyRustSessionMarker(effect)
      events.publish("rust_session_commit")
    case "session_result":
      rustSessionResult = effect["value"] as? [String: Any]
      if rustSessionResult == nil, let values = effect["value"] as? [[String: Any]] {
        rustSessionResult = ["rows": values]
      }
    case "store_configuration_cache":
      if let identity = Self.cacheIdentity(effect["identity"]), let bytes = Self.bytes(effect["bytes"]) {
        productStore?.storeRefloatSchema(Data(bytes), identity: identity)
      }
    case "remove_configuration_cache":
      if let identity = Self.cacheIdentity(effect["identity"]) {
        productStore?.removeCachedRefloatSchema(identity: identity)
      }
    case "mutation":
      let value = effect["value"] as? String ?? ""
      if value.contains("Failed") {
        refloatWaiter?.finish(.failure(.unavailable))
        refloatWaiter = nil
      } else if value.contains("Idempotent") || value.contains("Confirmed") || value.contains("Released") {
        if let rustControls { refloatWaiter?.finish(.success(rustControls)) }
        refloatWaiter = nil
      }
    case "failure":
      let failure = effect["kind"] as? String ?? ""
      events.publish("rust_runtime_failure")
      if failure.hasPrefix("Mutation") {
        refloatWaiter?.finish(.failure(.unavailable))
        refloatWaiter = nil
      }
      if failure.hasPrefix("Configuration")
        || failure.contains("ConfigurationWrite")
        || failure.contains("TuneApply")
      {
        finishRefloatConfigurationRequest(.failure(Self.configurationFailure(failure)))
      }
    case "diagnostic":
      events.publish("rust_runtime_diagnostic")
    default:
      throw RustApplicationRuntimeError.internalFailure
    }
  }

  private func armRustTimer(_ effect: [String: Any], repeating: Bool, deadline: Bool) throws {
    guard let token = Self.uint64(effect["token"]),
          let interval = Self.int64(effect["timeout_ms"] ?? effect["interval_ms"]),
          interval > 0
    else { throw RustApplicationRuntimeError.internalFailure }
    let command = deadline ? "deadline_fired" : "timer_fired"
    if repeating, rustTimerIntervals[token] == interval, rustTimers[token] != nil { return }
    let action: @Sendable () -> Void = { [weak self] in
      self?.lock.withLock {
        guard let self else { return }
        do {
          try self.performRustCommand([
            "command": command, "generation": self.rustGeneration,
            "token": token, "now_ms": self.clock.monotonicMilliseconds(),
          ])
        } catch {
          self.events.publish("rust_runtime_timer_failure")
        }
        if deadline { self.rustDeadlines.removeValue(forKey: token)?.cancel() }
      }
    }
    let timer = scheduler ?? SystemScheduler()
    let cancellation = repeating || deadline
      ? timer.scheduleRepeating(everyMilliseconds: interval, action: action)
      : timer.schedule(afterMilliseconds: interval, action: action)
    if deadline {
      rustDeadlines.removeValue(forKey: token)?.cancel()
      rustDeadlines[token] = cancellation
    } else {
      rustTimers.removeValue(forKey: token)?.cancel()
      rustTimers[token] = cancellation
      rustTimerIntervals[token] = interval
    }
  }

  private func prepareRustActivation(_ effect: [String: Any]) throws {
    guard let generation = Self.uint64(effect["generation"]),
          let route = Self.controllerRoute(effect["selected_route"]),
          let firmware = Self.firmware(effect["selected_identity"], route: route),
          let packageVersion = effect["refloat_version"] as? String
    else { throw RustApplicationRuntimeError.internalFailure }
    rustGeneration = generation
    rustControllerRoute = route
    rustFirmware = firmware
    rustRefloatPackageVersion = packageVersion
    if trackingMode == .saved, let deviceID = activeDeviceID {
      currentSavedDevice = try productStore?.ensureSavedDevice(
        deviceID: deviceID, advertisedName: activeDeviceName,
        stableIdentity: Self.stableBoardIdentity(firmware.controllerUUID), at: clock.now()
      )
    }
    var sessionStarted = false
    if trackingMode == .saved, logStore.active() == nil, let deviceID = activeDeviceID {
      try events.publish(logStore.create(deviceID: deviceID, advertisedName: activeDeviceName, at: clock.now()))
      sessionStarted = true
      sessionControllerRoute = route
      sessionControllerUUID = firmware.controllerUUID
      sessionFirmware = firmware
      sessionRefloatPackageVersion = packageVersion
    }
    if !rustSessionAttached, let url = trackingMode == .saved ? logStore.activeSessionURL() : nil {
      try performRustCommand([
        "command": "attach_session", "path": url.path, "mode": "open",
        "expected_riding_persistence_ms": samplingSettings.ridingPersistIntervalMs,
        "expected_idle_persistence_ms": samplingSettings.idlePersistIntervalMs,
        "committed_at_epoch_ms": Int64(clock.now().timeIntervalSince1970 * 1_000),
      ])
      rustSessionAttached = true
    }
    var cached: Any = NSNull()
    if let identity = Self.cacheIdentity(effect["configuration_cache_identity"]),
       let bytes = productStore?.cachedRefloatSchema(identity: identity)
    { cached = Array(bytes) }
    try performRustCommand([
      "command": "activate", "generation": generation,
      "now_ms": clock.monotonicMilliseconds(), "session_started": sessionStarted,
      "cached_configuration_schema": cached,
    ])
  }

  private func storeRustRoutingHint(_ effect: [String: Any]) throws {
    guard trackingMode == .saved, let savedDeviceID = currentSavedDevice?.savedDeviceID,
          let route = Self.controllerRoute(effect["route"]), case let .can(canID) = route,
          let identity = effect["identity"] as? [String: Any], let uuid = Self.bytes(identity["controller_uuid"])
    else { return }
    try productStore?.storeControllerRoutingHint(
      ControllerRoutingHint(savedDeviceID: savedDeviceID, canID: canID, controllerUUID: Data(uuid)),
      at: clock.now()
    )
  }

  private func publishRustState(_ effect: [String: Any]) throws {
    guard let generation = Self.uint64(effect["generation"]), let phase = effect["phase"] as? String else {
      throw RustApplicationRuntimeError.internalFailure
    }
    rustGeneration = generation
    rustControllerRoute = Self.controllerRoute(effect["controller_route"])
    rustFirmware = Self.firmware(effect["selected_firmware"], route: rustControllerRoute)
    rustControllerCandidates = (effect["controller_candidates"] as? [[String: Any]] ?? []).compactMap {
      guard let canID = Self.uint8($0["can_id"]), let version = $0["vesc_version"] as? String,
            let supported = $0["vesc_supported"] as? Bool else { return nil }
      return ControllerCandidate(canID: canID, hardwareName: $0["hardware_name"] as? String,
                                 firmwareName: $0["firmware_name"] as? String,
                                 vescVersion: version, vescSupported: supported)
    }
    if let issue = effect["connection_issue"] as? [String: Any],
       let raw = issue["code"] as? String, let code = ConnectionIssueCode(rawValue: raw),
       let message = issue["message"] as? String
    {
      rustConnectionIssue = ConnectionIssue(code: code, message: message)
    } else { rustConnectionIssue = nil }
    if let limits = effect["current_limits"] as? [String: Any] {
      currentLimits = ControllerCurrentLimits(
        motorCurrentMaxA: Self.double(limits["motor_current_max_a"]) ?? 0,
        motorCurrentMinA: Self.double(limits["motor_current_min_a"]) ?? 0,
        batteryCurrentMaxA: Self.double(limits["battery_current_max_a"]) ?? 0,
        batteryCurrentMinA: Self.double(limits["battery_current_min_a"]) ?? 0,
        batterySeriesCells: Self.uint8(limits["battery_series_cells"])
      )
    } else { currentLimits = nil }
    rustControls = Self.controls(effect["controls"])
    if let configuration = effect["configuration"] as? [String: Any] {
      try updateRustConfiguration(configuration)
    }
    if let actions = effect["controller_actions"] as? [String: Any],
       let available = actions["shutdown_available"] as? Bool,
       let pending = actions["shutdown_pending"] as? Bool
    {
      rustControllerActions = ControllerActions(
        shutdownAvailable: available, pendingAction: pending ? .shutdown : nil
      )
    }
    deviceKind = (effect["standalone_bms"] as? Bool) == true ? .bms : (rustFirmware == nil ? nil : .board)
    let state: ConnectionState = switch phase {
    case "identifying", "awaitingactivation": .identifying
    case "selectingcontroller": .selectingController
    case "ready": .ready
    case "failed": .failed
    default: .disconnected
    }
    transition(state)
  }

  private func publishRustTelemetry(_ effect: [String: Any]) throws {
    guard let sequence = Self.int64(effect["sequence"]), let elapsed = Self.int64(effect["elapsed_ms"]),
          let observed = Self.int64(effect["observed_at_epoch_ms"]),
          let ride = Self.rideState(effect["ride_state"])
    else { throw RustApplicationRuntimeError.internalFailure }
    let extensions = (effect["extensions"] as? [[String: Any]] ?? []).reduce(into: [String: Double]()) {
      if let name = $1["name"] as? String, let value = Self.double($1["value"]) { $0[name] = value }
    }
    let sample = TelemetrySample(
      sequence: sequence, observedAt: Date(timeIntervalSince1970: Double(observed) / 1_000),
      slowObservedAt: Self.int64(effect["slow_observed_at_epoch_ms"])
        .map { Date(timeIntervalSince1970: Double($0) / 1_000) },
      elapsedMs: elapsed, rideState: ride,
      packageState: Self.enumValue(effect["package_state"], PackageState.self),
      packageMode: Self.packageMode(effect["package_mode"]),
      footpadState: Self.enumValue(effect["footpad_state"], FootpadState.self),
      stopCondition: Self.stopCondition(effect["stop_condition"]),
      setpointAdjustment: Self.setpointAdjustment(effect["setpoint_adjustment"]),
      wheelslip: effect["wheelslip"] as? Bool, darkride: effect["darkride"] as? Bool,
      charging: effect["charging"] as? Bool, alertReason: Self.alertReason(effect["alert_reason"]),
      firmwareFault: Self.firmwareFault(effect["firmware_fault"]),
      packVoltageV: Self.double(effect["pack_voltage_v"]), dutyCycle: Self.double(effect["duty_cycle"]),
      controllerTempC: Self.double(effect["controller_temp_c"]), motorTempC: Self.double(effect["motor_temp_c"]),
      motorCurrentA: Self.double(effect["motor_current_a"]), batteryCurrentA: Self.double(effect["battery_current_a"]),
      erpm: Self.double(effect["erpm"]), speedKph: Self.double(effect["speed_kph"]),
      pitchDeg: Self.double(effect["pitch_deg"]), balancePitchDeg: Self.double(effect["balance_pitch_deg"]),
      rollDeg: Self.double(effect["roll_deg"]),
      directionalMotorCurrentA: Self.double(effect["directional_motor_current_a"]),
      filteredMotorCurrentA: Self.double(effect["filtered_motor_current_a"]),
      requestedBalanceCurrentA: Self.double(effect["requested_balance_current_a"]),
      boosterCurrentA: Self.double(effect["booster_current_a"]),
      fieldWeakeningCurrentA: Self.double(effect["field_weakening_current_a"]),
      footpadAdc1V: Self.double(effect["footpad_adc1_v"]), footpadAdc2V: Self.double(effect["footpad_adc2_v"]),
      remoteInput: Self.double(effect["remote_input"]), setpointDeg: Self.double(effect["setpoint_deg"]),
      atrSetpointDeg: Self.double(effect["atr_setpoint_deg"]),
      brakeTiltSetpointDeg: Self.double(effect["brake_tilt_setpoint_deg"]),
      torqueTiltSetpointDeg: Self.double(effect["torque_tilt_setpoint_deg"]),
      turnTiltSetpointDeg: Self.double(effect["turn_tilt_setpoint_deg"]),
      remoteSetpointDeg: Self.double(effect["remote_setpoint_deg"]),
      atrAccelerationDifferenceErpmPerSecond: Self.double(effect["atr_acceleration_difference_erpm_per_second"]),
      atrSpeedBoost: Self.double(effect["atr_speed_boost"]), distanceM: Self.double(effect["distance_m"]),
      odometerM: Self.double(effect["odometer_m"]), ampHoursDrawnAh: Self.double(effect["amp_hours_drawn_ah"]),
      ampHoursChargedAh: Self.double(effect["amp_hours_charged_ah"]),
      wattHoursDrawnWh: Self.double(effect["watt_hours_drawn_wh"]),
      wattHoursChargedWh: Self.double(effect["watt_hours_charged_wh"]),
      batteryLevel: Self.double(effect["battery_level"]),
      chargingCurrentA: Self.double(effect["charging_current_a"]),
      chargingVoltageV: Self.double(effect["charging_voltage_v"]),
      extensionFields: extensions
    )
    latestPersistedSample = sample
    updateLocationCapture(rideState: ride)
    events.publish(sample)
  }

  private func updateRustConfiguration(_ value: [String: Any]) throws {
    guard let rawAvailability = value["availability"] as? String,
          let availability = RefloatConfigurationAvailability(rawValue: rawAvailability),
          let writable = value["writable"] as? Bool
    else { throw RustApplicationRuntimeError.internalFailure }
    let identity = value["identity"] as? [String: Any]
    let pending: RefloatConfigurationOperation? = switch value["pending"] as? String {
    case "schema": .schema
    case "readcurrent": .readCurrent
    case "readdefault": .readDefault
    case "write": .write
    case nil: nil
    default: throw RustApplicationRuntimeError.internalFailure
    }
    rustConfigurationSchema = try Self.configurationSchema(value["schema"])
    rustConfigurationCurrent = try Self.configurationValues(
      value["current_values"], revision: Self.uint64(value["current_revision"]), source: "current"
    )
    rustConfigurationDefaults = try Self.configurationValues(
      value["default_values"], revision: Self.uint64(value["default_revision"]), source: "default"
    )
    rustConfigurationSummary = RefloatConfigurationSummary(
      availability: availability, writable: writable,
      packageVersion: identity?["package_version"] as? String,
      schemaFingerprint: identity?["fingerprint"] as? String,
      wireSignature: Self.uint64(identity?["wire_signature"]).map(UInt32.init),
      currentLoaded: rustConfigurationCurrent != nil, defaultLoaded: rustConfigurationDefaults != nil,
      pendingOperation: pending
    )
    events.publish(rustConfigurationSummary)
    if pending != nil, refloatConfigurationWaiter != nil {
      refloatConfigurationObservedPending = true
    } else if pending == nil, refloatConfigurationObservedPending {
      finishRefloatConfigurationRequest(.success(()))
    }
  }

  private func publishRustBmsState(_ effect: [String: Any]) throws {
    guard let raw = effect["status"] as? String, let status = BmsStatus(rawValue: raw) else {
      throw RustApplicationRuntimeError.internalFailure
    }
    let connection = Self.bmsConnection(effect["route"])
    let candidates = (effect["candidates"] as? [[String: Any]] ?? []).compactMap { item -> BmsCandidate? in
      guard let version = item["version"] as? String else { return nil }
      let route: BmsConnection = Self.uint8(item["can_id"]).map(BmsConnection.can) ?? .direct
      return BmsCandidate(connection: route, hardwareName: item["hardware_name"] as? String, vescVersion: version)
    }
    rustBmsState = BmsState(status: status, connection: connection, candidates: candidates,
                            latestSnapshot: rustBmsState.latestSnapshot)
    events.publish(rustBmsState)
  }

  private func publishRustBmsObservation(_ effect: [String: Any]) throws {
    guard let observed = Self.int64(effect["observed_at_epoch_ms"]),
          let total = Self.double(effect["total_voltage_v"]), let charge = Self.double(effect["charge_voltage_v"]),
          let input = Self.double(effect["input_current_a"]), let icInput = Self.double(effect["input_ic_current_a"]),
          let ampHours = Self.double(effect["session_amp_hours_ah"]),
          let wattHours = Self.double(effect["session_watt_hours_wh"]),
          let cells = Self.doubles(effect["cell_voltages_v"]), let balancing = effect["balancing"] as? [Bool],
          let temperatures = Self.doubles(effect["temperatures_c"]),
          let icTemperature = Self.double(effect["ic_temperature_c"]),
          let humidityTemperature = Self.double(effect["humidity_sensor_temperature_c"]),
          let humidity = Self.double(effect["humidity_percent"]),
          let highest = Self.double(effect["highest_cell_temperature_c"]),
          let chargeState = Self.double(effect["state_of_charge"]), let health = Self.double(effect["state_of_health"]),
          let reportedCanID = Self.uint8(effect["reported_can_id"]),
          let lifetimeChargeAh = Self.double(effect["lifetime_charge_amp_hours_ah"]),
          let lifetimeChargeWh = Self.double(effect["lifetime_charge_watt_hours_wh"]),
          let lifetimeDischargeAh = Self.double(effect["lifetime_discharge_amp_hours_ah"]),
          let lifetimeDischargeWh = Self.double(effect["lifetime_discharge_watt_hours_wh"]),
          let pressure = Self.double(effect["pressure_pa"])
    else { throw RustApplicationRuntimeError.internalFailure }
    let snapshot = BmsSnapshot(
      observedAt: Date(timeIntervalSince1970: Double(observed) / 1_000), totalVoltageV: total,
      chargeVoltageV: charge, inputCurrentA: input, inputICCurrentA: icInput,
      sessionAmpHoursAh: ampHours, sessionWattHoursWh: wattHours, cellVoltagesV: cells,
      balancing: balancing, temperaturesC: temperatures, icTemperatureC: icTemperature,
      humiditySensorTemperatureC: humidityTemperature, humidityPercent: humidity,
      highestCellTemperatureC: highest, stateOfCharge: chargeState, stateOfHealth: health,
      reportedCanID: reportedCanID, lifetimeChargeAmpHoursAh: lifetimeChargeAh,
      lifetimeChargeWattHoursWh: lifetimeChargeWh, lifetimeDischargeAmpHoursAh: lifetimeDischargeAh,
      lifetimeDischargeWattHoursWh: lifetimeDischargeWh, pressurePa: pressure,
      dataVersion: Self.uint8(effect["data_version"]), status: effect["status"] as? String
    )
    rustBmsState = BmsState(status: .attached, connection: Self.bmsConnection(effect["route"]),
                            candidates: rustBmsState.candidates, latestSnapshot: snapshot)
    events.publish(snapshot)
  }

  private func applyRustSessionMarker(_ effect: [String: Any]) throws {
    guard let active = logStore.active(),
          let historyRevision = Self.int64(effect["history_revision"]),
          let sampleSequence = Self.int64(effect["committed_sample_sequence"]),
          let eventSequence = Self.int64(effect["committed_event_sequence"]),
          let elapsed = Self.int64(effect["committed_elapsed_ms"]),
          let recordedSamples = Self.int64(effect["recorded_sample_count"]),
          let storedSamples = Self.int64(effect["stored_sample_count"]),
          let locationSequence = Self.int64(effect["committed_location_sequence"]),
          let locationElapsed = Self.int64(effect["committed_location_elapsed_ms"]),
          let recordedLocations = Self.int64(effect["recorded_location_count"]),
          let storedLocations = Self.int64(effect["stored_location_count"]),
          let locationStatus = effect["last_location_status"] as? String,
          let eventCount = Self.int64(effect["committed_event_count"]),
          let committedAt = Self.int64(effect["committed_at_epoch_ms"]),
          let ridingElapsed = Self.int64(effect["riding_elapsed_ms"])
    else { throw RustApplicationRuntimeError.internalFailure }
    let snapshot = SessionPublishedSnapshotRecord(
      historyRevision: historyRevision, committedSampleSequence: sampleSequence,
      committedEventSequence: eventSequence, committedElapsedMs: elapsed,
      recordedSampleCount: recordedSamples, storedSampleCount: storedSamples,
      committedLocationSequence: locationSequence, committedLocationElapsedMs: locationElapsed,
      recordedLocationCount: recordedLocations, storedLocationCount: storedLocations,
      locationStatus: locationStatus
    )
    try sessionStore?.applySessionCommitMarker(
      sessionID: active.sessionID.value,
      marker: SessionCommitMarkerRecord(
        snapshot: snapshot, committedEventCount: eventCount,
        committedAt: Date(timeIntervalSince1970: Double(committedAt) / 1_000),
        ridingElapsedMs: ridingElapsed, distanceM: Self.double(effect["distance_m"]),
        maximumSpeedKph: Self.double(effect["maximum_speed_kph"]),
        maximumSpeedSourceSequence: Self.int64(effect["maximum_speed_source_sequence"]),
        maximumDutyCycle: Self.double(effect["maximum_duty_cycle"]),
        maximumDutySourceSequence: Self.int64(effect["maximum_duty_source_sequence"]),
        lastSpeedKph: Self.double(effect["last_speed_kph"]),
        lastRideState: effect["last_ride_state"] as? String
      )
    )
  }

  private static func uint64(_ value: Any?) -> UInt64? {
    guard let number = value as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
          number.doubleValue >= 0, number.doubleValue.rounded() == number.doubleValue
    else { return nil }
    return number.uint64Value
  }

  private static func int64(_ value: Any?) -> Int64? {
    guard let number = value as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
          number.doubleValue.rounded() == number.doubleValue
    else { return nil }
    return number.int64Value
  }

  private static func uint8(_ value: Any?) -> UInt8? {
    guard let value = uint64(value), value <= UInt8.max else { return nil }
    return UInt8(value)
  }

  private static func double(_ value: Any?) -> Double? {
    guard let number = value as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
          number.doubleValue.isFinite else { return nil }
    return number.doubleValue
  }

  private static func bytes(_ value: Any?) -> [UInt8]? {
    guard let values = value as? [Any] else { return nil }
    let bytes = values.compactMap(uint8)
    return bytes.count == values.count ? bytes : nil
  }

  private static func doubles(_ value: Any?) -> [Double]? {
    guard let values = value as? [Any] else { return nil }
    let numbers = values.compactMap(double)
    return numbers.count == values.count ? numbers : nil
  }

  private static func enumValue<T: RawRepresentable>(_ value: Any?, _: T.Type) -> T?
    where T.RawValue == String
  {
    (value as? String).flatMap(T.init(rawValue:))
  }

  private static func rideState(_ value: Any?) -> RideState? {
    enumValue(value, RideState.self)
  }

  private static func packageMode(_ value: Any?) -> PackageMode? {
    switch value as? String {
    case "handtest": .handTest
    case let value?: PackageMode(rawValue: value)
    case nil: nil
    }
  }

  private static func stopCondition(_ value: Any?) -> StopCondition? {
    switch value as? String {
    case "switchhalf": .switchHalf
    case "switchfull": .switchFull
    case "reversestop": .reverseStop
    case "quickstop": .quickStop
    case let value?: StopCondition(rawValue: value)
    case nil: nil
    }
  }

  private static func setpointAdjustment(_ value: Any?) -> SetpointAdjustment? {
    switch value as? String {
    case "reversestop": .reverseStop
    case let value?: SetpointAdjustment(rawValue: value)
    case nil: nil
    }
  }

  private static func alertReason(_ value: Any?) -> AlertReason? {
    guard let raw = value as? String else { return nil }
    return AlertReason.allCases.first { String(describing: $0).lowercased() == raw }
  }

  private static func firmwareFault(_ value: Any?) -> FirmwareFault? {
    guard let raw = value as? String else { return nil }
    return FirmwareFault.allCases.first { String(describing: $0).lowercased() == raw }
  }

  private static func controllerRoute(_ value: Any?) -> ControllerRoute? {
    guard let value = value as? [String: Any], let kind = value["kind"] as? String else { return nil }
    if kind == "direct" { return .direct }
    if kind == "can", let canID = uint8(value["can_id"]) { return .can(canID) }
    return nil
  }

  private static func bmsConnection(_ value: Any?) -> BmsConnection? {
    switch controllerRoute(value) {
    case .direct: .direct
    case let .can(canID): .can(canID)
    case nil: nil
    }
  }

  private static func firmware(_ value: Any?, route: ControllerRoute?) -> FirmwareRecord? {
    guard let value = value as? [String: Any], let major = uint8(value["major"]),
          let minor = uint8(value["minor"]), let test = uint8(value["test_version"]),
          let hardwareType = uint8(value["hardware_type"]),
          let customCount = uint8(value["custom_configuration_count"])
    else { return nil }
    return FirmwareRecord(
      major: major, minor: minor, testVersion: test, hardwareName: value["hardware_name"] as? String,
      firmwareName: value["firmware_name"] as? String, hardwareType: hardwareType,
      canID: route?.canID, controllerUUID: bytes(value["controller_uuid"]).map { Data($0) },
      customConfigurationCount: customCount,
      hardwareConfigurationCRC: uint64(value["hardware_configuration_crc"]).map(UInt32.init)
    )
  }

  private static func controls(_ value: Any?) -> RefloatControls? {
    guard let value = value as? [String: Any], let raw = value["status"] as? String,
          let status = RefloatControlStatus(rawValue: raw), let writable = value["writable"] as? Bool,
          let lighting = value["lighting_available"] as? Bool,
          let clearAvailable = value["fatal_error_clear_available"] as? Bool
    else { return nil }
    let pending = (value["pending_action"] as? String).flatMap { raw -> RefloatPendingAction? in
      switch raw {
      case "clearfatalerror": .clearFatalError
      default: RefloatPendingAction(rawValue: raw)
      }
    }
    return RefloatControls(
      status: status, writable: writable, packageEnabled: value["package_enabled"] as? Bool,
      flywheelEnabled: value["flywheel_enabled"] as? Bool, lightingAvailable: lighting,
      ledsEnabled: value["leds_enabled"] as? Bool, headlightsEnabled: value["headlights_enabled"] as? Bool,
      fatalErrorClearAvailable: clearAvailable, fatalErrorLatched: value["fatal_error_latched"] as? Bool,
      fatalErrorActive: value["fatal_error_active"] as? Bool,
      fatalErrorClearable: value["fatal_error_clearable"] as? Bool, pendingAction: pending
    )
  }

  private static func configurationFailure(_ value: String) -> RefloatConfigurationServiceError {
    if value.contains("Conflict") { return .conflict }
    if value.contains("StaleResponse") { return .staleResponse }
    if value.contains("UnauditedSchema") { return .unauditedSchema }
    if value.contains("MalformedResponse") || value.contains("Codec") { return .malformedResponse }
    if value.contains("InvalidState") { return .invalidState }
    return .unavailable
  }

  private static func configurationValue(_ value: Any?) throws -> RefloatConfigurationValue {
    guard let value = value as? [String: Any], let type = value["type"] as? String else {
      throw RustApplicationRuntimeError.internalFailure
    }
    switch type {
    case "double":
      guard let number = double(value["value"]) else { throw RustApplicationRuntimeError.internalFailure }
      return .double(number)
    case "integer":
      guard let number = int64(value["value"]) else { throw RustApplicationRuntimeError.internalFailure }
      return .integer(number)
    case "boolean":
      guard let boolean = value["value"] as? Bool else { throw RustApplicationRuntimeError.internalFailure }
      return .boolean(boolean)
    case "string":
      guard let string = value["value"] as? String else { throw RustApplicationRuntimeError.internalFailure }
      return .string(string)
    default: throw RustApplicationRuntimeError.internalFailure
    }
  }

  private static func rustConfigurationValues(
    _ values: [String: RefloatConfigurationValue]
  ) -> [String: Any] {
    values.mapValues { value in
      switch value {
      case let .double(value): ["type": "double", "value": value]
      case let .integer(value): ["type": "integer", "value": value]
      case let .boolean(value): ["type": "boolean", "value": value]
      case let .string(value): ["type": "string", "value": value]
      }
    }
  }

  private static func configurationValues(
    _ value: Any?, revision: UInt64?, source: String
  ) throws -> RefloatConfigurationValues? {
    guard let value, !(value is NSNull) else { return nil }
    guard let revision, let object = value as? [String: Any] else {
      throw RustApplicationRuntimeError.internalFailure
    }
    var values: [String: RefloatConfigurationValue] = [:]
    for (name, raw) in object {
      values[name] = try configurationValue(raw)
    }
    return RefloatConfigurationValues(revision: revision, source: source, values: values)
  }

  private static func configurationSchema(_ value: Any?) throws -> RefloatConfigurationSchema? {
    guard let value, !(value is NSNull) else { return nil }
    guard let object = value as? [String: Any], let fingerprint = object["fingerprint"] as? String,
          let signature = uint64(object["wire_signature"]), signature <= UInt32.max,
          let rawFields = object["fields"] as? [[String: Any]],
          let rawGroups = object["groups"] as? [[String: Any]]
    else { throw RustApplicationRuntimeError.internalFailure }
    let fields = try rawFields.map { field -> RefloatConfigurationField in
      guard let name = field["name"] as? String, let longName = field["long_name"] as? String,
            let rawType = field["field_type"] as? String,
            let type = configurationFieldType(rawType), let suffix = field["suffix"] as? String,
            let choices = field["choices"] as? [String], let description = field["description"] as? String
      else { throw RustApplicationRuntimeError.internalFailure }
      let wireType: RefloatConfigurationWireType? = switch field["wire_type"] as? String {
      case "uint8": .uint8
      case "int8": .int8
      case "uint16": .uint16
      case "int16": .int16
      case "uint32": .uint32
      case "int32": .int32
      case "decimal16": .double16
      case "decimal32": .double32
      case "decimal32auto": .double32Auto
      case nil: nil
      default: throw RustApplicationRuntimeError.internalFailure
      }
      return try RefloatConfigurationField(
        name: name, longName: longName, type: type, wireType: wireType,
        defaultValue: configurationValue(field["default_value"]),
        minimum: double(field["minimum"]), maximum: double(field["maximum"]),
        step: double(field["step"]), precision: int64(field["precision"]).map(Int.init),
        scale: double(field["scale"]), suffix: suffix, choices: choices, description: description
      )
    }
    let groups = try rawGroups.map { group -> RefloatConfigurationGroup in
      guard let name = group["name"] as? String,
            let rawSubgroups = group["subgroups"] as? [[String: Any]]
      else { throw RustApplicationRuntimeError.internalFailure }
      return try RefloatConfigurationGroup(name: name, subgroups: rawSubgroups.map { subgroup in
        guard let name = subgroup["name"] as? String, let fields = subgroup["fields"] as? [String]
        else { throw RustApplicationRuntimeError.internalFailure }
        return RefloatConfigurationSubgroup(name: name, fields: fields)
      })
    }
    return RefloatConfigurationSchema(
      fingerprint: fingerprint, wireSignature: UInt32(signature), fields: fields, groups: groups
    )
  }

  private static func configurationFieldType(_ value: String) -> RefloatConfigurationFieldType? {
    switch value {
    case "decimal": .double
    case "integer": .integer
    case "string": .string
    case "enumeration": .enumeration
    case "boolean": .boolean
    case "bitfield": .bitfield
    default: nil
    }
  }

  private static func cacheIdentity(_ value: Any?) -> RefloatSchemaCacheIdentity? {
    guard let value = value as? [String: Any], let firmware = value["firmware_version"] as? String,
          let hardware = value["hardware_name"] as? String, let package = value["package_version"] as? String,
          let index = uint8(value["custom_index"]), let crc = uint64(value["hardware_configuration_crc"]),
          crc <= UInt32.max
    else { return nil }
    return RefloatSchemaCacheIdentity(firmwareVersion: firmware, hardwareName: hardware,
                                      packageVersion: package, customIndex: index,
                                      hardwareConfigurationCRC: UInt32(crc))
  }

  private static func permissionAllowsUse(_ state: BluetoothPermissionState) -> Bool {
    state == .granted || state == .notRequired
  }

  private static func permissionIsTerminal(_ state: BluetoothPermissionState) -> Bool {
    state == .denied || state == .restricted
  }
}

public enum ApplicationError: Error, Equatable {
  case activeDeviceConflict
  case bluetoothUnavailable
  case permissionRequired
  case invalidScan
  case unknownDevice
  case deviceNotReady
  case productStoreUnavailable
  case invalidControllerSelection
}

private final class RefloatMutationWaiter: @unchecked Sendable {
  private let lock = NSLock()
  private let semaphore = DispatchSemaphore(value: 0)
  private var result: Result<RefloatControls, RefloatMutationError>?

  func finish(_ result: Result<RefloatControls, RefloatMutationError>) {
    let shouldSignal = lock.withLock {
      guard self.result == nil else { return false }
      self.result = result
      return true
    }
    if shouldSignal { semaphore.signal() }
  }

  func wait(timeout: TimeInterval) throws -> RefloatControls {
    guard semaphore.wait(timeout: .now() + timeout) == .success,
          let result = lock.withLock({ result })
    else { throw RefloatMutationError.unavailable }
    return try result.get()
  }
}

private final class RefloatConfigurationRequestWaiter: @unchecked Sendable {
  private let lock = NSLock()
  private let semaphore = DispatchSemaphore(value: 0)
  private var result: Result<Void, any Error>?

  func finish(_ result: Result<Void, any Error>) {
    let shouldSignal = lock.withLock {
      guard self.result == nil else { return false }
      self.result = result
      return true
    }
    if shouldSignal { semaphore.signal() }
  }

  func wait(timeout: TimeInterval) throws {
    guard semaphore.wait(timeout: .now() + timeout) == .success,
          let result = lock.withLock({ result })
    else { throw RefloatConfigurationServiceError.unavailable }
    try result.get()
  }
}
