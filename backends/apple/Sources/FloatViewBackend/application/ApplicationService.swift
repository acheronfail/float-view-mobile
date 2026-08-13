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
  private var deviceKind: DeviceKind?
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
  private let refloatMutations = DeviceMutationCoordinator()
  private let refloatConfiguration: RefloatConfigurationService
  private var refloatProfile: RefloatControlProfile?
  private var latestRefloatTelemetry: RefloatTelemetryRecord?
  private var latestRefloatTelemetryMonotonicMs: Int64?
  private var refloatRefreshTimer: (any CancellationPort)?
  private var refloatMutationTimer: (any CancellationPort)?
  private var shutdownIntentTimer: (any CancellationPort)?
  private var refloatWaiter: RefloatMutationWaiter?
  private var refloatConfigurationWaiter: RefloatConfigurationRequestWaiter?
  private lazy var refloatScheduler: any SchedulerPort = scheduler ?? SystemScheduler()
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
        if self.refloatMutations.controllerActions(shutdownAvailable: false).pendingAction == .shutdown {
          self.shutdownIntentTimer?.cancel()
          self.shutdownIntentTimer = nil
          _ = self.refloatMutations.clearShutdown(generation: self.vescGeneration)
          self.finishReconnect(reason: "controller_shutdown")
          return
        }
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
    refloatConfiguration = RefloatConfigurationService(cache: productStore)
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

  public func withSessionReadCancellation<T>(
    _ cancellation: SessionReadCancellation,
    body: () throws -> T
  ) rethrows -> T {
    guard let sessionStore else { return try body() }
    return try sessionStore.withSessionReadCancellation(cancellation, body: body)
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
      if state == .failed || state == .disconnected,
         refloatMutations.controllerActions(shutdownAvailable: false).pendingAction == .shutdown
      {
        shutdownIntentTimer?.cancel()
        shutdownIntentTimer = nil
        _ = refloatMutations.clearShutdown(generation: vescGeneration)
        finishReconnect(reason: "controller_shutdown")
        return
      }
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
          case let .refloatLighting(command, _):
            (VescCommand.customAppData.rawValue, command.rawValue)
          case let .customConfiguration(response):
            switch response {
            case .schemaChunk: (VescCommand.getCustomConfigurationXML.rawValue, nil)
            case let .values(command, _, _): (command.rawValue, nil)
            case .writeAcknowledgement: (VescCommand.setCustomConfiguration.rawValue, nil)
            }
          }
          let deferNext = switch message {
          case .refloatTelemetry, .customConfiguration: true
          default: false
          }
          var deferredResumed = false
          if !vescRequests.receive(
            payload: bytes,
            command: identity.0,
            refloatCommand: identity.1,
            validContext: { context in Self.matchesCustomConfiguration(message, context: context) },
            deferNext: deferNext
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
          case let .refloatInfo(decision):
            guard connectionState == .identifying else { continue }
            performDiscoveryEffect(controllerDiscovery.acceptRefloatInfo(decision))
          case let .refloatTelemetryLayout(fields, runtimeFields, supported):
            telemetryPolling.layoutEstablished()
            performDiscoveryEffect(
              controllerDiscovery.acceptTelemetryLayout(
                fields: fields,
                runtimeFields: runtimeFields,
                supported: supported
              )
            )
          case let .refloatTelemetry(value):
            latestRefloatTelemetry = value
            latestRefloatTelemetryMonotonicMs = clock.monotonicMilliseconds()
            let wasPending = refloatMutations.snapshot()?.pendingAction
            let updated = refloatMutations.observeTelemetry(value)
            if wasPending != nil, updated?.pendingAction == nil {
              refloatConfiguration.invalidateCurrent()
              finishRefloatMutation(.success(updated!))
            }
            if let mode = value.packageMode {
              if mode == .flywheel {
                cancelFlywheelProhibitedRequests(
                  preservingRefloatControl: updated?.pendingAction == .flywheel
                )
                if let pending = updated?.pendingAction, pending != .flywheel {
                  failRefloatMutation(generation: vescGeneration)
                }
                _ = refloatMutations.clearShutdown(generation: vescGeneration)
                refloatMutations.finishConfiguration(generation: vescGeneration)
              }
              performRefloatConfigurationEffect(
                refloatConfiguration.setFlywheel(mode == .flywheel)
              )
            }
            vescRequests.resume()
            deferredResumed = true
            telemetryPolling.receive(value)
          case let .refloatLighting(_, state):
            let wasPending = refloatMutations.snapshot()?.pendingAction
            let updated = refloatMutations.observeLighting(state)
            if wasPending != nil, updated?.pendingAction == nil {
              refloatConfiguration.invalidateCurrent()
              finishRefloatMutation(.success(updated!))
            }
          case let .customConfiguration(response):
            guard let requestContext else { continue }
            do {
              let effect = try refloatConfiguration.accept(
                response, context: requestContext, generation: vescGeneration
              )
              performRefloatConfigurationEffect(effect)
              if case .refloatConfigurationVerification = requestContext {
                refloatMutations.finishConfiguration(generation: vescGeneration)
                telemetryPolling.refresh()
                if let route = controllerDiscovery.controllerRoute, let profile = refloatProfile {
                  requestRefloatLighting(route: route, profile: profile)
                }
              }
              events.publish(refloatConfiguration.summary())
              switch requestContext {
              case .refloatConfigurationValues, .refloatConfigurationVerification:
                finishRefloatConfigurationRequest(.success(()))
              default:
                break
              }
            } catch {
              if case .refloatConfigurationWrite = requestContext {
                refloatMutations.finishConfiguration(generation: vescGeneration)
              }
              if case .refloatConfigurationVerification = requestContext {
                refloatMutations.finishConfiguration(generation: vescGeneration)
              }
              finishRefloatConfigurationRequest(.failure(error))
              events.publish(String(describing: error))
            }
          }
          if deferNext, !deferredResumed { vescRequests.resume() }
          events.publish(message)
        }
      } catch {
        events.publish(String(describing: error))
      }
    }
  }

  private static func matchesCustomConfiguration(
    _ message: VescProtocolMessage, context: VescRequestContext
  ) -> Bool {
    guard case let .customConfiguration(response) = message else { return true }
    return switch (response, context) {
    case let (.schemaChunk(index, _, offset, _), .refloatConfigurationSchema(_, expected, expectedOffset, _)):
      index == expected && offset == expectedOffset
    case let (.values(command, index, _), .refloatConfigurationValues(_, expected, defaults, _)):
      index == expected && command == (defaults ? .getCustomConfigurationDefault : .getCustomConfiguration)
    case let (.values(command, index, _), .refloatConfigurationVerification(_, expected, _)):
      index == expected && command == .getCustomConfiguration
    case (.writeAcknowledgement, .refloatConfigurationWrite): true
    default: false
    }
  }

  private func cancelFlywheelProhibitedRequests(preservingRefloatControl: Bool) {
    vescRequests.cancelMatching({ context in
      switch context {
      case .refloatControl:
        !preservingRefloatControl
      case .controllerShutdown, .refloatConfigurationWrite, .refloatConfigurationVerification:
        true
      case let .refloatConfigurationValues(_, _, defaults, _): !defaults
      default: false
      }
    }, resume: false)
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
      controllerUUID: value.controllerUUID,
      customConfigurationCount: value.customConfigurationCount,
      hardwareConfigurationCRC: value.hardwareConfigurationCRC
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
      deviceKind: deviceKind,
      trackingMode: trackingMode,
      savedDevice: trackingMode == .saved ? currentSavedDevice : nil,
      activeSession: activeSession ?? nil,
      latestSample: latestPersistedSample,
      latestLocation: latestLocation,

      currentLimits: currentLimits,
      controllerRoute: controllerDiscovery.controllerRoute,
      controllerCandidates: controllerDiscovery.controllerCandidates,
      connectionIssue: controllerDiscovery.connectionIssue,
      bms: auxiliaryDiscovery.state,
      refloatControls: refloatMutations.snapshot(),
      controllerActions: controllerActionsSnapshot()
    )
  }

  public func shutdownController() throws {
    try lock.withLock {
      guard let protocolCodec,
            let route = controllerDiscovery.controllerRoute,
            let firmware = controllerDiscovery.firmware,
            let telemetry = latestRefloatTelemetry
      else { throw RefloatMutationError.invalidState }
      let context = ControllerShutdownContext(
        generation: vescGeneration,
        connectionReady: connectionState == .ready,
        shutdownSupported: VescCompatibility.supportsShutdown(firmware),
        rideState: telemetry.rideState,
        packageMode: telemetry.packageMode
      )
      let actions = try refloatMutations.authorizeShutdown(context: context)
      do {
        let payload = try protocolCodec.controllerShutdownRequest(route: route)
        events.publish(actions)
        vescRequests.enqueueOneWay(
          payload: payload,
          expectedResponse: .init(command: VescCommand.shutdown.rawValue),
          priority: .ridingTelemetry,
          context: .controllerShutdown(.init(route)),
          timeoutMs: Self.shutdownIntentTimeoutMs,
          generation: vescGeneration
        ) { [weak self] _, outcome in
          guard outcome != .sent else { return }
          self?.lock.withLock { self?.expireShutdownIntent(generation: context.generation) }
        }
        shutdownIntentTimer?.cancel()
        shutdownIntentTimer = refloatScheduler.schedule(
          afterMilliseconds: Self.shutdownIntentTimeoutMs
        ) { [weak self] in
          self?.lock.withLock { self?.expireShutdownIntent(generation: context.generation) }
        }
      } catch {
        _ = refloatMutations.clearShutdown(generation: context.generation)
        throw error
      }
    }
  }

  public func setRefloatControl(_ control: RefloatBooleanControl, enabled: Bool) throws -> RefloatControls {
    let waiter = try lock.withLock { () throws -> RefloatMutationWaiter? in
      let context = try refloatMutationContext()
      switch try refloatMutations.authorizeSet(control: control, enabled: enabled, context: context) {
      case .idempotent: return nil
      case let .mutation(mutation, state):
        let waiter = RefloatMutationWaiter()
        refloatWaiter = waiter
        events.publish(state)
        try submitRefloatMutation(mutation, route: context.route, generation: context.generation)
        armRefloatMutationTimeout(generation: context.generation)
        return waiter
      }
    }
    guard let waiter else { return refloatMutations.snapshot()! }
    return try waiter.wait(timeout: 3.25)
  }

  public func clearRefloatFatalError() throws -> RefloatControls {
    let waiter = try lock.withLock { () throws -> RefloatMutationWaiter? in
      let context = try refloatMutationContext()
      switch try refloatMutations.authorizeFatalClear(context: context) {
      case .idempotent: return nil
      case let .mutation(mutation, _):
        let waiter = RefloatMutationWaiter()
        refloatWaiter = waiter
        events.publish(refloatMutations.snapshot()!)
        try submitRefloatMutation(mutation, route: context.route, generation: context.generation)
        armRefloatMutationTimeout(generation: context.generation)
        return waiter
      }
    }
    guard let waiter else { return refloatMutations.snapshot()! }
    return try waiter.wait(timeout: 3.25)
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

  public func refloatConfigurationSummarySnapshot() -> RefloatConfigurationSummary {
    refloatConfiguration.summary()
  }

  public func refloatConfigurationSchemaSnapshot() -> RefloatConfigurationSchema? {
    refloatConfiguration.loadedSchema()
  }

  public func refloatConfigurationValuesSnapshot(defaults: Bool) -> RefloatConfigurationValues? {
    refloatConfiguration.loadedValues(defaults: defaults)
  }

  public func readRefloatConfiguration(defaults: Bool) throws -> RefloatConfigurationValues {
    let waiter = try lock.withLock { () throws -> RefloatConfigurationRequestWaiter in
      let waiter = RefloatConfigurationRequestWaiter()
      refloatConfigurationWaiter = waiter
      try performRefloatConfigurationEffect(refloatConfiguration.requestValues(defaults: defaults))
      events.publish(refloatConfiguration.summary())
      return waiter
    }
    try waiter.wait(timeout: 3.25)
    guard let values = refloatConfiguration.loadedValues(defaults: defaults) else {
      throw RefloatConfigurationServiceError.invalidState
    }
    return values
  }

  public func writeRefloatConfiguration(
    packageVersion: String, fingerprint: String, wireSignature: UInt32,
    baseRevision: UInt64, values: [String: RefloatConfigurationValue]
  ) throws -> RefloatConfigurationValues {
    let waiter = try lock.withLock { () throws -> RefloatConfigurationRequestWaiter in
      guard let telemetry = latestRefloatTelemetry,
            let firmware = controllerDiscovery.firmware,
            let observed = latestRefloatTelemetryMonotonicMs,
            clock.monotonicMilliseconds() - observed <= max(3_000, idleProbeIntervalMs * 2)
      else { throw RefloatMutationError.invalidState }
      let context = RefloatConfigurationMutationContext(
        generation: vescGeneration,
        connectionReady: connectionState == .ready,
        writable: VescCompatibility.accessMode(firmware) == .readWrite,
        rideState: telemetry.rideState,
        footpadState: telemetry.footpadState,
        packageState: telemetry.packageState,
        packageMode: telemetry.packageMode
      )
      try refloatMutations.authorizeConfiguration(context: context)
      do {
        let waiter = RefloatConfigurationRequestWaiter()
        refloatConfigurationWaiter = waiter
        let effect = try refloatConfiguration.requestWrite(
          packageVersion: packageVersion, fingerprint: fingerprint,
          wireSignature: wireSignature, baseRevision: baseRevision, values: values
        )
        performRefloatConfigurationEffect(effect)
        events.publish(refloatConfiguration.summary())
        return waiter
      } catch {
        refloatMutations.finishConfiguration(generation: context.generation)
        throw error
      }
    }
    try waiter.wait(timeout: 3.25)
    guard let values = refloatConfiguration.loadedValues(defaults: false) else {
      throw RefloatConfigurationServiceError.invalidState
    }
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
    let currentRevision = refloatConfiguration.loadedValues(defaults: false)?.revision
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
      packageVersion: record.identity.packageVersion,
      fingerprint: record.identity.schemaFingerprint,
      wireSignature: record.identity.wireSignature,
      baseRevision: baseRevision, values: values
    )
  }

  private func validateRefloatTuneValues(
    identity: RefloatConfigurationIdentityRecord, sourceRevision: UInt64?,
    values: [String: RefloatConfigurationValue]
  ) throws {
    let summary = refloatConfiguration.summary()
    guard identity.packageName == "refloat", summary.packageVersion == identity.packageVersion,
          summary.schemaFingerprint == identity.schemaFingerprint,
          summary.wireSignature == identity.wireSignature,
          sourceRevision != nil,
          refloatConfiguration.loadedValues(defaults: false)?.revision == sourceRevision,
          let schema = refloatConfiguration.loadedSchema()
    else { throw RefloatConfigurationServiceError.invalidState }
    _ = try RefloatConfigurationValueCodec.encode(values, schema: schema)
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
    waiter?.finish(result)
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
    diagnosticAttempt: Int? = nil,
    completion: @escaping @Sendable (VescRequestCompletion) -> Void = { _ in }
  ) {
    if priority == .controllerIdentification {
      events.publish(Self.identificationDiagnostic(
        context: context, attempt: diagnosticAttempt, outcome: "submitted"
      ))
    }
    vescRequests.enqueue(
      payload: payload,
      expectedResponse: expected,
      priority: priority,
      context: context,
      timeoutMs: timeoutMs,
      generation: vescGeneration
    ) { [weak self] _, outcome in
      if priority == .controllerIdentification {
        self?.events.publish(Self.identificationDiagnostic(
          context: context,
          attempt: diagnosticAttempt,
          outcome: Self.requestOutcomeDiagnostic(outcome)
        ))
      }
      completion(outcome)
    }
  }

  private func requestInitialFirmware(
    attempt: Int,
    identificationGeneration: Int,
    requestGeneration: VescRequestGeneration
  ) {
    guard activeDeviceID != nil, connectionState == .identifying,
          controllerDiscovery.identificationGeneration == identificationGeneration,
          vescGeneration == requestGeneration,
          let protocolCodec
    else { return }
    do {
      try submitVescRequest(
        payload: protocolCodec.firmwareRequest(route: .direct),
        expected: .init(command: VescCommand.getFirmwareVersion.rawValue),
        priority: .controllerIdentification,
        context: .controllerFirmware(.direct),
        timeoutMs: attempt == 1
          ? Self.initialIdentificationAttemptTimeoutMs
          : Self.identificationTimeoutMs - Self.initialIdentificationAttemptTimeoutMs,
        diagnosticAttempt: attempt
      ) { [weak self] outcome in
        guard outcome == .timedOut, attempt == 1 else { return }
        self?.lock.withLock {
          self?.requestInitialFirmware(
            attempt: 2,
            identificationGeneration: identificationGeneration,
            requestGeneration: requestGeneration
          )
        }
      }
    } catch {
      failIdentification(.transportSetupFailed, "Unable to request VESC firmware.")
      events.publish(String(describing: error))
    }
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
      if BMSIdentityPolicy.supports(value) {
        if trackingMode == .saved, logStore.active() != nil {
          failIdentification(
            .controllerChanged,
            "The connected device is a BMS, not the controller for the active session."
          )
          return
        }
        completeBMSReady(localIdentity: value)
        return
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
      if route == .direct {
        requestInitialFirmware(
          attempt: 1,
          identificationGeneration: controllerDiscovery.identificationGeneration,
          requestGeneration: vescGeneration
        )
        return
      }
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
    deviceKind = .board
    controllerDiscovery.clearIssue()
    clearDiscoveredDevices(publish: false)
    transition(.ready)
    configureRefloatControls(firmware: firmware)
    performRefloatConfigurationEffect(refloatConfiguration.begin(
      route: route,
      generation: vescGeneration,
      firmware: firmware,
      reportedVersion: controllerDiscovery.refloatPackageVersion ?? "",
      writable: VescCompatibility.accessMode(firmware) == .readWrite,
      flywheel: latestRefloatTelemetry?.packageMode == .flywheel
    ))
    events.publish(refloatConfiguration.summary())
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

  private func completeBMSReady(localIdentity: FirmwareRecord) {
    cancelIdentificationTimeout()
    cancelCanProbeTimeout()
    cancelReconnect()
    clearDiscoveredDevices(publish: false)
    deviceKind = .bms
    let effect = auxiliaryDiscovery.begin(
      localIdentity: localIdentity,
      reusableInventory: nil,
      preferredCanID: nil
    )
    transition(.ready)
    events.publish("standalone_bms_ready")
    events.publish(auxiliaryDiscovery.state)
    performAuxiliaryEffect(effect)
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
            case .response, .sent: break
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
    events.publish("connection_identification outcome=failed code=\(code.rawValue)")
    cancelIdentificationTimeout()
    cancelCanProbeTimeout()
    vescRequests.cancel(generation: vescGeneration)
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

  private func configureRefloatControls(firmware: FirmwareRecord) {
    guard let profile = controllerDiscovery.refloatControlProfile,
          let access = VescCompatibility.accessMode(firmware)
    else {
      refloatProfile = nil
      return
    }
    refloatProfile = profile
    latestRefloatTelemetry = nil
    latestRefloatTelemetryMonotonicMs = nil
    events.publish(refloatMutations.configure(
      generation: vescGeneration, profile: profile, writable: access == .readWrite
    ))
    scheduleRefloatLightingRefresh()
  }

  private func performRefloatConfigurationEffect(_ effect: RefloatConfigurationEffect) {
    guard let protocolCodec else { return }
    do {
      let payload: Data
      let expected: VescExpectedResponse
      let context: VescRequestContext
      switch effect {
      case .none:
        return
      case let .schemaChunk(route, length, offset, operation):
        payload = try protocolCodec.customConfigurationSchemaRequest(
          route: route, length: length, offset: offset
        )
        expected = .init(command: VescCommand.getCustomConfigurationXML.rawValue)
        context = .refloatConfigurationSchema(
          route: .init(route), index: RefloatConfigurationService.index,
          offset: offset, operation: operation
        )
      case let .values(route, defaults, operation):
        payload = try protocolCodec.customConfigurationValuesRequest(route: route, defaults: defaults)
        expected = .init(command: defaults
          ? VescCommand.getCustomConfigurationDefault.rawValue
          : VescCommand.getCustomConfiguration.rawValue)
        context = .refloatConfigurationValues(
          route: .init(route), index: RefloatConfigurationService.index,
          defaults: defaults, operation: operation
        )
      case let .write(route, values, operation):
        payload = try protocolCodec.customConfigurationWriteRequest(route: route, values: values)
        expected = .init(command: VescCommand.setCustomConfiguration.rawValue)
        context = .refloatConfigurationWrite(
          route: .init(route), index: RefloatConfigurationService.index, operation: operation
        )
      case let .verification(route, operation):
        payload = try protocolCodec.customConfigurationValuesRequest(route: route, defaults: false)
        expected = .init(command: VescCommand.getCustomConfiguration.rawValue)
        context = .refloatConfigurationVerification(
          route: .init(route), index: RefloatConfigurationService.index, operation: operation
        )
      }
      let requestGeneration = vescGeneration
      submitVescRequest(
        payload: payload,
        expected: expected,
        priority: .controllerWork,
        context: context,
        timeoutMs: RefloatConfigurationService.requestTimeoutMs
      ) { [weak self] outcome in
        switch outcome {
        case .response, .sent: return
        default:
          self?.refloatConfiguration.requestFailed(
            context: context, generation: requestGeneration
          )
          self?.finishRefloatConfigurationRequest(
            .failure(RefloatConfigurationServiceError.unavailable)
          )
          if case .refloatConfigurationWrite = context {
            self?.refloatMutations.finishConfiguration(generation: requestGeneration)
          }
          if case .refloatConfigurationVerification = context {
            self?.refloatMutations.finishConfiguration(generation: requestGeneration)
          }
        }
      }
    } catch {
      refloatConfiguration.abortActiveOperation(generation: vescGeneration)
      switch effect {
      case .write, .verification:
        refloatMutations.finishConfiguration(generation: vescGeneration)
      default:
        break
      }
      events.publish(String(describing: error))
    }
  }

  private func requestRefloatLighting(route: ControllerRoute, profile: RefloatControlProfile) {
    guard connectionState == .ready, refloatMutations.snapshot()?.pendingAction == nil else { return }
    do {
      let current = RefloatLightingState(
        ledsEnabled: refloatMutations.snapshot()?.ledsEnabled ?? false,
        headlightsEnabled: refloatMutations.snapshot()?.headlightsEnabled ?? false
      )
      let command = try RefloatControlCodec.lightingRequest(
        profile: profile, control: nil, current: current
      )
      let payload = try Data(VescFrameCodec.encode(VescCommandRouter.route(command, via: route)))
      submitVescRequest(
        payload: payload,
        expected: .init(command: VescCommand.customAppData.rawValue, refloatCommand: command[2]),
        priority: .auxiliary,
        context: .refloatLighting(.init(route)),
        timeoutMs: Self.canProbeTimeoutMs
      )
    } catch {
      events.publish("refloat_lighting_refresh_failed")
    }
  }

  private func scheduleRefloatLightingRefresh() {
    refloatRefreshTimer?.cancel()
    refloatRefreshTimer = refloatScheduler.schedule(afterMilliseconds: Self.refloatRefreshMs) { [weak self] in
      self?.lock.withLock {
        guard let self,
              let route = self.controllerDiscovery.controllerRoute,
              let profile = self.refloatProfile
        else { return }
        self.requestRefloatLighting(route: route, profile: profile)
        self.scheduleRefloatLightingRefresh()
      }
    }
  }

  private func refloatMutationContext() throws -> RefloatMutationContext {
    guard connectionState == .ready,
          let route = controllerDiscovery.controllerRoute,
          let profile = refloatProfile,
          let telemetry = latestRefloatTelemetry,
          let firmware = controllerDiscovery.firmware
    else { throw RefloatMutationError.invalidState }
    return RefloatMutationContext(
      generation: vescGeneration, route: route, profile: profile,
      connectionReady: true, writable: VescCompatibility.accessMode(firmware) == .readWrite,
      rideState: telemetry.rideState, packageState: telemetry.packageState,
      packageMode: telemetry.packageMode
    )
  }

  private func submitRefloatMutation(
    _ mutation: AuthorizedRefloatMutation,
    route: ControllerRoute,
    generation: VescRequestGeneration
  ) throws {
    let data = try Data(VescFrameCodec.encode(VescCommandRouter.route(mutation.payload, via: route)))
    let complete: @Sendable (VescScheduledRequest, VescRequestCompletion) -> Void = {
      [weak self] _, outcome in
      switch outcome {
      case .response, .sent: break
      case .timedOut, .queueFull, .cancelled, .staleGeneration:
        self?.lock.withLock { self?.failRefloatMutation(generation: generation) }
      }
    }
    if mutation.oneWay {
      vescRequests.enqueueOneWay(
        payload: data, expectedResponse: mutation.expectedResponse,
        priority: .ridingTelemetry, context: .refloatControl(.init(route)),
        timeoutMs: Self.refloatMutationTimeoutMs, generation: generation, completion: complete
      )
    } else {
      vescRequests.enqueue(
        payload: data, expectedResponse: mutation.expectedResponse,
        priority: .ridingTelemetry, context: .refloatControl(.init(route)),
        timeoutMs: Self.refloatMutationTimeoutMs, generation: generation, completion: complete
      )
    }
  }

  private func armRefloatMutationTimeout(generation: VescRequestGeneration) {
    refloatMutationTimer?.cancel()
    refloatMutationTimer = refloatScheduler.schedule(
      afterMilliseconds: Self.refloatMutationTimeoutMs
    ) { [weak self] in self?.lock.withLock { self?.failRefloatMutation(generation: generation) } }
  }

  private func failRefloatMutation(generation: VescRequestGeneration) {
    guard let updated = refloatMutations.failPending(generation: generation) else { return }
    events.publish("refloat_control_confirmation_timeout")
    events.publish(updated)
    finishRefloatMutation(.failure(.unavailable))
  }

  private func finishRefloatMutation(_ result: Result<RefloatControls, RefloatMutationError>) {
    refloatMutationTimer?.cancel()
    refloatMutationTimer = nil
    let waiter = refloatWaiter
    refloatWaiter = nil
    waiter?.finish(result)
  }

  private func cancelRefloatControls() {
    refloatRefreshTimer?.cancel()
    refloatRefreshTimer = nil
    refloatMutationTimer?.cancel()
    refloatMutationTimer = nil
    shutdownIntentTimer?.cancel()
    shutdownIntentTimer = nil
    refloatProfile = nil
    latestRefloatTelemetry = nil
    _ = refloatMutations.cancel()
    finishRefloatMutation(.failure(.unavailable))
  }

  private func controllerActionsSnapshot() -> ControllerActions {
    let supported = controllerDiscovery.firmware.map(VescCompatibility.supportsShutdown) ?? false
    let telemetry = latestRefloatTelemetry
    let available = connectionState == .ready
      && supported
      && telemetry?.rideState == .idle
      && telemetry?.packageMode != .flywheel
      && refloatMutations.snapshot()?.pendingAction == nil
    return refloatMutations.controllerActions(shutdownAvailable: available)
  }

  private func expireShutdownIntent(generation: VescRequestGeneration) {
    guard let actions = refloatMutations.clearShutdown(generation: generation) else { return }
    shutdownIntentTimer?.cancel()
    shutdownIntentTimer = nil
    events.publish("controller_shutdown_unconfirmed")
    events.publish(actions)
  }

  private func resetFirmwareDiscovery() {
    finishRefloatConfigurationRequest(.failure(RefloatConfigurationServiceError.unavailable))
    refloatConfiguration.reset()
    vescGeneration = vescRequests.beginGeneration()
    telemetryPolling.cancelLayout()
    telemetryPolling.stop()
    controllerDiscovery.reset()
    auxiliaryDiscovery.reset()
    deviceKind = nil
    bmsPollTimer?.cancel()
    bmsPollTimer = nil
    bmsFailureCount = 0
    currentLimits = nil
    cancelRefloatControls()
    currentSavedDevice = nil
    cancelIdentificationTimeout()
    cancelCanProbeTimeout()
    protocolCodec?.reset()
  }

  private static let discoveryPublishIntervalMs: Int64 = 250
  private static let discoveryExpiryMs: Int64 = 120_000
  private static let maxReconnectBackoffMs: Int64 = 30_000
  private static let identificationTimeoutMs: Int64 = 10_000
  private static let initialIdentificationAttemptTimeoutMs: Int64 = 2_000
  // COMM_PING_CAN can synchronously probe all 255 CAN IDs at 10 ms per ID.
  private static let canScanTimeoutMs: Int64 = 10_000
  private static let canProbeTimeoutMs: Int64 = 1_000
  private static let refloatRefreshMs: Int64 = 1_000
  private static let refloatMutationTimeoutMs: Int64 = 3_000
  /// Keep intent through a bounded BLE link-supervision timeout. A powered-off
  /// controller may not be reported disconnected immediately after the write.
  private static let shutdownIntentTimeoutMs: Int64 = 30_000

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

  private static func identificationDiagnostic(
    context: VescRequestContext,
    attempt: Int?,
    outcome: String
  ) -> String {
    let stage: String
    switch context {
    case .controllerFirmware: stage = "firmware"
    case .canPing: stage = "can_ping"
    case .refloatInfo: stage = "refloat_info"
    case .refloatLayout: stage = "telemetry_layout"
    default: stage = "other"
    }
    let route = switch context.route {
    case .direct: "direct"
    case let .can(canID): "can:\(canID)"
    case nil: "none"
    }
    return "connection_identification stage=\(stage) route=\(route) "
      + "attempt=\(attempt ?? 1) outcome=\(outcome)"
  }

  private static func requestOutcomeDiagnostic(_ outcome: VescRequestCompletion) -> String {
    switch outcome {
    case .response: "response"
    case .sent: "sent"
    case .timedOut: "timed_out"
    case .queueFull: "queue_full"
    case .cancelled: "cancelled"
    case .staleGeneration: "stale_generation"
    }
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
