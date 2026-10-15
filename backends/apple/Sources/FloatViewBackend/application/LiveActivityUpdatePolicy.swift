import Foundation

/// ActivityKit submission policy, not a guarantee of OS rendering frequency.
/// Callers supply the exact formatted strings used by the widget.
struct LiveActivityUpdatePolicy {
  static let staleAfter: TimeInterval = 15

  struct Presentation: Equatable {
    let source: String?
    let load: String
    let distance: String
    let battery: String
    let voltage: String
    let temperature: String
    let temperatureSource: String?
    let temperatureRGB: [Int]?
    let status: String?
    let footpadLeft: String
    let footpadRight: String
    let imperial: Bool
  }

  enum Reason: String {
    case initial, transition, display, freshness, baseline
  }

  private var lastPresentation: Presentation?
  private var lastObservedAt: Date?
  private var lastWasFresh = false
  private var lastSubmittedAt: TimeInterval?

  func reason(
    for presentation: Presentation, observedAt: Date?, now: Date, uptime: TimeInterval
  ) -> Reason? {
    guard let previous = lastPresentation, let submittedAt = lastSubmittedAt else { return .initial }
    let fresh = isFresh(observedAt, now: now)
    if presentation.status != previous.status || presentation.footpadLeft != previous.footpadLeft
      || presentation.footpadRight != previous.footpadRight || fresh != lastWasFresh
      || (fresh && !isFresh(lastObservedAt, now: now))
    {
      return .transition
    }
    let elapsed = uptime - submittedAt
    if presentation != previous, elapsed >= 5 {
      return .display
    }
    // Advance the stale deadline even when formatted readings have not changed.
    // Never make a stopped source appear fresh by refreshing with wall-clock time.
    if fresh, observedAt != lastObservedAt, elapsed >= 10 {
      return .freshness
    }
    return nil
  }

  mutating func submitted(
    _ presentation: Presentation, observedAt: Date?, now: Date, uptime: TimeInterval
  ) {
    lastPresentation = presentation
    lastObservedAt = observedAt
    lastWasFresh = isFresh(observedAt, now: now)
    lastSubmittedAt = uptime
  }

  private func isFresh(_ observedAt: Date?, now: Date) -> Bool {
    observedAt.map { now < $0.addingTimeInterval(Self.staleAfter) } ?? false
  }
}
