@testable import FloatViewBackend
import Foundation
import Testing

@Test func adaptiveSamplingPersistsTransitionsAndTail() {
  var policy = AdaptiveSamplingPolicy(
    settings: SamplingSettings(
      idlePersistIntervalMs: 1_000,
      ridingPersistIntervalMs: 100,
      postRideTailMs: 500
    )
  )
  let decisions = [
    policy.shouldPersist(rideState: .idle, elapsedMs: 0),
    policy.shouldPersist(rideState: .idle, elapsedMs: 500),
    policy.shouldPersist(rideState: .riding, elapsedMs: 600),
    policy.shouldPersist(rideState: .riding, elapsedMs: 700),
    policy.shouldPersist(rideState: .idle, elapsedMs: 750),
    policy.shouldPersist(rideState: .idle, elapsedMs: 850),
    policy.shouldPersist(rideState: .idle, elapsedMs: 1_300),
  ]
  #expect(decisions == [true, false, true, true, true, true, false])
}

@Test func adaptiveSamplingToleratesBoundedTimerJitter() {
  var policy = AdaptiveSamplingPolicy(
    settings: SamplingSettings(
      idlePersistIntervalMs: 1_000,
      ridingPersistIntervalMs: 100,
      postRideTailMs: 500
    )
  )
  let decisions = [
    policy.shouldPersist(rideState: .riding, elapsedMs: 0),
    policy.shouldPersist(rideState: .riding, elapsedMs: 79),
    policy.shouldPersist(rideState: .riding, elapsedMs: 80),
    policy.shouldPersist(rideState: .riding, elapsedMs: 159),
    policy.shouldPersist(rideState: .riding, elapsedMs: 160),
  ]
  #expect(decisions == [true, false, true, false, true])
}
