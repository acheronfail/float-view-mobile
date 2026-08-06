import Foundation

public enum VescRequestPriority: Int, Sendable, Comparable {
  case auxiliary = 0
  case controllerWork = 1
  case ridingTelemetry = 2
  case controllerIdentification = 3

  public static func < (left: Self, right: Self) -> Bool {
    left.rawValue < right.rawValue
  }
}

public enum VescRequestRoute: Sendable, Equatable {
  case direct
  case can(UInt8)
}

public extension VescRequestRoute {
  init(_ route: ControllerRoute) {
    switch route {
    case .direct: self = .direct
    case let .can(canID): self = .can(canID)
    }
  }
}

public struct VescExpectedResponse: Sendable, Equatable {
  public let command: UInt8
  public let refloatCommand: UInt8?

  public init(command: UInt8, refloatCommand: UInt8? = nil) {
    self.command = command
    self.refloatCommand = refloatCommand
  }
}

public enum VescRequestContext: Sendable, Equatable {
  case controllerFirmware(VescRequestRoute)
  case canPing
  case auxiliaryCanPing
  case refloatInfo(VescRequestRoute)
  case refloatLayout(VescRequestRoute)
  case refloatTelemetry(VescRequestRoute)
  case currentLimits(VescRequestRoute)
  case auxiliaryFirmware(VescRequestRoute)
  case bmsValues(VescRequestRoute)

  public var route: VescRequestRoute? {
    switch self {
    case .canPing, .auxiliaryCanPing: nil
    case let .controllerFirmware(route), let .refloatInfo(route), let .refloatLayout(route),
         let .refloatTelemetry(route), let .currentLimits(route),
         let .auxiliaryFirmware(route), let .bmsValues(route): route
    }
  }
}

public struct VescRequestGeneration: RawRepresentable, Sendable, Equatable, Hashable {
  public let rawValue: UInt64

  public init(rawValue: UInt64) {
    self.rawValue = rawValue
  }
}

public struct VescScheduledRequest: Sendable, Equatable {
  public static let maxQueuedRequests = 64
  public static let maxPayloadBytes = 65_536

  public let sequence: UInt64
  public let payload: Data
  public let expectedResponse: VescExpectedResponse
  public let priority: VescRequestPriority
  public let context: VescRequestContext
  public let deadlineMonotonicMs: Int64
  public let generation: VescRequestGeneration

  public init(
    sequence: UInt64,
    payload: Data,
    expectedResponse: VescExpectedResponse,
    priority: VescRequestPriority,
    context: VescRequestContext,
    deadlineMonotonicMs: Int64,
    generation: VescRequestGeneration
  ) {
    precondition(payload.count <= Self.maxPayloadBytes)
    precondition(deadlineMonotonicMs >= 0)
    self.sequence = sequence
    self.payload = payload
    self.expectedResponse = expectedResponse
    self.priority = priority
    self.context = context
    self.deadlineMonotonicMs = deadlineMonotonicMs
    self.generation = generation
  }
}

public enum VescRequestCompletion: Sendable, Equatable {
  case response(Data)
  case timedOut
  case queueFull
  case cancelled
  case staleGeneration
}

public final class VescRequestScheduler: @unchecked Sendable {
  public typealias Completion = @Sendable (VescScheduledRequest, VescRequestCompletion) -> Void

  private struct Pending {
    let request: VescScheduledRequest
    let timeoutMs: Int64
    let completion: Completion
  }

  private let clock: any ClockPort
  private let timerScheduler: any SchedulerPort
  private let write: @Sendable (Data) -> Void
  private let diagnostic: @Sendable (String) -> Void
  private let lock = NSRecursiveLock()
  private var generation = VescRequestGeneration(rawValue: 0)
  private var nextSequence: UInt64 = 0
  private var queue: [Pending] = []
  private var inFlight: Pending?
  private var deadlineTimer: (any CancellationPort)?

  public init(
    clock: any ClockPort,
    scheduler: any SchedulerPort,
    write: @escaping @Sendable (Data) -> Void,
    diagnostic: @escaping @Sendable (String) -> Void = { _ in }
  ) {
    self.clock = clock
    timerScheduler = scheduler
    self.write = write
    self.diagnostic = diagnostic
  }

  @discardableResult
  public func beginGeneration() -> VescRequestGeneration {
    let cancelled: [Pending] = lock.withLock {
      generation = .init(rawValue: generation.rawValue &+ 1)
      deadlineTimer?.cancel()
      deadlineTimer = nil
      let values = queue + [inFlight].compactMap { $0 }
      queue.removeAll(keepingCapacity: true)
      inFlight = nil
      return values
    }
    cancelled.forEach { $0.completion($0.request, .cancelled) }
    return currentGeneration
  }

  public var currentGeneration: VescRequestGeneration {
    lock.withLock { generation }
  }

  public var queuedCount: Int {
    lock.withLock { queue.count }
  }

  public var inFlightRequest: VescScheduledRequest? {
    lock.withLock { inFlight?.request }
  }

  public func enqueue(
    payload: Data,
    expectedResponse: VescExpectedResponse,
    priority: VescRequestPriority,
    context: VescRequestContext,
    timeoutMs: Int64,
    generation requestGeneration: VescRequestGeneration,
    completion: @escaping Completion
  ) {
    precondition(timeoutMs > 0)
    let result: (Pending?, VescRequestCompletion?) = lock.withLock {
      nextSequence &+= 1
      let request = VescScheduledRequest(
        sequence: nextSequence,
        payload: payload,
        expectedResponse: expectedResponse,
        priority: priority,
        context: context,
        deadlineMonotonicMs: clock.monotonicMilliseconds() + timeoutMs,
        generation: requestGeneration
      )
      let pending = Pending(request: request, timeoutMs: timeoutMs, completion: completion)
      guard requestGeneration == generation else { return (pending, .staleGeneration) }
      guard queue.count < VescScheduledRequest.maxQueuedRequests else {
        return (pending, .queueFull)
      }
      queue.append(pending)
      return (nil, nil)
    }
    if let rejected = result.0, let outcome = result.1 {
      diagnostic(outcome == .queueFull ? "vesc_request_queue_full" : "vesc_request_stale_generation")
      rejected.completion(rejected.request, outcome)
      return
    }
    pump()
  }

  @discardableResult
  public func receive(
    payload: Data,
    command: UInt8,
    refloatCommand: UInt8? = nil
  ) -> Bool {
    let completed: Pending? = lock.withLock {
      guard let current = inFlight,
            current.request.generation == generation,
            current.request.expectedResponse.command == command,
            current.request.expectedResponse.refloatCommand == refloatCommand
      else { return nil }
      deadlineTimer?.cancel()
      deadlineTimer = nil
      inFlight = nil
      return current
    }
    guard let completed else {
      diagnostic("vesc_unmatched_or_late_response")
      return false
    }
    completed.completion(completed.request, .response(payload))
    pump()
    return true
  }

  public func cancel(generation cancelledGeneration: VescRequestGeneration) {
    let cancelled: [Pending] = lock.withLock {
      var values = queue.filter { $0.request.generation == cancelledGeneration }
      queue.removeAll { $0.request.generation == cancelledGeneration }
      if inFlight?.request.generation == cancelledGeneration, let current = inFlight {
        deadlineTimer?.cancel()
        deadlineTimer = nil
        inFlight = nil
        values.append(current)
      }
      return values
    }
    cancelled.forEach { $0.completion($0.request, .cancelled) }
    pump()
  }

  private func pump() {
    let send: Pending? = lock.withLock {
      guard inFlight == nil, !queue.isEmpty else { return nil }
      let index = queue.indices.min { left, right in
        let lhs = queue[left].request
        let rhs = queue[right].request
        return lhs.priority == rhs.priority ? lhs.sequence < rhs.sequence : lhs.priority > rhs.priority
      }!
      let next = queue.remove(at: index)
      guard next.request.generation == generation else { return nil }
      let dispatched = Pending(
        request: VescScheduledRequest(
          sequence: next.request.sequence,
          payload: next.request.payload,
          expectedResponse: next.request.expectedResponse,
          priority: next.request.priority,
          context: next.request.context,
          deadlineMonotonicMs: clock.monotonicMilliseconds() + next.timeoutMs,
          generation: next.request.generation
        ),
        timeoutMs: next.timeoutMs,
        completion: next.completion
      )
      inFlight = dispatched
      deadlineTimer = timerScheduler.schedule(afterMilliseconds: dispatched.timeoutMs) { [weak self] in
        self?.timeout(sequence: dispatched.request.sequence, generation: dispatched.request.generation)
      }
      return dispatched
    }
    guard let send else {
      discardStaleQueuedRequests()
      return
    }
    write(send.request.payload)
  }

  private func timeout(sequence: UInt64, generation timeoutGeneration: VescRequestGeneration) {
    let completed: Pending? = lock.withLock {
      guard let current = inFlight,
            current.request.sequence == sequence,
            current.request.generation == timeoutGeneration
      else { return nil }
      deadlineTimer = nil
      inFlight = nil
      return current
    }
    guard let completed else { return }
    completed.completion(completed.request, .timedOut)
    pump()
  }

  private func discardStaleQueuedRequests() {
    let stale: [Pending] = lock.withLock {
      let values = queue.filter { $0.request.generation != generation }
      queue.removeAll { $0.request.generation != generation }
      return values
    }
    stale.forEach { $0.completion($0.request, .staleGeneration) }
    if !stale.isEmpty { pump() }
  }
}
