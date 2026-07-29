import Foundation

/// Owns route-aware layout negotiation, polling cadence, and decoded samples.
final class TelemetryPolling: @unchecked Sendable {
  private let clock: any ClockPort
  private let scheduler: (any SchedulerPort)?
  private let protocolCodec: (any VescProtocolPort)?
  private let write: @Sendable (Data, DeviceID) -> Void
  private let onSample: @Sendable (TelemetrySample) -> Void
  private let onFailure: @Sendable (String) -> Void
  private let onDiagnostic: @Sendable (String) -> Void
  private let lock = NSRecursiveLock()

  private var layoutTimer: (any CancellationPort)?
  private var layoutAttempts = 0
  private var pollingTimer: (any CancellationPort)?
  private var pollingIntervalMs: Int64?
  private var startedAtMs: Int64?
  private var sequence: Int64 = 0
  private var previousRideState: RideState = .unknown
  private var postRideProbeUntilMs: Int64?
  private var deviceID: DeviceID?
  private var route: ControllerRoute?
  private var family: RefloatTelemetryFamily = .dynamic
  private var idleIntervalMs: Int64 = 1_000
  private var postRideTailMs: Int64 = 30_000
  private var active = false

  init(
    clock: any ClockPort,
    scheduler: (any SchedulerPort)?,
    protocolCodec: (any VescProtocolPort)?,
    write: @escaping @Sendable (Data, DeviceID) -> Void,
    onSample: @escaping @Sendable (TelemetrySample) -> Void,
    onFailure: @escaping @Sendable (String) -> Void,
    onDiagnostic: @escaping @Sendable (String) -> Void
  ) {
    self.clock = clock
    self.scheduler = scheduler
    self.protocolCodec = protocolCodec
    self.write = write
    self.onSample = onSample
    self.onFailure = onFailure
    self.onDiagnostic = onDiagnostic
  }

  func resetSession() {
    lock.withLock {
      stop()
      cancelLayout()
      startedAtMs = nil
      sequence = 0
      deviceID = nil
      route = nil
      family = .dynamic
    }
  }

  func markSessionStarted() {
    lock.withLock { startedAtMs = clock.monotonicMilliseconds() }
  }

  func beginLayout(deviceID: DeviceID, route: ControllerRoute) {
    lock.withLock {
      guard layoutTimer == nil else { return }
      self.deviceID = deviceID
      self.route = route
      layoutAttempts = 0
      sendLayoutRequest()
      layoutTimer = scheduler?.scheduleRepeating(
        everyMilliseconds: Self.layoutRetryIntervalMs
      ) { [weak self] in self?.retryLayout() }
    }
  }

  func layoutEstablished() {
    lock.withLock { cancelLayout() }
  }

  func start(
    deviceID: DeviceID,
    route: ControllerRoute,
    family: RefloatTelemetryFamily,
    idleIntervalMs: Int64,
    postRideTailMs: Int64
  ) {
    lock.withLock {
      guard scheduler != nil, protocolCodec != nil else { return }
      self.deviceID = deviceID
      self.route = route
      self.family = family
      self.idleIntervalMs = idleIntervalMs
      self.postRideTailMs = postRideTailMs
      active = true
      restart(previousRideState == .riding ? Self.ridingIntervalMs : idleIntervalMs)
    }
  }

  func updateSettings(idleIntervalMs: Int64, postRideTailMs: Int64) {
    lock.withLock {
      self.idleIntervalMs = idleIntervalMs
      self.postRideTailMs = postRideTailMs
      if active {
        restart(previousRideState == .riding ? Self.ridingIntervalMs : idleIntervalMs)
      }
    }
  }

  func receive(_ value: RefloatTelemetryRecord) {
    lock.withLock {
      guard active else { return }
      let nowMs = clock.monotonicMilliseconds()
      if startedAtMs == nil { startedAtMs = nowMs }
      sequence += 1
      onSample(
        TelemetrySample(
          sequence: sequence,
          observedAt: clock.now(),
          elapsedMs: max(0, nowMs - (startedAtMs ?? nowMs)),
          rideState: value.rideState,
          boardState: Int(value.boardState),
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
      if value.rideState == .riding {
        postRideProbeUntilMs = nil
      } else if previousRideState == .riding {
        postRideProbeUntilMs = nowMs + postRideTailMs
      }
      let inPostRideTail = postRideProbeUntilMs.map { nowMs <= $0 } ?? false
      previousRideState = value.rideState
      let target = value.rideState == .riding || inPostRideTail
        ? Self.ridingIntervalMs : idleIntervalMs
      if target != pollingIntervalMs { restart(target) }
    }
  }

  func stop() {
    lock.withLock {
      active = false
      pollingTimer?.cancel()
      pollingTimer = nil
      pollingIntervalMs = nil
      previousRideState = .unknown
      postRideProbeUntilMs = nil
    }
  }

  func cancelLayout() {
    lock.withLock {
      layoutTimer?.cancel()
      layoutTimer = nil
      layoutAttempts = 0
    }
  }

  private func retryLayout() {
    lock.withLock {
      guard layoutTimer != nil else { return }
      guard layoutAttempts < Self.maxLayoutAttempts else {
        cancelLayout()
        onFailure("refloat_telemetry_layout_timeout")
        return
      }
      sendLayoutRequest()
    }
  }

  private func sendLayoutRequest() {
    guard let deviceID, let route, let protocolCodec else { return }
    do {
      layoutAttempts += 1
      write(
        try RefloatPolling.layoutRequest(protocolCodec: protocolCodec, route: route),
        deviceID
      )
    } catch {
      onDiagnostic(String(describing: error))
    }
  }

  private func restart(_ intervalMs: Int64) {
    guard let scheduler else { return }
    pollingTimer?.cancel()
    pollingIntervalMs = intervalMs
    requestSample()
    pollingTimer = scheduler.scheduleRepeating(everyMilliseconds: intervalMs) {
      [weak self] in self?.requestSample()
    }
  }

  private func requestSample() {
    lock.withLock {
      guard active, let deviceID, let route, let protocolCodec else { return }
      do {
        write(
          try RefloatPolling.telemetryRequest(
            protocolCodec: protocolCodec,
            route: route,
            family: family
          ),
          deviceID
        )
      } catch {
        onDiagnostic(String(describing: error))
      }
    }
  }

  private static let ridingIntervalMs: Int64 = 100
  private static let layoutRetryIntervalMs: Int64 = 1_000
  private static let maxLayoutAttempts = 5
}
