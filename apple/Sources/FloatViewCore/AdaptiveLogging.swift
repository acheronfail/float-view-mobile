import Foundation

public struct SamplingSettings: Equatable, Sendable {
  public var idlePersistIntervalMs: Int64
  public var ridingPersistIntervalMs: Int64
  public var postRideTailMs: Int64
  public var reconnectDurationMs: Int64

  public init(
    idlePersistIntervalMs: Int64 = 1_000,
    ridingPersistIntervalMs: Int64 = 100,
    postRideTailMs: Int64 = 15_000,
    reconnectDurationMs: Int64 = 300_000
  ) {
    self.idlePersistIntervalMs = idlePersistIntervalMs
    self.ridingPersistIntervalMs = ridingPersistIntervalMs
    self.postRideTailMs = postRideTailMs
    self.reconnectDurationMs = reconnectDurationMs
  }
}

public struct AdaptiveSamplingPolicy: Sendable {
  public let settings: SamplingSettings
  private var lastPersistedMs: Int64?
  private var previousRideState: RideState = .unknown
  private var tailUntilMs: Int64?

  public init(settings: SamplingSettings = .init()) {
    self.settings = settings
  }

  public mutating func shouldPersist(rideState: RideState, elapsedMs: Int64) -> Bool {
    if rideState == .riding {
      tailUntilMs = nil
    } else if previousRideState == .riding {
      tailUntilMs = elapsedMs + settings.postRideTailMs
    }
    let inTail = tailUntilMs.map { elapsedMs <= $0 } ?? false
    let interval = rideState == .riding || inTail
      ? settings.ridingPersistIntervalMs : settings.idlePersistIntervalMs
    let transition = rideState != previousRideState
    previousRideState = rideState
    let jitterToleranceMs = min(25, max(0, interval / 5))
    let dueThresholdMs = max(0, interval - jitterToleranceMs)
    let due = lastPersistedMs.map { elapsedMs - $0 >= dueThresholdMs } ?? true
    if transition || due {
      lastPersistedMs = elapsedMs
      return true
    }
    return false
  }
}

public enum AlertKind: String, Sendable {
  case dutyHigh
  case voltageLow
  case voltageHigh
  case motorTemperatureHigh
  case controllerTemperatureHigh
  case footpad
  case connectionLoss
}

public struct AlertRule: Sendable {
  public let id: String
  public let kind: AlertKind
  public let threshold: Double?
  public let durationMs: Int64
  public let hysteresis: Double
  public let cooldownMs: Int64

  public init(
    id: String,
    kind: AlertKind,
    threshold: Double?,
    durationMs: Int64 = 0,
    hysteresis: Double = 0,
    cooldownMs: Int64 = 10_000
  ) {
    self.id = id
    self.kind = kind
    self.threshold = threshold
    self.durationMs = durationMs
    self.hysteresis = hysteresis
    self.cooldownMs = cooldownMs
  }
}

public struct AlertEvaluator: Sendable {
  private var activeSince: [String: Int64] = [:]
  private var lastFired: [String: Int64] = [:]

  public init() {}

  public mutating func evaluate(
    _ rule: AlertRule,
    sample: TelemetrySample,
    connected: Bool,
    at elapsedMs: Int64
  ) -> Bool {
    let value: Double?
    let high: Bool
    switch rule.kind {
    case .dutyHigh: value = sample.dutyCycle; high = true
    case .voltageLow: value = sample.packVoltageV; high = false
    case .voltageHigh: value = sample.packVoltageV; high = true
    case .motorTemperatureHigh: value = sample.motorTempC; high = true
    case .controllerTemperatureHigh: value = sample.controllerTempC; high = true
    case .footpad: value = sample.footpadState.map(Double.init); high = false
    case .connectionLoss: value = connected ? 0 : 1; high = true
    }
    guard let threshold = rule.threshold, let value else { return false }
    let breached = high ? value >= threshold : value <= threshold
    if !breached {
      activeSince[rule.id] = nil
      return false
    }
    let since = activeSince[rule.id] ?? elapsedMs
    activeSince[rule.id] = since
    guard elapsedMs - since >= rule.durationMs else { return false }
    guard lastFired[rule.id].map({ elapsedMs - $0 >= rule.cooldownMs }) ?? true else {
      return false
    }
    lastFired[rule.id] = elapsedMs
    return true
  }
}
