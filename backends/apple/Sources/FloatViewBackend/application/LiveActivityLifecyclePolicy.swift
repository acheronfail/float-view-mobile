import Foundation

/// Lifecycle decisions independent of ActivityKit and transport attempt IDs.
struct LiveActivityLifecyclePolicy {
  enum EndState { case ended, dismissed }

  private(set) var generation: UInt64?
  private var dismissed = false
  private var retryAt: TimeInterval = 0

  mutating func receive(generation: UInt64?) {
    if self.generation != generation {
      self.generation = generation
      dismissed = false
      retryAt = 0
    }
  }

  mutating func activityFinished(_ state: EndState) {
    // System expiry is recoverable. A person's dismissal requires explicit intent.
    if state == .dismissed {
      dismissed = true
    }
  }

  mutating func show() {
    dismissed = false
    retryAt = 0
  }

  mutating func startFailed(uptime: TimeInterval) {
    retryAt = uptime + 30
  }

  func canStart(foreground: Bool, enabled: Bool, uptime: TimeInterval) -> Bool {
    generation != nil && !dismissed && foreground && enabled && uptime >= retryAt
  }
}
