@testable import FloatViewBackend
import Foundation
import Testing

@Test func schedulerPrioritizesControllerAndRidingWork() {
  let harness = SchedulerHarness()
  let generation = harness.subject.beginGeneration()
  harness.enqueue(1, priority: .controllerWork, context: .currentLimits(.direct), generation: generation)
  harness.enqueue(2, priority: .auxiliary, context: .bmsValues(.can(42)), generation: generation)
  harness.enqueue(3, priority: .ridingTelemetry, context: .refloatTelemetry(.direct), generation: generation)
  harness.enqueue(4, priority: .controllerIdentification, context: .controllerFirmware(.can(7)), generation: generation)

  #expect(harness.writes == [1])
  #expect(harness.subject.receive(payload: Data([1]), command: 1))
  #expect(harness.writes == [1, 4])
  #expect(harness.subject.receive(payload: Data([4]), command: 1))
  #expect(harness.writes == [1, 4, 3])
  #expect(harness.subject.receive(payload: Data([3]), command: 1))
  #expect(harness.writes == [1, 4, 3, 2])
}

@Test func schedulerAttributesSequentialRoutesAndAdvancesAfterTimeout() {
  let harness = SchedulerHarness()
  let generation = harness.subject.beginGeneration()
  harness.enqueue(7, context: .controllerFirmware(.can(7)), generation: generation)
  harness.enqueue(42, context: .auxiliaryFirmware(.can(42)), generation: generation)
  harness.timers.fireFirstActive()
  #expect(harness.writes == [7, 42])
  #expect(harness.completions.first?.1 == .timedOut)
  #expect(harness.subject.inFlightRequest?.context == .auxiliaryFirmware(.can(42)))
  #expect(harness.subject.receive(payload: Data([42]), command: 1))
  #expect(harness.completions.map(\.0.context) == [
    .controllerFirmware(.can(7)), .auxiliaryFirmware(.can(42)),
  ])
}

@Test func schedulerBoundsQueueCancelsReconnectAndRejectsLateResponses() {
  let harness = SchedulerHarness()
  let generation = harness.subject.beginGeneration()
  harness.enqueue(0, generation: generation)
  for value in 1 ... VescScheduledRequest.maxQueuedRequests {
    harness.enqueue(UInt8(value), generation: generation)
  }
  harness.enqueue(255, generation: generation)
  #expect(harness.completions.last?.1 == .queueFull)

  let newer = harness.subject.beginGeneration()
  #expect(harness.completions.filter { $0.1 == .cancelled }.count == 65)
  #expect(!harness.subject.receive(payload: Data([0]), command: 1))
  harness.enqueue(9, generation: generation)
  #expect(harness.completions.last?.1 == .staleGeneration)
  harness.enqueue(10, generation: newer)
  #expect(harness.writes.last == 10)
}

@Test func schedulerStartsResponseDeadlineWhenWorkLeavesTheQueue() {
  let harness = SchedulerHarness()
  let generation = harness.subject.beginGeneration()
  harness.enqueue(62, context: .auxiliaryCanPing, timeoutMs: 10_000, generation: generation)
  harness.enqueue(
    31, priority: .controllerWork, context: .refloatTelemetry(.direct),
    timeoutMs: 900, generation: generation
  )

  harness.clock.milliseconds = 2_550
  #expect(harness.subject.receive(payload: Data([62]), command: 1))

  #expect(harness.writes == [62, 31])
  #expect(harness.subject.inFlightRequest?.deadlineMonotonicMs == 3_450)
  #expect(harness.completions.map(\.1) == [.response(Data([62]))])
}

@Test func schedulerReleasesOneWayWriteBeforeTelemetryConfirmationWork() {
  let harness = SchedulerHarness()
  let generation = harness.subject.beginGeneration()
  harness.subject.enqueueOneWay(
    payload: Data([12]), expectedResponse: .init(command: 36, refloatCommand: 12),
    priority: .controllerWork, context: .refloatTelemetry(.direct), timeoutMs: 100,
    generation: generation
  ) { [weak harness] request, outcome in harness?.completions.append((request, outcome)) }
  harness.enqueue(
    31, priority: .ridingTelemetry, context: .refloatTelemetry(.direct),
    generation: generation
  )
  #expect(harness.writes == [12, 31])
  #expect(harness.completions.first?.1 == .sent)
  #expect(harness.subject.inFlightRequest?.payload == Data([31]))
}

@Test func schedulerKeepsConfigurationInFlightUntilItsTypedContextMatches() {
  let harness = SchedulerHarness()
  let generation = harness.subject.beginGeneration()
  harness.enqueue(
    92, priority: .controllerWork,
    context: .refloatConfigurationSchema(route: .can(42), index: 0, offset: 400, operation: 7),
    generation: generation
  )
  harness.enqueue(
    31, priority: .ridingTelemetry, context: .refloatTelemetry(.can(42)), generation: generation
  )

  #expect(!harness.subject.receive(payload: Data([92]), command: 1) { context in
    context == .refloatConfigurationSchema(route: .can(42), index: 0, offset: 0, operation: 7)
  })
  #expect(harness.writes == [92])
  #expect(harness.subject.receive(payload: Data([92]), command: 1) { context in
    context == .refloatConfigurationSchema(route: .can(42), index: 0, offset: 400, operation: 7)
  })
  #expect(harness.writes == [92, 31])
}

@Test func schedulerCanProcessFlywheelTelemetryBeforeAdvancingQueuedConfiguration() {
  let harness = SchedulerHarness()
  let generation = harness.subject.beginGeneration()
  harness.enqueue(31, priority: .ridingTelemetry, context: .refloatTelemetry(.direct), generation: generation)
  harness.enqueue(
    93, priority: .controllerWork,
    context: .refloatConfigurationValues(route: .direct, index: 0, defaults: false, operation: 4),
    generation: generation
  )
  #expect(harness.subject.receive(payload: Data([31]), command: 1, deferNext: true))
  #expect(harness.writes == [31])
  harness.subject.cancelMatching({
    if case let .refloatConfigurationValues(_, _, defaults, _) = $0 { return !defaults }
    return false
  }, resume: false)
  harness.subject.resume()
  #expect(harness.writes == [31])
  #expect(harness.completions.map(\.1) == [.response(Data([31])), .cancelled])
}

private final class SchedulerHarness: @unchecked Sendable {
  let clock = SchedulerTestClock()
  let timers = SchedulerTestTimers()
  var writes: [UInt8] = []
  var completions: [(VescScheduledRequest, VescRequestCompletion)] = []
  lazy var subject = VescRequestScheduler(
    clock: clock,
    scheduler: timers,
    write: { [weak self] data in self?.writes.append(data.first ?? 0) }
  )

  func enqueue(
    _ byte: UInt8,
    priority: VescRequestPriority = .auxiliary,
    context: VescRequestContext = .bmsValues(.direct),
    timeoutMs: Int64 = 100,
    generation: VescRequestGeneration
  ) {
    subject.enqueue(
      payload: Data([byte]), expectedResponse: .init(command: 1), priority: priority,
      context: context, timeoutMs: timeoutMs, generation: generation
    ) { [weak self] request, outcome in self?.completions.append((request, outcome)) }
  }
}

private final class SchedulerTestClock: ClockPort, @unchecked Sendable {
  var milliseconds: Int64 = 0
  func monotonicMilliseconds() -> Int64 {
    milliseconds
  }

  func now() -> Date {
    Date(timeIntervalSince1970: 0)
  }
}

private final class SchedulerTestTimers: SchedulerPort, @unchecked Sendable {
  private final class Item: @unchecked Sendable {
    var active = true
    let action: @Sendable () -> Void
    init(_ action: @escaping @Sendable () -> Void) {
      self.action = action
    }
  }

  private var items: [Item] = []
  func schedule(afterMilliseconds _: Int64, action: @escaping @Sendable () -> Void) -> any CancellationPort {
    let item = Item(action)
    items.append(item)
    return SchedulerTestCancellation { item.active = false }
  }

  func scheduleRepeating(everyMilliseconds _: Int64, action: @escaping @Sendable () -> Void) -> any CancellationPort {
    schedule(afterMilliseconds: 0, action: action)
  }

  func fireFirstActive() {
    guard let item = items.first(where: { $0.active }) else { return }
    item.active = false
    item.action()
  }
}

private final class SchedulerTestCancellation: CancellationPort, @unchecked Sendable {
  let action: @Sendable () -> Void
  init(_ action: @escaping @Sendable () -> Void) {
    self.action = action
  }

  func cancel() {
    action()
  }
}
