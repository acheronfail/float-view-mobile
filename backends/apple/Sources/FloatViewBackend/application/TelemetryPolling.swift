import Foundation

/// Owns route-aware layout negotiation, one serialized poll stream, compact
/// caching, and normalized sample sequencing.
final class TelemetryPolling: @unchecked Sendable {
  private enum PendingRequest { case realtime, compact, legacy }

  private let clock: any ClockPort
  private let scheduler: (any SchedulerPort)?
  private let protocolCodec: (any VescProtocolPort)?
  private let submit: @Sendable (
    Data, VescExpectedResponse, VescRequestPriority, VescRequestContext, Int64,
    @escaping @Sendable (VescRequestCompletion) -> Void
  ) -> Void
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
  private var postRideTailMs: Int64 = 5_000
  private var pendingRequest: PendingRequest?
  private var pendingSinceMs: Int64?
  private var nextCompactDueMs: Int64?
  private var latestCompact: (value: RefloatTelemetryRecord, observedAt: Date)?
  private var active = false

  init(
    clock: any ClockPort,
    scheduler: (any SchedulerPort)?,
    protocolCodec: (any VescProtocolPort)?,
    submit: @escaping @Sendable (
      Data, VescExpectedResponse, VescRequestPriority, VescRequestContext, Int64,
      @escaping @Sendable (VescRequestCompletion) -> Void
    ) -> Void,
    onSample: @escaping @Sendable (TelemetrySample) -> Void,
    onFailure: @escaping @Sendable (String) -> Void,
    onDiagnostic: @escaping @Sendable (String) -> Void
  ) {
    self.clock = clock
    self.scheduler = scheduler
    self.protocolCodec = protocolCodec
    self.submit = submit
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
      pendingRequest = nil
      pendingSinceMs = nil
      latestCompact = nil
      nextCompactDueMs = family == .dynamic
        ? clock.monotonicMilliseconds() + Self.compactIntervalMs : nil
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

  func refresh() {
    requestSample()
  }

  func receive(_ value: RefloatTelemetryRecord) {
    lock.withLock {
      guard active, responseMatchesPending(value) else { return }
      pendingRequest = nil
      pendingSinceMs = nil
      let observedAt = clock.now()
      if family == .dynamic, value.kind == .compact {
        latestCompact = (value, observedAt)
        return
      }

      let nowMs = clock.monotonicMilliseconds()
      if startedAtMs == nil { startedAtMs = nowMs }
      sequence += 1
      let sample = normalizedSample(
        value,
        sequence: sequence,
        observedAt: observedAt,
        elapsedMs: max(0, nowMs - (startedAtMs ?? nowMs))
      )
      onSample(sample)
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
      pendingRequest = nil
      pendingSinceMs = nil
      nextCompactDueMs = nil
      latestCompact = nil
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
    guard deviceID != nil, let route, let protocolCodec else { return }
    do {
      layoutAttempts += 1
      let bytes = try RefloatPolling.layoutRequest(protocolCodec: protocolCodec, route: route)
      submit(
        bytes,
        .init(command: VescCommand.customAppData.rawValue,
              refloatCommand: RefloatCommand.realtimeDataInternalIDs.rawValue),
        .controllerWork,
        .refloatLayout(.init(route)),
        Self.requestTimeoutMs
      ) { [weak self] outcome in
        if outcome == .timedOut { self?.onDiagnostic("refloat_telemetry_layout_request_timeout") }
      }
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
      guard active, deviceID != nil, let route, let protocolCodec else { return }
      let nowMs = clock.monotonicMilliseconds()
      if pendingRequest != nil { return }
      let request: PendingRequest
      do {
        let bytes: Data
        if family == .legacyFixed {
          request = .legacy
          bytes = try RefloatPolling.telemetryRequest(
            protocolCodec: protocolCodec,
            route: route,
            family: family
          )
        } else if let due = nextCompactDueMs, nowMs >= due {
          request = .compact
          nextCompactDueMs = nowMs + Self.compactIntervalMs
          bytes = try RefloatPolling.compactRequest(protocolCodec: protocolCodec, route: route)
        } else {
          request = .realtime
          bytes = try RefloatPolling.telemetryRequest(
            protocolCodec: protocolCodec,
            route: route,
            family: family
          )
        }
        pendingRequest = request
        pendingSinceMs = nowMs
        let refloatCommand: UInt8 = switch request {
        case .realtime: RefloatCommand.realtimeDataInternal.rawValue
        case .compact, .legacy: RefloatCommand.getAllData.rawValue
        }
        let ridingPriority = previousRideState == .riding ||
          (postRideProbeUntilMs.map { nowMs <= $0 } ?? false)
        submit(
          bytes,
          .init(command: VescCommand.customAppData.rawValue, refloatCommand: refloatCommand),
          ridingPriority ? .ridingTelemetry : .controllerWork,
          .refloatTelemetry(.init(route)),
          Self.requestTimeoutMs
        ) { [weak self] outcome in
          guard outcome == .timedOut else { return }
          self?.lock.withLock {
            guard self?.pendingRequest == request else { return }
            self?.pendingRequest = nil
            self?.pendingSinceMs = nil
            self?.onDiagnostic("refloat_telemetry_request_timeout")
          }
        }
      } catch {
        onDiagnostic(String(describing: error))
      }
    }
  }

  private func responseMatchesPending(_ value: RefloatTelemetryRecord) -> Bool {
    switch (pendingRequest, family, value.kind) {
    case (.legacy?, .legacyFixed, .compact), (.realtime?, .dynamic, .realtime),
         (.compact?, .dynamic, .compact):
      return true
    default:
      onDiagnostic("refloat_unexpected_telemetry_response")
      return false
    }
  }

  private func normalizedSample(
    _ realtime: RefloatTelemetryRecord,
    sequence: Int64,
    observedAt: Date,
    elapsedMs: Int64
  ) -> TelemetrySample {
    let compact: (value: RefloatTelemetryRecord, observedAt: Date)? =
      family == .dynamic ? latestCompact : (value: realtime, observedAt: observedAt)
    return TelemetrySample(
      sequence: sequence,
      observedAt: observedAt,
      slowObservedAt: compact?.observedAt,
      elapsedMs: elapsedMs,
      rideState: realtime.rideState,
      packageState: realtime.packageState,
      packageMode: realtime.packageMode,
      footpadState: realtime.footpadState,
      stopCondition: realtime.stopCondition,
      setpointAdjustment: realtime.setpointAdjustment,
      wheelslip: realtime.wheelslip,
      darkride: realtime.darkride,
      charging: realtime.charging,
      alertReason: realtime.alertReason,
      firmwareFault: realtime.firmwareFault,
      packVoltageV: realtime.packVoltageV,
      dutyCycle: realtime.dutyCycle,
      controllerTempC: realtime.controllerTempC,
      motorTempC: realtime.motorTempC,
      motorCurrentA: realtime.motorCurrentA,
      batteryCurrentA: realtime.batteryCurrentA,
      erpm: realtime.erpm,
      speedKph: realtime.speedKph,
      pitchDeg: realtime.pitchDeg,
      balancePitchDeg: realtime.balancePitchDeg,
      rollDeg: realtime.rollDeg,
      directionalMotorCurrentA: realtime.directionalMotorCurrentA,
      filteredMotorCurrentA: realtime.filteredMotorCurrentA,
      requestedBalanceCurrentA: realtime.requestedBalanceCurrentA,
      boosterCurrentA: realtime.boosterCurrentA,
      fieldWeakeningCurrentA: compact?.value.fieldWeakeningCurrentA,
      footpadAdc1V: realtime.footpadAdc1V,
      footpadAdc2V: realtime.footpadAdc2V,
      remoteInput: realtime.remoteInput,
      setpointDeg: realtime.setpointDeg,
      atrSetpointDeg: realtime.atrSetpointDeg,
      brakeTiltSetpointDeg: realtime.brakeTiltSetpointDeg,
      torqueTiltSetpointDeg: realtime.torqueTiltSetpointDeg,
      turnTiltSetpointDeg: realtime.turnTiltSetpointDeg,
      remoteSetpointDeg: realtime.remoteSetpointDeg,
      atrAccelerationDifferenceErpmPerSecond: realtime.atrAccelerationDifferenceErpmPerSecond,
      atrSpeedBoost: realtime.atrSpeedBoost,
      distanceM: compact?.value.distanceM,
      odometerM: compact?.value.odometerM,
      ampHoursDrawnAh: compact?.value.ampHoursDrawnAh,
      ampHoursChargedAh: compact?.value.ampHoursChargedAh,
      wattHoursDrawnWh: compact?.value.wattHoursDrawnWh,
      wattHoursChargedWh: compact?.value.wattHoursChargedWh,
      batteryLevel: compact?.value.batteryLevel,
      chargingCurrentA: compact?.value.chargingCurrentA,
      chargingVoltageV: compact?.value.chargingVoltageV,
      extensionFields: realtime.extensions
    )
  }

  private static let ridingIntervalMs: Int64 = 100
  private static let compactIntervalMs: Int64 = 1_000
  private static let requestTimeoutMs: Int64 = 900
  private static let layoutRetryIntervalMs: Int64 = 1_000
  private static let maxLayoutAttempts = 5
}
