import Foundation

/// Lifecycle decisions independent of ActivityKit and transport attempt IDs.
struct LiveActivityLifecyclePolicy {
  static let backgroundReconnectGrace: TimeInterval = 10

  enum EndState { case ended, dismissed }

  private(set) var generation: UInt64?
  private(set) var reconnectDeadline: TimeInterval?
  private var reconnecting = false
  private var dismissed = false
  private var retryAt: TimeInterval = 0

  mutating func receive(generation: UInt64?) {
    if self.generation != generation {
      self.generation = generation
      dismissed = false
      retryAt = 0
      reconnectDeadline = nil
      reconnecting = false
    }
  }

  /// Repeated snapshots must not extend the execution allowance.
  mutating func reconnectChanged(reconnecting: Bool, foreground: Bool, uptime: TimeInterval) {
    self.reconnecting = reconnecting
    if generation == nil || !reconnecting || foreground {
      reconnectDeadline = nil
    } else if reconnectDeadline == nil {
      reconnectDeadline = uptime + Self.backgroundReconnectGrace
    }
  }

  func shouldEndReconnect(uptime: TimeInterval) -> Bool {
    reconnectDeadline.map { uptime >= $0 } ?? false
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
    generation != nil && !reconnecting && !dismissed && foreground && enabled && uptime >= retryAt
  }
}
