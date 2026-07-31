import Foundation

public struct SamplingSettings: Equatable, Sendable {
  public var idleProbeIntervalMs: Int64
  public var idlePersistIntervalMs: Int64
  public var ridingPersistIntervalMs: Int64
  public var postRideTailMs: Int64
  public var reconnectDurationMs: Int64

  public init(
    idleProbeIntervalMs: Int64 = 500,
    idlePersistIntervalMs: Int64 = 1_000,
    ridingPersistIntervalMs: Int64 = 100,
    postRideTailMs: Int64 = 5_000,
    reconnectDurationMs: Int64 = 300_000
  ) {
    self.idleProbeIntervalMs = idleProbeIntervalMs
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
