import Foundation
import Testing
@testable import FloatViewCore

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

@Test func alertDurationAndCooldownAreDeterministic() {
  let sample = TelemetrySample(
    sequence: 1,
    observedAt: .init(timeIntervalSince1970: 0),
    elapsedMs: 0,
    rideState: .riding,
    dutyCycle: 0.9
  )
  let rule = AlertRule(
    id: "duty",
    kind: .dutyHigh,
    threshold: 0.8,
    durationMs: 500,
    cooldownMs: 1_000
  )
  var evaluator = AlertEvaluator()
  let decisions = [
    evaluator.evaluate(rule, sample: sample, connected: true, at: 0),
    evaluator.evaluate(rule, sample: sample, connected: true, at: 500),
    evaluator.evaluate(rule, sample: sample, connected: true, at: 600),
    evaluator.evaluate(rule, sample: sample, connected: true, at: 1_500),
  ]
  #expect(decisions == [false, true, false, true])
}
