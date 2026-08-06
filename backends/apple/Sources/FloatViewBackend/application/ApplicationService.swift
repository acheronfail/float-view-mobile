import CryptoKit
import Foundation

public final class ApplicationService: @unchecked Sendable {
  private let bluetooth: BluetoothPort
  private let clock: ClockPort
  private let logStore: LogStorePort
  private let productStore: (any ProductStorePort)?
  private let sessionStore: (any SessionStorePort)?
  private let protocolCodec: VescProtocolPort?
  private let scheduler: (any SchedulerPort)?
  private let locationProvider: (any LocationProviderPort)?
  private let events: EventSink
  private var reconnectDurationMs: Int64
  private var idleProbeIntervalMs: Int64
  private let lock = NSRecursiveLock()
  private var sampling: AdaptiveSamplingPolicy
  private var explicitDisconnect = false
  private var scanSequence = 0
  private var knownDevicesByID: [DeviceID: KnownDevice] = [:]
  private var discoveryPublishTimer: (any CancellationPort)?
  private var discoveryUpdatePending = false
  private var discoveryExpiryTimer: (any CancellationPort)?
  private var discoveryExpiryGeneration = 0
  private let controllerDiscovery = ControllerDiscovery()
  private let auxiliaryDiscovery = AuxiliaryNodeDiscovery()
  private var trackingMode: TrackingMode = .saved
  private var currentSavedDevice: SavedDeviceRecord?
  private var identificationTimer: (any CancellationPort)?
  private var canProbeTimer: (any CancellationPort)?
  private var bmsPollTimer: (any CancellationPort)?
  private var bmsFailureCount = 0
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
  private var vescGeneration = VescRequestGeneration(rawValue: 0)
  private lazy var vescRequests =
    VescRequestScheduler(
      clock: clock,
      scheduler: scheduler ?? SystemScheduler(),
      write: { [weak self] bytes in
        guard let self, let deviceID = self.activeDeviceID else { return }
        self.bluetooth.write(bytes, to: deviceID)
      },
      diagnostic: { [weak self] message in self?.events.publish(message) }
    )
  private var reconnectTimer: (any CancellationPort)?
  private var reconnectStartedAtMs: Int64?
  private var reconnectAttempt = 0
  private lazy var telemetryPolling = TelemetryPolling(
    clock: clock,
    scheduler: scheduler,
    protocolCodec: protocolCodec,
    submit: { [weak self] payload, expected, priority, context, timeout, completion in
      self?.submitVescRequest(
        payload: payload,
        expected: expected,
        priority: priority,
        context: context,
        timeoutMs: timeout,
        completion: completion
      )
    },
    onSample: { [weak self] sample in
      do {
        try self?.ingest(sample)
      } catch {
        self?.events.publish(String(describing: error))
      }
    },
    onFailure: { [weak self] reason in
      self?.lock.withLock {
        guard let self else { return }
        self.events.publish(reason)
        self.failIdentification(
          .identificationTimeout,
          "Refloat telemetry negotiation timed out."
        )
      }
    },
    onDiagnostic: { [weak self] message in self?.events.publish(message) }
  )

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
    protocolCodec: VescProtocolPort? = nil,
    scheduler: (any SchedulerPort)? = nil,
    events: EventSink,
    samplingSettings: SamplingSettings = .init(),
    locationProvider: (any LocationProviderPort)? = nil
  ) {
    self.bluetooth = bluetooth
    self.clock = clock
    self.logStore = logStore
    self.productStore = productStore
    self.sessionStore = sessionStore
    self.protocolCodec = protocolCodec
    self.scheduler = scheduler
    self.locationProvider = locationProvider
    self.events = events
    reconnectDurationMs = samplingSettings.reconnectDurationMs
    idleProbeIntervalMs = samplingSettings.idleProbeIntervalMs
    sampling = AdaptiveSamplingPolicy(settings: samplingSettings)
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
    if connectionState == .ready, currentSavedDevice?.savedDeviceID == savedDeviceID {
      bmsPollTimer?.cancel()
      bmsPollTimer = nil
      startAuxiliaryDiscovery()
    }
    return profile
  }

  public func selectController(deviceID: DeviceID, canID: UInt8) throws {
    lock.lock()
    defer { lock.unlock() }
    guard activeDeviceID == deviceID,
          connectionState == .selectingController,
          controllerDiscovery.canSelect(canID: canID)
    else {
      throw ApplicationError.invalidControllerSelection
    }
    guard case let .requestRefloatInfo(route, selectedFirmware?) =
      controllerDiscovery.selectController(canID: canID)
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
        controllerUUID: ControllerDiscoveryPolicy.meaningfulUUID(selectedFirmware.controllerUUID),
        at: clock.now()
      )
      events.publish(profile)
    }
    controllerDiscovery.clearIssue()
    transition(.identifying)
    performDiscoveryEffect(.requestRefloatInfo(route: route, rememberController: nil))
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
      sampling = AdaptiveSamplingPolicy(settings: SamplingSettings(
        idleProbeIntervalMs: value.idleProbeIntervalMs,
        idlePersistIntervalMs: value.idlePersistIntervalMs,
        ridingPersistIntervalMs: value.ridingPersistIntervalMs,
        postRideTailMs: value.postRideTailMs,
        reconnectDurationMs: value.reconnectDurationMs
      ))
      telemetryPolling.updateSettings(
        idleIntervalMs: value.idleProbeIntervalMs,
        postRideTailMs: value.postRideTailMs
      )
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
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionHistoryConfiguration(sessionID: sessionID)
  }

  public func querySessionSeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int,
    publishedSampleSequence: Int64? = nil
  ) throws -> [SessionSeriesPoint] {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.querySeries(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs,
      fields: fields, maxPoints: maxPoints,
      publishedSampleSequence: publishedSampleSequence
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
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.querySeriesSource(
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
    expectedPersistenceMs: Int64,
    expectedRidingPersistenceMs: Int64?,
    expectedIdlePersistenceMs: Int64?,
    retainedResolutionSeconds: Int64,
    publishedSampleSequence: Int64? = nil
  ) throws -> SessionSeriesReductionRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.reduceSeries(
      sessionID: sessionID,
      startElapsedMs: startElapsedMs,
      endElapsedMs: endElapsedMs,
      fields: fields,
      buckets: buckets,
      gapPolicy: gapPolicy,
      expectedPersistenceMs: expectedPersistenceMs,
      expectedRidingPersistenceMs: expectedRidingPersistenceMs,
      expectedIdlePersistenceMs: expectedIdlePersistenceMs,
      retainedResolutionSeconds: retainedResolutionSeconds,
      publishedSampleSequence: publishedSampleSequence
    )
  }

  public func cancelHistoricalReads(sessionID: String) {
    sessionStore?.cancelHistoricalReads(sessionID: sessionID)
  }

  public func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    publishedEventSequence: Int64? = nil
  ) throws -> [SessionEventRecord] {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionEvents(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs,
      publishedEventSequence: publishedEventSequence
    )
  }

  public func querySessionLocations(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    limit: Int,
    cursor: String?,
    publishedLocationSequence: Int64?
  ) throws -> SessionLocationPageRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.queryLocations(
      sessionID: sessionID,
      startElapsedMs: startElapsedMs,
      endElapsedMs: endElapsedMs,
      limit: limit,
      cursor: cursor,
      publishedLocationSequence: publishedLocationSequence
    )
  }

  public func sessionExtensionFields(sessionID: String) throws -> [String] {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionExtensionFields(sessionID: sessionID)
  }

  public func sessionPruneMetadata(sessionID: String) throws -> SessionPruneMetadataRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionPruneMetadata(sessionID: sessionID)
  }

  public func prunePreview(sessionID: String, targetResolutionSeconds: Int64) throws -> SessionPrunePreviewRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.prunePreview(sessionID: sessionID, targetResolutionSeconds: targetResolutionSeconds)
  }

  public func pruneSession(sessionID: String, targetResolutionSeconds: Int64) throws -> SessionPrunePreviewRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.pruneSession(sessionID: sessionID, targetResolutionSeconds: targetResolutionSeconds)
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
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.querySampleWindow(
      sessionID: sessionID, centerElapsedMs: centerElapsedMs,
      beforeMs: beforeMs, afterMs: afterMs, fields: fields,
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
    telemetryPolling.resetSession()
    latestPersistedSample = nil
    latestLocation = nil
    resetFirmwareDiscovery()
    protocolCodec?.reset()
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
      if state == .ready {
        if protocolCodec == nil {
          completeWithoutProtocol()
          return
        }
        transition(.identifying)
        startFirmwareDiscovery(deviceID)
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
        controllerDiscovery.markFailure(
          .transportSetupFailed,
          "The VESC Bluetooth transport could not be prepared."
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
        resetFirmwareDiscovery()
      }
      transition(state)
    case let .notification(deviceID, bytes):
      guard deviceID == activeDeviceID else { return }
      guard let protocolCodec else {
        events.publish(event)
        return
      }
      do {
        for message in try protocolCodec.receive(bytes) {
          let requestContext = vescRequests.inFlightRequest?.context
          let identity: (UInt8, UInt8?) = switch message {
          case .firmware: (VescCommand.getFirmwareVersion.rawValue, nil)
          case .currentLimits: (VescCommand.getMotorConfiguration.rawValue, nil)
          case .canDevices: (VescCommand.pingCAN.rawValue, nil)
          case .bmsValuesPayload: (VescCommand.bmsGetValues.rawValue, nil)
          case .refloatInfo: (VescCommand.customAppData.rawValue, RefloatCommand.info.rawValue)
          case .refloatTelemetryLayout:
            (VescCommand.customAppData.rawValue, RefloatCommand.realtimeDataInternalIDs.rawValue)
          case let .refloatTelemetry(value):
            (VescCommand.customAppData.rawValue,
             value.kind == .realtime
               ? RefloatCommand.realtimeDataInternal.rawValue : RefloatCommand.getAllData.rawValue)
          }
          if !vescRequests.receive(
            payload: bytes,
            command: identity.0,
            refloatCommand: identity.1
          )
          { continue }
          switch message {
          case let .firmware(value):
            let attributed = attributedFirmware(value, context: requestContext)
            if case .some(.auxiliaryFirmware) = requestContext {
              events.publish(
                "bms_discovery_identity:\(Self.bmsRouteDiagnostic(attributed.canID)):"
                  + "type=\(attributed.hardwareType):version=\(attributed.major).\(attributed.minor):"
                  + "hardware=\(Self.bmsHardwareDiagnostic(attributed.hardwareName))"
              )
              performAuxiliaryEffect(auxiliaryDiscovery.acceptFirmware(
                attributed, generation: auxiliaryDiscovery.generation
              ))
            } else {
              handleFirmware(attributed)
            }
          case let .currentLimits(value):
            if connectionState == .ready { currentLimits = value }
          case let .canDevices(canIDs):
            if requestContext == .auxiliaryCanPing {
              events.publish("bms_discovery_scan_result:count=\(canIDs.count)")
              performAuxiliaryEffect(auxiliaryDiscovery.acceptCANIDs(
                canIDs, generation: auxiliaryDiscovery.generation
              ))
            } else {
              handleCanDevices(canIDs)
            }
          case let .bmsValuesPayload(payload):
            guard case let .bmsValues(route) = requestContext else { continue }
            let expectedCanID: UInt8? = switch route {
            case .direct: nil
            case let .can(canID): canID
            }
            do {
              let snapshot = try VescPayloadDecoder.bmsValues(
                Array(payload), expectedCanID: expectedCanID, observedAt: clock.now()
              )
              bmsFailureCount = 0
              auxiliaryDiscovery.acceptSnapshot(snapshot)
              events.publish("bms_values_attached:\(Self.bmsRouteDiagnostic(expectedCanID))")
              events.publish(snapshot)
            } catch {
              events.publish("bms_values_decode_failed:\(Self.bmsRouteDiagnostic(expectedCanID))")
              recordBmsFailure()
            }
          case let .refloatInfo(version):
            guard connectionState == .identifying else { continue }
            performDiscoveryEffect(controllerDiscovery.acceptRefloatInfo(version))
          case let .refloatTelemetryLayout(fields, runtimeFields):
            telemetryPolling.layoutEstablished()
            performDiscoveryEffect(
              controllerDiscovery.acceptTelemetryLayout(
                fields: fields,
                runtimeFields: runtimeFields
              )
            )
          case let .refloatTelemetry(value):
            telemetryPolling.receive(value)
          }
          events.publish(message)
        }
      } catch {
        events.publish(String(describing: error))
      }
    }
  }

  private func attributedFirmware(
    _ value: FirmwareRecord,
    context: VescRequestContext?
  ) -> FirmwareRecord {
    let attributedCanID: UInt8? = switch context {
    case let .controllerFirmware(.can(canID)), let .auxiliaryFirmware(.can(canID)): canID
    default: nil
    }
    return FirmwareRecord(
      major: value.major,
      minor: value.minor,
      testVersion: value.testVersion,
      hardwareName: value.hardwareName,
      firmwareName: value.firmwareName,
      hardwareType: value.hardwareType,
      canID: attributedCanID ?? value.canID,
      controllerUUID: value.controllerUUID
    )
  }

  public func disconnect() throws {
    lock.lock()
    defer { lock.unlock() }
    guard let deviceID = activeDeviceID else { return }
    transition(.disconnecting)
    explicitDisconnect = true
    cancelReconnect()
    telemetryPolling.cancelLayout()
    telemetryPolling.stop()
    stopLocationCapture()
    if trackingMode == .saved, let active = logStore.active() {
      try events.publish(
        logStore.close(jobID: active.jobID, at: clock.now(), reason: "explicit_disconnect")
      )
    }
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
    if sampling.shouldPersist(rideState: sample.rideState, elapsedMs: sample.elapsedMs) {
      try logStore.append(jobID: active.jobID, samples: [sample])
      latestPersistedSample = sample
      events.publish(sample)
    }
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
      try logStore.appendLocations(
        jobID: jobID,
        observations: [LocationObservation(
          observedAt: observation.observedAt,
          elapsedMs: elapsedMs,
          latitude: observation.latitude,
          longitude: observation.longitude,
          accuracyM: observation.accuracyM
        )]
      )
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
      trackingMode: trackingMode,
      savedDevice: trackingMode == .saved ? currentSavedDevice : nil,
      activeSession: activeSession ?? nil,
      latestSample: latestPersistedSample,
      latestLocation: latestLocation,

      currentLimits: currentLimits,
      controllerRoute: controllerDiscovery.controllerRoute,
      controllerCandidates: controllerDiscovery.controllerCandidates,
      connectionIssue: controllerDiscovery.connectionIssue,
      bms: auxiliaryDiscovery.state
    )
  }

  public func firmwareSnapshot() -> FirmwareRecord? {
    lock.lock()
    defer { lock.unlock() }
    return controllerDiscovery.firmware
  }

  public func refloatPackageVersionSnapshot() -> String? {
    lock.lock()
    defer { lock.unlock() }
    return controllerDiscovery.refloatPackageVersion
  }

  private func startFirmwareDiscovery(_ deviceID: DeviceID) {
    guard protocolCodec != nil, activeDeviceID == deviceID else { return }
    vescGeneration = vescRequests.beginGeneration()
    let (generation, effect) = controllerDiscovery.begin()
    guard effect != .none else { return }
    identificationTimer?.cancel()
    identificationTimer = scheduler?.schedule(
      afterMilliseconds: Self.identificationTimeoutMs
    ) { [weak self] in
      self?.identificationTimedOut(generation: generation)
    }
    performDiscoveryEffect(effect)
  }

  private func submitVescRequest(
    payload: Data,
    expected: VescExpectedResponse,
    priority: VescRequestPriority,
    context: VescRequestContext,
    timeoutMs: Int64,
    completion: @escaping @Sendable (VescRequestCompletion) -> Void = { _ in }
  ) {
    vescRequests.enqueue(
      payload: payload,
      expectedResponse: expected,
      priority: priority,
      context: context,
      timeoutMs: timeoutMs,
      generation: vescGeneration
    ) { _, outcome in completion(outcome) }
  }

  private func handleFirmware(_ value: FirmwareRecord) {
    if value.canID == nil {
      if trackingMode == .saved {
        do {
          let stableIdentity = Self.stableBoardIdentity(value.controllerUUID)
          currentSavedDevice = try productStore?.ensureSavedDevice(
            deviceID: activeDeviceID ?? DeviceID(""),
            advertisedName: activeDeviceName,
            stableIdentity: stableIdentity,
            at: clock.now()
          )
          if let currentSavedDevice {
            let saved = (try? productStore?.savedDevices()) ?? [currentSavedDevice]
            bluetooth.configureSavedDeviceIDs(Set(
              saved.compactMap { $0.bluetoothIdentifier.map(DeviceID.init) }
            ))
          }
        } catch {
          failIdentification(.transportSetupFailed, "Unable to save the VESC device.")
          return
        }
      }
      let profile = try? currentSavedDevice.flatMap {
        try productStore?.boardProfile(savedDeviceID: $0.savedDeviceID)
      }
      let hint = try? currentSavedDevice.flatMap {
        try productStore?.controllerRoutingHint(savedDeviceID: $0.savedDeviceID)
      }
      performDiscoveryEffect(controllerDiscovery.acceptTransport(
        value,
        preferredCanID: profile.flatMap {
          ControllerDiscoveryPolicy.preferredCanID($0.valuesJSON)
        },
        routingHint: hint ?? nil
      ))
      return
    }
    guard controllerDiscovery.activeCanProbeID == value.canID else { return }
    cancelCanProbeTimeout()
    performDiscoveryEffect(
      controllerDiscovery.acceptCANFirmware(value, continuity: controllerContinuity())
    )
  }

  private func handleCanDevices(_ rawCanIDs: [UInt8]) {
    performDiscoveryEffect(
      controllerDiscovery.acceptCANIDs(rawCanIDs, continuity: controllerContinuity())
    )
  }

  private func controllerContinuity() -> ControllerContinuity {
    ControllerContinuity(
      hasActiveSession: trackingMode == .saved && logStore.active() != nil,
      route: sessionControllerRoute,
      controllerUUID: sessionControllerUUID
    )
  }

  private func performDiscoveryEffect(_ effect: ControllerDiscoveryEffect) {
    switch effect {
    case let .probeFirmware(route):
      guard activeDeviceID != nil, let protocolCodec else { return }
      if case let .can(canID) = route { scheduleCanProbeTimeout(canID: canID) }
      do {
        try submitVescRequest(
          payload: protocolCodec.firmwareRequest(route: route),
          expected: .init(command: VescCommand.getFirmwareVersion.rawValue),
          priority: .controllerIdentification,
          context: .controllerFirmware(.init(route)),
          timeoutMs: route == .direct ? Self.identificationTimeoutMs : Self.canProbeTimeoutMs
        )
      } catch {
        if route == .direct {
          failIdentification(.transportSetupFailed, "Unable to request VESC firmware.")
          events.publish(String(describing: error))
        } else {
          failIdentification(.transportSetupFailed, "Unable to probe the CAN controller.")
        }
      }
    case .pingCAN:
      cancelCanProbeTimeout()
      guard activeDeviceID != nil, let protocolCodec else { return }
      do {
        try submitVescRequest(
          payload: protocolCodec.canPingRequest(),
          expected: .init(command: VescCommand.pingCAN.rawValue),
          priority: .controllerIdentification,
          context: .canPing,
          timeoutMs: Self.identificationTimeoutMs
        )
      } catch {
        failIdentification(.transportSetupFailed, "Unable to discover CAN controllers.")
      }
    case let .clearHintThen(next):
      invalidateAutomaticHint()
      restartIdentificationTimeout()
      performDiscoveryEffect(next)
    case let .requestRefloatInfo(route, selected):
      if trackingMode == .saved,
         let selected,
         let savedDevice = currentSavedDevice,
         let canID = selected.canID,
         let uuid = ControllerDiscoveryPolicy.meaningfulUUID(selected.controllerUUID)
      {
        try? productStore?.storeControllerRoutingHint(
          ControllerRoutingHint(
            savedDeviceID: savedDevice.savedDeviceID,
            canID: canID,
            controllerUUID: uuid
          ),
          at: clock.now()
        )
      }
      guard activeDeviceID != nil, let protocolCodec else { return }
      do {
        try submitVescRequest(
          payload: protocolCodec.refloatInfoRequest(route: route),
          expected: .init(
            command: VescCommand.customAppData.rawValue,
            refloatCommand: RefloatCommand.info.rawValue
          ),
          priority: .controllerIdentification,
          context: .refloatInfo(.init(route)),
          timeoutMs: Self.identificationTimeoutMs
        )
      } catch {
        events.publish(String(describing: error))
      }
    case .requestTelemetryLayout:
      guard let activeDeviceID, let route = controllerDiscovery.controllerRoute else { return }
      telemetryPolling.beginLayout(deviceID: activeDeviceID, route: route)
    case .ready:
      completeApplicationReady()
    case .selectingController:
      cancelIdentificationTimeout()
      transition(.selectingController)
    case let .failed(issue):
      failIdentification(issue.code, issue.message)
    case .none:
      break
    }
  }

  private func completeApplicationReady() {
    guard let deviceID = activeDeviceID,
          let route = controllerDiscovery.controllerRoute,
          let firmware = controllerDiscovery.firmware
    else { return }
    cancelIdentificationTimeout()
    if trackingMode == .saved, logStore.active() != nil {
      guard controllerContinuityIsProven(route: route, firmware: firmware) else {
        failIdentification(
          .controllerChanged,
          "The connected controller no longer matches the active session."
        )
        return
      }
    } else if trackingMode == .saved {
      do {
        try events.publish(logStore.create(
          deviceID: deviceID,
          advertisedName: activeDeviceName,
          at: clock.now()
        ))
        telemetryPolling.markSessionStarted()
        sessionControllerRoute = route
        sessionControllerUUID = ControllerDiscoveryPolicy.meaningfulUUID(firmware.controllerUUID)
        sessionFirmware = firmware
        sessionRefloatPackageVersion = controllerDiscovery.refloatPackageVersion
      } catch {
        failIdentification(.transportSetupFailed, "Unable to start durable telemetry.")
        return
      }
    }
    cancelReconnect()
    controllerDiscovery.clearIssue()
    clearDiscoveredDevices(publish: false)
    transition(.ready)
    if let protocolCodec {
      do {
        try submitVescRequest(
          payload: protocolCodec.motorConfigurationRequest(route: route),
          expected: .init(command: VescCommand.getMotorConfiguration.rawValue),
          priority: .controllerWork,
          context: .currentLimits(.init(route)),
          timeoutMs: Self.canProbeTimeoutMs
        )
      } catch {
        events.publish(String(describing: error))
      }
    }
    telemetryPolling.start(
      deviceID: deviceID,
      route: route,
      family: controllerDiscovery.refloatFamily ?? .dynamic,
      idleIntervalMs: idleProbeIntervalMs,
      postRideTailMs: sampling.settings.postRideTailMs
    )
    startAuxiliaryDiscovery()
  }

  private func startAuxiliaryDiscovery() {
    guard protocolCodec?.supportsBmsTelemetry == true,
          let localIdentity = controllerDiscovery.transportFirmware else { return }
    let profile = try? currentSavedDevice.flatMap {
      try productStore?.boardProfile(savedDeviceID: $0.savedDeviceID)
    }
    let preferred = profile.flatMap {
      ControllerDiscoveryPolicy.preferredBmsCanID($0.valuesJSON)
    }
    performAuxiliaryEffect(auxiliaryDiscovery.begin(
      localIdentity: localIdentity,
      reusableInventory: controllerDiscovery.inventoryComplete
        ? controllerDiscovery.classifiedInventory : nil,
      preferredCanID: preferred
    ))
  }

  private func performAuxiliaryEffect(_ effect: AuxiliaryDiscoveryEffect) {
    guard connectionState == .ready, let protocolCodec else { return }
    switch effect {
    case .pingCAN:
      events.publish("bms_discovery_scan_started")
      do {
        try submitVescRequest(
          payload: protocolCodec.canPingRequest(),
          expected: .init(command: VescCommand.pingCAN.rawValue),
          priority: .auxiliary, context: .auxiliaryCanPing,
          timeoutMs: Self.canScanTimeoutMs,
          completion: { [weak self] completion in
            guard case .timedOut = completion else { return }
            self?.lock.withLock { self?.auxiliaryDiscovery.markUnavailable() }
          }
        )
      } catch { auxiliaryDiscovery.markUnavailable() }
    case let .probeFirmware(route):
      events.publish("bms_discovery_probe:\(Self.bmsRouteDiagnostic(route.canID))")
      let generation = auxiliaryDiscovery.generation
      let canID: UInt8? = if case let .can(value) = route { value } else { nil }
      do {
        try submitVescRequest(
          payload: protocolCodec.firmwareRequest(route: route),
          expected: .init(command: VescCommand.getFirmwareVersion.rawValue),
          priority: .auxiliary, context: .auxiliaryFirmware(.init(route)),
          timeoutMs: Self.canProbeTimeoutMs,
          completion: { [weak self] completion in
            guard case .timedOut = completion, let canID else { return }
            self?.lock.withLock {
              guard let self else { return }
              self.performAuxiliaryEffect(self.auxiliaryDiscovery.probeTimedOut(
                canID: canID, generation: generation
              ))
            }
          }
        )
      } catch { auxiliaryDiscovery.markUnavailable() }
    case let .requestValues(connection):
      let canID: UInt8? = if case let .can(value) = connection { value } else { nil }
      events.publish("bms_values_requested:\(Self.bmsRouteDiagnostic(canID))")
      requestBmsValues(connection)
    case .none:
      events.publish("bms_discovery_complete:\(auxiliaryDiscovery.state.status.rawValue)")
      events.publish(auxiliaryDiscovery.state)
    }
  }

  private func requestBmsValues(_ connection: BmsConnection) {
    guard connectionState == .ready, let protocolCodec else { return }
    bmsPollTimer?.cancel()
    bmsPollTimer = nil
    let route: VescRequestRoute = switch connection {
    case .direct: .direct
    case let .can(canID): .can(canID)
    }
    do {
      try submitVescRequest(
        payload: protocolCodec.bmsValuesRequest(connection: connection),
        expected: .init(command: VescCommand.bmsGetValues.rawValue),
        priority: .auxiliary, context: .bmsValues(route), timeoutMs: 1_000,
        completion: { [weak self] completion in
          self?.lock.withLock {
            guard let self, self.connectionState == .ready else { return }
            switch completion {
            case .timedOut, .queueFull: self.recordBmsFailure()
            case .response: break
            case .cancelled, .staleGeneration: return
            }
            self.bmsPollTimer = self.scheduler?.schedule(afterMilliseconds: 1_000) { [weak self] in
              self?.lock.withLock {
                guard let self, let selected = self.auxiliaryDiscovery.selectedConnection else { return }
                self.requestBmsValues(selected)
              }
            }
          }
        }
      )
    } catch { recordBmsFailure() }
  }

  private func recordBmsFailure() {
    bmsFailureCount += 1
    if bmsFailureCount >= 3 {
      auxiliaryDiscovery.markUnavailable()
      events.publish(auxiliaryDiscovery.state)
    }
  }

  /// Production hosts always provide a VESC codec and never take this path.
  private func completeWithoutProtocol() {
    guard let deviceID = activeDeviceID else { return }
    if trackingMode == .saved, logStore.active() == nil {
      do {
        try events.publish(logStore.create(
          deviceID: deviceID,
          advertisedName: activeDeviceName,
          at: clock.now()
        ))
        telemetryPolling.markSessionStarted()
      } catch {
        transition(.failed)
        explicitDisconnect = true
        bluetooth.disconnect(deviceID)
        events.publish(String(describing: error))
        return
      }
    }
    cancelReconnect()
    clearDiscoveredDevices(publish: false)
    transition(.ready)
  }

  private func controllerContinuityIsProven(
    route: ControllerRoute,
    firmware: FirmwareRecord
  ) -> Bool {
    guard let previousRoute = sessionControllerRoute else { return true }
    let currentUUID = ControllerDiscoveryPolicy.meaningfulUUID(firmware.controllerUUID)
    if let sessionControllerUUID {
      return sessionControllerUUID == currentUUID
    }
    return previousRoute == route
      && sessionFirmware.map {
        ControllerDiscoveryPolicy.samePublicIdentity($0, firmware)
      } == true
      && sessionRefloatPackageVersion == controllerDiscovery.refloatPackageVersion
  }

  private func identificationTimedOut(generation: Int) {
    lock.withLock {
      guard connectionState == .identifying else { return }
      performDiscoveryEffect(
        controllerDiscovery.identificationTimedOut(generation: generation)
      )
    }
  }

  private func failIdentification(_ code: ConnectionIssueCode, _ message: String) {
    cancelIdentificationTimeout()
    cancelCanProbeTimeout()
    telemetryPolling.cancelLayout()
    telemetryPolling.stop()
    controllerDiscovery.markFailure(code, message)
    if trackingMode == .saved, code == .controllerChanged, let active = logStore.active() {
      _ = try? logStore.close(jobID: active.jobID, at: clock.now(), reason: code.rawValue)
    }
    transition(.failed)
  }

  private func cancelIdentificationTimeout() {
    identificationTimer?.cancel()
    identificationTimer = nil
  }

  private func restartIdentificationTimeout() {
    identificationTimer?.cancel()
    let generation = controllerDiscovery.identificationGeneration
    identificationTimer = scheduler?.schedule(
      afterMilliseconds: Self.identificationTimeoutMs
    ) { [weak self] in
      self?.identificationTimedOut(generation: generation)
    }
  }

  private func invalidateAutomaticHint() {
    if let savedDeviceID = currentSavedDevice?.savedDeviceID {
      try? productStore?.clearControllerRoutingHint(savedDeviceID: savedDeviceID)
    }
  }

  private func scheduleCanProbeTimeout(canID: UInt8) {
    cancelCanProbeTimeout()
    let generation = controllerDiscovery.canProbeGeneration
    canProbeTimer = scheduler?.schedule(
      afterMilliseconds: Self.canProbeTimeoutMs
    ) { [weak self] in
      self?.canProbeTimedOut(canID: canID, generation: generation)
    }
  }

  private func canProbeTimedOut(canID: UInt8, generation: Int) {
    lock.withLock {
      guard connectionState == .identifying else { return }
      let effect = controllerDiscovery.canProbeTimedOut(
        canID: canID,
        generation: generation
      )
      guard effect != .none else { return }
      cancelCanProbeTimeout()
      performDiscoveryEffect(effect)
    }
  }

  private func cancelCanProbeTimeout() {
    canProbeTimer?.cancel()
    canProbeTimer = nil
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
    resetFirmwareDiscovery()
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
    resetFirmwareDiscovery()
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
    guard let value = ControllerDiscoveryPolicy.meaningfulUUID(value) else { return nil }
    var input = Data("float-view-board-v1\0".utf8)
    input.append(value)
    let digest = SHA256.hash(data: input)
    return "vesc-v1-" + digest.map { String(format: "%02x", $0) }.joined()
  }

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

  private func resetFirmwareDiscovery() {
    vescGeneration = vescRequests.beginGeneration()
    telemetryPolling.cancelLayout()
    telemetryPolling.stop()
    controllerDiscovery.reset()
    auxiliaryDiscovery.reset()
    bmsPollTimer?.cancel()
    bmsPollTimer = nil
    bmsFailureCount = 0
    currentLimits = nil
    currentSavedDevice = nil
    cancelIdentificationTimeout()
    cancelCanProbeTimeout()
    protocolCodec?.reset()
  }

  private static let discoveryPublishIntervalMs: Int64 = 250
  private static let discoveryExpiryMs: Int64 = 120_000
  private static let maxReconnectBackoffMs: Int64 = 30_000
  private static let identificationTimeoutMs: Int64 = 10_000
  // COMM_PING_CAN can synchronously probe all 255 CAN IDs at 10 ms per ID.
  private static let canScanTimeoutMs: Int64 = 10_000
  private static let canProbeTimeoutMs: Int64 = 1_000

  private static func bmsRouteDiagnostic(_ canID: UInt8?) -> String {
    canID.map { "can=\($0)" } ?? "direct"
  }

  private static func bmsHardwareDiagnostic(_ value: String?) -> String {
    guard let value else { return "unknown" }
    let scalarValues = value.unicodeScalars.prefix(32).map { scalar -> Character in
      let accepted = scalar.isASCII
        && (CharacterSet.alphanumerics.contains(scalar) || scalar == "-" || scalar == "_")
      return accepted ? Character(String(scalar)) : "_"
    }
    return scalarValues.isEmpty ? "unknown" : String(scalarValues)
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
