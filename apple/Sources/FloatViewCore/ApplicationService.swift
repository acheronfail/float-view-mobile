import Foundation

public final class ApplicationService: @unchecked Sendable {
  private let bluetooth: BluetoothPort
  private let clock: ClockPort
  private let logStore: LogStorePort
  private let protocolCodec: VescProtocolPort?
  private let scheduler: (any SchedulerPort)?
  private let events: EventSink
  private let reconnectDurationMs: Int64
  private let lock = NSRecursiveLock()
  private var sampling: AdaptiveSamplingPolicy
  private var explicitDisconnect = false
  private var scanSequence = 0
  private var knownDevicesByID: [DeviceID: KnownDevice] = [:]
  private var firmware: FirmwareRecord?
  private var transportFirmware: FirmwareRecord?
  private var refloatPackageVersion: String?
  private var pendingCanIDs: [UInt8] = []
  private var firmwareDiscoveryStarted = false
  private var telemetryLayoutTimer: (any CancellationPort)?
  private var telemetryLayoutAttempts = 0
  private var telemetryTimer: (any CancellationPort)?
  private var telemetryIntervalMs: Int64?
  private var telemetryStartedAtMs: Int64?
  private var telemetrySequence: Int64 = 0
  private var reconnectTimer: (any CancellationPort)?
  private var reconnectStartedAtMs: Int64?
  private var reconnectAttempt = 0

  public private(set) var activeDeviceID: DeviceID?
  public private(set) var connectionState: ConnectionState = .disconnected
  public private(set) var bluetoothState: BluetoothState
  public private(set) var activeScanID: String?
  public var knownDevices: [KnownDevice] {
    knownDevicesByID.values.sorted {
      if $0.lastSeenAt == $1.lastSeenAt { return $0.deviceID.value < $1.deviceID.value }
      return $0.lastSeenAt > $1.lastSeenAt
    }
  }

  public init(
    bluetooth: BluetoothPort,
    clock: ClockPort,
    logStore: LogStorePort,
    protocolCodec: VescProtocolPort? = nil,
    scheduler: (any SchedulerPort)? = nil,
    events: EventSink,
    samplingSettings: SamplingSettings = .init()
  ) {
    self.bluetooth = bluetooth
    self.clock = clock
    self.logStore = logStore
    self.protocolCodec = protocolCodec
    self.scheduler = scheduler
    self.events = events
    self.reconnectDurationMs = samplingSettings.reconnectDurationMs
    self.sampling = AdaptiveSamplingPolicy(settings: samplingSettings)
    self.bluetoothState = bluetooth.currentState()
  }

  @discardableResult
  public func requestBluetoothPermission() -> BluetoothState {
    lock.lock()
    defer { lock.unlock() }
    bluetooth.requestPermission()
    bluetoothState = bluetooth.currentState()
    return bluetoothState
  }

  public func startDiscovery() throws -> String {
    lock.lock()
    defer { lock.unlock() }
    guard bluetoothState.radio == .poweredOn else {
      throw ApplicationError.bluetoothUnavailable
    }
    guard bluetoothState.scanPermission == .granted ||
          bluetoothState.scanPermission == .notRequired
    else {
      throw ApplicationError.permissionRequired
    }
    if let activeScanID { return activeScanID }
    scanSequence += 1
    let scanID = "scan-\(scanSequence)"
    activeScanID = scanID
    bluetooth.startDiscovery()
    return scanID
  }

  public func stopDiscovery(scanID: String) throws {
    lock.lock()
    defer { lock.unlock() }
    guard let activeScanID else { return }
    guard scanID == activeScanID else { throw ApplicationError.invalidScan }
    bluetooth.stopDiscovery()
    self.activeScanID = nil
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
    }
    activeDeviceID = deviceID
    explicitDisconnect = false
    cancelReconnect()
    telemetryTimer?.cancel()
    telemetryTimer = nil
    telemetryIntervalMs = nil
    telemetryStartedAtMs = nil
    telemetrySequence = 0
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
      events.publish(device)
    case .scanFailed(let reason):
      activeScanID = nil
      events.publish(reason)
    case .connectionChanged(let deviceID, let state):
      guard deviceID == activeDeviceID else { return }
      if state == .ready, logStore.active() == nil {
        do {
          events.publish(try logStore.create(deviceID: deviceID, at: clock.now()))
          telemetryStartedAtMs = clock.monotonicMilliseconds()
        } catch {
          transition(.failed)
          explicitDisconnect = true
          bluetooth.disconnect(deviceID)
          events.publish(String(describing: error))
          return
        }
      }
      if state == .ready {
        cancelReconnect()
        transition(.ready)
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
      transition(state)
      if state == .disconnected {
        cancelReconnect()
        activeDeviceID = nil
        resetFirmwareDiscovery()
      }
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
            pendingCanIDs = canIDs
            requestNextCanFirmware()
          case .refloatInfo(let version):
            refloatPackageVersion = version
            if version == "1.2" || version.hasPrefix("1.2.") {
              requestRefloatTelemetryLayout()
            }
          case .refloatTelemetryLayout:
            cancelTelemetryLayoutRetry()
            startTelemetryPolling()
          case .refloatTelemetry(let value):
            try handleRefloatTelemetry(value)
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
    cancelTelemetryLayoutRetry()
    stopTelemetryPolling()
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
    }
    events.publish(sample)
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
    return ActiveConnectionSnapshot(
      deviceID: activeDeviceID,
      name: knownDevicesByID[activeDeviceID]?.name,
      state: connectionState
    )
  }

  public func firmwareSnapshot() -> FirmwareRecord? {
    lock.lock()
    defer { lock.unlock() }
    return firmware
  }

  public func refloatPackageVersionSnapshot() -> String? {
    lock.lock()
    defer { lock.unlock() }
    return refloatPackageVersion
  }

  public func requestFirmwareInfo() throws -> FirmwareRecord? {
    lock.lock()
    defer { lock.unlock() }
    guard let activeDeviceID, connectionState == .ready else {
      throw ApplicationError.deviceNotReady
    }
    if !firmwareDiscoveryStarted { startFirmwareDiscovery(activeDeviceID) }
    return firmware
  }

  private func startFirmwareDiscovery(_ deviceID: DeviceID) {
    guard !firmwareDiscoveryStarted, let protocolCodec else { return }
    firmwareDiscoveryStarted = true
    do {
      bluetooth.write(try protocolCodec.firmwareRequest(), to: deviceID)
    } catch {
      firmwareDiscoveryStarted = false
      events.publish(String(describing: error))
    }
  }

  private func handleFirmware(_ value: FirmwareRecord) {
    if value.canID == nil {
      transportFirmware = value
      if value.hardwareType == Self.vescHardwareType {
        firmware = value
        requestRefloatInfo(canID: nil)
      } else if let activeDeviceID, let protocolCodec {
        do {
          bluetooth.write(try protocolCodec.canPingRequest(), to: activeDeviceID)
        } catch {
          events.publish(String(describing: error))
        }
      }
      return
    }
    if value.hardwareType == Self.vescHardwareType, firmware == nil {
      firmware = value
      pendingCanIDs.removeAll()
      requestRefloatInfo(canID: value.canID)
      return
    }
    requestNextCanFirmware()
  }

  private func requestRefloatInfo(canID: UInt8?) {
    guard let activeDeviceID, let protocolCodec else { return }
    do {
      bluetooth.write(try protocolCodec.refloatInfoRequest(canID: canID), to: activeDeviceID)
    } catch {
      events.publish(String(describing: error))
    }
  }

  private func requestNextCanFirmware() {
    guard !pendingCanIDs.isEmpty, let activeDeviceID, let protocolCodec else { return }
    let canID = pendingCanIDs.removeFirst()
    do {
      bluetooth.write(try protocolCodec.firmwareRequest(canID: canID), to: activeDeviceID)
    } catch {
      events.publish(String(describing: error))
    }
  }

  private func requestRefloatTelemetryLayout() {
    guard telemetryLayoutTimer == nil else { return }
    telemetryLayoutAttempts = 0
    sendRefloatTelemetryLayoutRequest()
    guard let scheduler else { return }
    telemetryLayoutTimer = scheduler.scheduleRepeating(
      everyMilliseconds: Self.telemetryLayoutRetryIntervalMs
    ) { [weak self] in
      self?.retryRefloatTelemetryLayout()
    }
  }

  private func retryRefloatTelemetryLayout() {
    lock.lock()
    defer { lock.unlock() }
    guard telemetryLayoutTimer != nil else { return }
    guard telemetryLayoutAttempts < Self.maxTelemetryLayoutAttempts else {
      cancelTelemetryLayoutRetry()
      failTelemetrySetup("refloat_telemetry_layout_timeout")
      return
    }
    sendRefloatTelemetryLayoutRequest()
  }

  private func sendRefloatTelemetryLayoutRequest() {
    guard let activeDeviceID, let canID = firmware?.canID, let protocolCodec else { return }
    do {
      telemetryLayoutAttempts += 1
      bluetooth.write(
        try protocolCodec.refloatTelemetryIDsRequest(canID: canID),
        to: activeDeviceID
      )
    } catch {
      events.publish(String(describing: error))
    }
  }

  private func cancelTelemetryLayoutRetry() {
    telemetryLayoutTimer?.cancel()
    telemetryLayoutTimer = nil
    telemetryLayoutAttempts = 0
  }

  private func failTelemetrySetup(_ reason: String) {
    guard let deviceID = activeDeviceID else { return }
    transition(.failed)
    stopTelemetryPolling()
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
    events.publish(reason)
    bluetooth.disconnect(deviceID)
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

  private func startTelemetryPolling() {
    guard scheduler != nil, firmware?.canID != nil else { return }
    restartTelemetryPolling(everyMilliseconds: Self.idleProbeIntervalMs)
  }

  private func restartTelemetryPolling(everyMilliseconds interval: Int64) {
    guard let scheduler else { return }
    telemetryTimer?.cancel()
    telemetryIntervalMs = interval
    requestTelemetrySample()
    telemetryTimer = scheduler.scheduleRepeating(everyMilliseconds: interval) { [weak self] in
      self?.requestTelemetrySample()
    }
  }

  private func requestTelemetrySample() {
    lock.lock()
    defer { lock.unlock() }
    guard connectionState == .ready,
          let activeDeviceID,
          let canID = firmware?.canID,
          let protocolCodec
    else { return }
    do {
      bluetooth.write(
        try protocolCodec.refloatTelemetryRequest(canID: canID),
        to: activeDeviceID
      )
    } catch {
      events.publish(String(describing: error))
    }
  }

  private func handleRefloatTelemetry(_ value: RefloatTelemetryRecord) throws {
    let nowMs = clock.monotonicMilliseconds()
    if telemetryStartedAtMs == nil { telemetryStartedAtMs = nowMs }
    telemetrySequence += 1
    try ingest(
      TelemetrySample(
        sequence: telemetrySequence,
        observedAt: clock.now(),
        elapsedMs: max(0, nowMs - (telemetryStartedAtMs ?? nowMs)),
        rideState: value.rideState,
        packVoltageV: value.packVoltageV,
        dutyCycle: value.dutyCycle,
        controllerTempC: value.controllerTempC,
        motorTempC: value.motorTempC,
        footpadState: Int(value.footpadState),
        extensionFields: [
          "motorCurrentA": value.motorCurrentA,
          "batteryCurrentA": value.batteryCurrentA,
          "erpm": Double(value.erpm),
          "speedKph": value.speedKph,
        ]
      )
    )
    let target = value.rideState == .riding
      ? Self.ridingProbeIntervalMs : Self.idleProbeIntervalMs
    if target != telemetryIntervalMs {
      restartTelemetryPolling(everyMilliseconds: target)
    }
  }

  private func stopTelemetryPolling() {
    telemetryTimer?.cancel()
    telemetryTimer = nil
    telemetryIntervalMs = nil
  }

  private func transition(_ state: ConnectionState) {
    connectionState = state
    events.publish(state)
  }

  private func resetFirmwareDiscovery() {
    cancelTelemetryLayoutRetry()
    stopTelemetryPolling()
    firmware = nil
    transportFirmware = nil
    refloatPackageVersion = nil
    pendingCanIDs.removeAll()
    firmwareDiscoveryStarted = false
    protocolCodec?.reset()
  }

  private static let vescHardwareType: UInt8 = 0
  private static let idleProbeIntervalMs: Int64 = 500
  private static let ridingProbeIntervalMs: Int64 = 100
  private static let telemetryLayoutRetryIntervalMs: Int64 = 1_000
  private static let maxTelemetryLayoutAttempts = 5
  private static let maxReconnectBackoffMs: Int64 = 30_000

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
}
