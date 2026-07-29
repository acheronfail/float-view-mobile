import Foundation

public final class ApplicationService: @unchecked Sendable {
  private let bluetooth: BluetoothPort
  private let clock: ClockPort
  private let logStore: LogStorePort
  private let productStore: (any ProductStorePort)?
  private let sessionStore: (any SessionStorePort)?
  private let protocolCodec: VescProtocolPort?
  private let scheduler: (any SchedulerPort)?
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
  private let controllerDiscovery = ControllerDiscovery()
  private var currentSavedDevice: SavedDeviceRecord?
  private var identificationTimer: (any CancellationPort)?
  private var canProbeTimer: (any CancellationPort)?
  private var sessionControllerRoute: ControllerRoute?
  private var sessionControllerUUID: Data?
  private var sessionFirmware: FirmwareRecord?
  private var sessionRefloatPackageVersion: String?
  private var latestPersistedSample: TelemetrySample?
  private var reconnectTimer: (any CancellationPort)?
  private var reconnectStartedAtMs: Int64?
  private var reconnectAttempt = 0
  private lazy var telemetryPolling = TelemetryPolling(
    clock: clock,
    scheduler: scheduler,
    protocolCodec: protocolCodec,
    write: { [weak self] bytes, deviceID in
      self?.bluetooth.write(bytes, to: deviceID)
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
    samplingSettings: SamplingSettings = .init()
  ) {
    self.bluetooth = bluetooth
    self.clock = clock
    self.logStore = logStore
    self.productStore = productStore
    self.sessionStore = sessionStore
    self.protocolCodec = protocolCodec
    self.scheduler = scheduler
    self.events = events
    self.reconnectDurationMs = samplingSettings.reconnectDurationMs
    self.idleProbeIntervalMs = samplingSettings.idleProbeIntervalMs
    self.sampling = AdaptiveSamplingPolicy(settings: samplingSettings)
    self.bluetoothState = bluetooth.currentState()
    let savedIDs = ((try? productStore?.savedDevices()) ?? [])
      .map { DeviceID($0.bluetoothIdentifier) }
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

  public func boardProfile(savedDeviceID: String) throws -> BoardProfileRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    return try productStore.boardProfile(savedDeviceID: savedDeviceID)
  }

  public func updateBoardProfile(
    savedDeviceID: String,
    expectedRevision: Int,
    patchJSON: String
  ) throws -> BoardProfileRecord {
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
    guard activeDeviceID == deviceID,
          connectionState == .selectingController,
          controllerDiscovery.canSelect(canID: canID),
          let productStore,
          let savedDevice = currentSavedDevice
    else {
      throw ApplicationError.invalidControllerSelection
    }
    guard case .requestRefloatInfo(let route, let selectedFirmware?) =
      controllerDiscovery.selectController(canID: canID)
    else {
      throw ApplicationError.invalidControllerSelection
    }
    let profile = try productStore.selectController(
      savedDeviceID: savedDevice.savedDeviceID,
      canID: canID,
      controllerUUID: ControllerDiscoveryPolicy.meaningfulUUID(selectedFirmware.controllerUUID),
      at: clock.now()
    )
    events.publish(profile)
    controllerDiscovery.clearIssue()
    transition(.identifying)
    performDiscoveryEffect(.requestRefloatInfo(route: route, rememberController: nil))
  }

  public func settings() throws -> SettingsRecord {
    guard let productStore else { throw ApplicationError.productStoreUnavailable }
    return try productStore.settings()
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

  public func querySessionSeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int
  ) throws -> [SessionSeriesPoint] {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.querySeries(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs,
      fields: fields, maxPoints: maxPoints
    )
  }

  public func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64
  ) throws -> [SessionEventRecord] {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionEvents(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs
    )
  }

  public func diagnosticWindow(
    sessionID: String,
    centerElapsedMs: Int64,
    beforeMs: Int64,
    afterMs: Int64,
    fields: [String]
  ) throws -> [SessionSeriesPoint] {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.diagnosticWindow(
      sessionID: sessionID, centerElapsedMs: centerElapsedMs,
      beforeMs: beforeMs, afterMs: afterMs, fields: fields
    )
  }

  public func deleteSession(sessionID: String) throws {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    try sessionStore.deleteSession(sessionID: sessionID)
  }

  public func exportSession(
    sessionID: String,
    format: String
  ) throws -> SessionExportRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.exportSession(sessionID: sessionID, format: format)
  }

  public func sessionGroups() throws -> [SessionGroupRecord] {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionGroups()
  }

  public func sessionGroup(sessionGroupID: String) throws -> SessionGroupRecord {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    return try sessionStore.sessionGroup(sessionGroupID: sessionGroupID)
  }

  public func unlinkSessionGroup(sessionGroupID: String) throws {
    guard let sessionStore else { throw ApplicationError.productStoreUnavailable }
    try sessionStore.unlinkSessionGroup(sessionGroupID: sessionGroupID)
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

  public func connect(_ deviceID: DeviceID) throws {
    lock.lock()
    defer { lock.unlock() }
    guard knownDevicesByID[deviceID] != nil else {
      throw ApplicationError.unknownDevice
    }
    guard activeDeviceID == nil || activeDeviceID == deviceID else {
      throw ApplicationError.activeDeviceConflict
    }
    if activeScanID != nil {
      bluetooth.stopDiscovery()
      activeScanID = nil
      cancelPendingDiscoveryPublish()
    }
    activeDeviceID = deviceID
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
    resetFirmwareDiscovery()
    protocolCodec?.reset()
    transition(.connecting)
    bluetooth.connect(deviceID)
  }

  public func receive(_ event: BluetoothEvent) throws {
    lock.lock()
    defer { lock.unlock() }
    switch event {
    case .stateChanged(let state):
      bluetoothState = state
      if (state.radio != .poweredOn || !Self.permissionAllowsUse(state.scanPermission)),
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
         logStore.active() != nil,
         !explicitDisconnect
      {
        beginReconnect()
      }
      events.publish(state)
    case .deviceFound(let deviceID, let name, let rssi):
      let device = KnownDevice(
        deviceID: deviceID,
        name: name,
        rssi: rssi,
        lastSeenAt: clock.now()
      )
      knownDevicesByID[deviceID] = device
      publishDiscoveryUpdate(device)
    case .scanFailed(let reason):
      activeScanID = nil
      cancelPendingDiscoveryPublish()
      events.publish(reason)
    case .connectionChanged(let deviceID, let state):
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
      if (state == .failed || state == .disconnected),
         !explicitDisconnect,
         logStore.active() != nil
      {
        beginReconnect()
        return
      }
      if state == .failed, logStore.active() == nil, !explicitDisconnect {
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
        latestPersistedSample = nil
        resetFirmwareDiscovery()
      }
      transition(state)
    case .notification(let deviceID, let bytes):
      guard deviceID == activeDeviceID else { return }
      guard let protocolCodec else {
        events.publish(event)
        return
      }
      do {
        for message in try protocolCodec.receive(bytes) {
          switch message {
          case .firmware(let value):
            handleFirmware(value)
          case .canDevices(let canIDs):
            handleCanDevices(canIDs)
          case .refloatInfo(let version):
            guard connectionState == .identifying else { continue }
            performDiscoveryEffect(controllerDiscovery.acceptRefloatInfo(version))
          case .refloatTelemetryLayout:
            telemetryPolling.layoutEstablished()
            performDiscoveryEffect(controllerDiscovery.acceptTelemetryLayout())
          case .refloatTelemetry(let value):
            telemetryPolling.receive(value)
          }
          events.publish(message)
        }
      } catch {
        events.publish(String(describing: error))
      }
    }
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
    if let active = logStore.active() {
      events.publish(
        try logStore.close(jobID: active.jobID, at: clock.now(), reason: "explicit_disconnect")
      )
    }
    bluetooth.disconnect(deviceID)
  }

  public func ingest(_ sample: TelemetrySample) throws {
    lock.lock()
    defer { lock.unlock() }
    guard connectionState == .ready, let active = logStore.active() else { return }
    if sampling.shouldPersist(rideState: sample.rideState, elapsedMs: sample.elapsedMs) {
      try logStore.append(jobID: active.jobID, samples: [sample])
      latestPersistedSample = sample
      events.publish(sample)
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
    let savedDevice = try? productStore?.savedDevices().first {
      $0.bluetoothIdentifier == activeDeviceID.value
    }
    let activeSession = logStore.active().flatMap { record in
      try? sessionStore?.session(sessionID: record.sessionID.value)
    }
    return ActiveConnectionSnapshot(
      deviceID: activeDeviceID,
      name: knownDevicesByID[activeDeviceID]?.name,
      state: connectionState,
      savedDevice: savedDevice ?? nil,
      activeSession: activeSession ?? nil,
      latestSample: latestPersistedSample
      ,
      controllerRoute: controllerDiscovery.controllerRoute,
      controllerCandidates: controllerDiscovery.controllerCandidates,
      connectionIssue: controllerDiscovery.connectionIssue
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

  private func handleFirmware(_ value: FirmwareRecord) {
    if value.canID == nil {
      do {
        currentSavedDevice = try productStore?.ensureSavedDevice(
          deviceID: activeDeviceID ?? DeviceID(""),
          advertisedName: activeDeviceID.flatMap { knownDevicesByID[$0]?.name },
          at: clock.now()
        )
        if let currentSavedDevice {
          let saved = (try? productStore?.savedDevices()) ?? [currentSavedDevice]
          bluetooth.configureSavedDeviceIDs(Set(
            saved.map { DeviceID($0.bluetoothIdentifier) }
          ))
        }
      } catch {
        failIdentification(.transportSetupFailed, "Unable to save the VESC device.")
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
      hasActiveSession: logStore.active() != nil,
      route: sessionControllerRoute,
      controllerUUID: sessionControllerUUID
    )
  }

  private func performDiscoveryEffect(_ effect: ControllerDiscoveryEffect) {
    switch effect {
    case .probeFirmware(let route):
      guard let activeDeviceID, let protocolCodec else { return }
      if case .can(let canID) = route { scheduleCanProbeTimeout(canID: canID) }
      do {
        bluetooth.write(try protocolCodec.firmwareRequest(route: route), to: activeDeviceID)
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
      guard let activeDeviceID, let protocolCodec else { return }
      do {
        bluetooth.write(try protocolCodec.canPingRequest(), to: activeDeviceID)
      } catch {
        failIdentification(.transportSetupFailed, "Unable to discover CAN controllers.")
      }
    case .clearHintThen(let next):
      invalidateAutomaticHint()
      restartIdentificationTimeout()
      performDiscoveryEffect(next)
    case .requestRefloatInfo(let route, let selected):
      if let selected,
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
      guard let activeDeviceID, let protocolCodec else { return }
      do {
        bluetooth.write(
          try protocolCodec.refloatInfoRequest(route: route),
          to: activeDeviceID
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
    case .failed(let issue):
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
    if logStore.active() != nil {
      guard controllerContinuityIsProven(route: route, firmware: firmware) else {
        failIdentification(
          .controllerChanged,
          "The connected controller no longer matches the active session."
        )
        return
      }
    } else {
      do {
        events.publish(try logStore.create(
          deviceID: deviceID,
          advertisedName: knownDevicesByID[deviceID]?.name,
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
    transition(.ready)
    telemetryPolling.start(
      deviceID: deviceID,
      route: route,
      family: controllerDiscovery.refloatFamily ?? .dynamic,
      idleIntervalMs: idleProbeIntervalMs,
      postRideTailMs: sampling.settings.postRideTailMs
    )
  }

  /// Production hosts always provide a VESC codec and never take this path.
  private func completeWithoutProtocol() {
    guard let deviceID = activeDeviceID else { return }
    if logStore.active() == nil {
      do {
        events.publish(try logStore.create(
          deviceID: deviceID,
          advertisedName: knownDevicesByID[deviceID]?.name,
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
    if code == .controllerChanged, let active = logStore.active() {
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
    if let active = logStore.active() {
      do {
        events.publish(
          try logStore.close(jobID: active.jobID, at: clock.now(), reason: reason)
        )
      } catch {
        events.publish(String(describing: error))
      }
    }
    explicitDisconnect = true
    if let deviceID { bluetooth.disconnect(deviceID) }
    activeDeviceID = nil
    latestPersistedSample = nil
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

  private func cancelPendingDiscoveryPublish() {
    discoveryPublishTimer?.cancel()
    discoveryPublishTimer = nil
    discoveryUpdatePending = false
  }

  private func resetFirmwareDiscovery() {
    telemetryPolling.cancelLayout()
    telemetryPolling.stop()
    controllerDiscovery.reset()
    currentSavedDevice = nil
    cancelIdentificationTimeout()
    cancelCanProbeTimeout()
    protocolCodec?.reset()
  }

  private static let discoveryPublishIntervalMs: Int64 = 250
  private static let maxReconnectBackoffMs: Int64 = 30_000
  private static let identificationTimeoutMs: Int64 = 10_000
  private static let canProbeTimeoutMs: Int64 = 1_000

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
