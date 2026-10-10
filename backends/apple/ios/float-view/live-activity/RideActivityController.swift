import ActivityKit
import Foundation
import UIKit

/// One latest-value mailbox and one serial ActivityKit worker; telemetry never
/// creates an unbounded queue of tasks. The OS still controls rendering cadence.
final class RideActivityController: @unchecked Sendable {
  private let lock = NSLock()
  private var latest: LiveActivityState?
  private var worker: Task<Void, Never>?

  @MainActor
  init() {
    worker = Task { [weak self] in
      // A new process has a new connection owner. Remove orphaned activities.
      for activity in Activity<RideActivityAttributes>.activities {
        await activity.end(nil, dismissalPolicy: .immediate)
      }
      var activity: Activity<RideActivityAttributes>?
      var dismissedGeneration: UInt64?
      var lastContent: RideActivityAttributes.ContentState?
      var retryAfter = Date.distantPast
      while !Task.isCancelled {
        guard self != nil else { return }
        let state = self?.snapshot()
        if let current = activity,
           current.attributes.generation != state?.generation
        {
          await current.end(nil, dismissalPolicy: .immediate)
          activity = nil
          lastContent = nil
        }
        if state == nil {
          dismissedGeneration = nil
        }
        if let current = activity,
           current.activityState == .dismissed || current.activityState == .ended
        {
          dismissedGeneration = current.attributes.generation
          activity = nil
        }
        if let state, dismissedGeneration != state.generation {
          let content = RideActivityAttributes.ContentState(
            source: state.source, percentage: state.percentage,
            distanceMeters: state.distanceMeters, batteryPercent: state.batteryPercent,
            batteryVoltage: state.batteryVoltage,
            status: state.status, footpadLeft: state.footpadLeft, footpadRight: state.footpadRight,
            imperial: state.imperial,
            observedAt: state.observedAt.map { Date(timeIntervalSince1970: Double($0) / 1000) }
          )
          let payload = ActivityContent(
            state: content,
            staleDate: (content.observedAt ?? .distantPast).addingTimeInterval(15)
          )
          if let current = activity {
            if content != lastContent {
              await current.update(payload)
              lastContent = content
            }
          } else if UIApplication.shared.applicationState == .active,
                    ActivityAuthorizationInfo().areActivitiesEnabled,
                    Date() >= retryAfter
          {
            do {
              activity = try Activity.request(
                attributes: RideActivityAttributes(generation: state.generation),
                content: payload, pushType: nil
              )
              lastContent = content
            } catch {
              // Authorization or system capacity may change. Bound retries.
              retryAfter = Date().addingTimeInterval(30)
            }
          }
        }
        try? await Task.sleep(for: .seconds(1))
      }
    }
  }

  private func snapshot() -> LiveActivityState? {
    lock.withLock { latest }
  }

  func receive(_ state: LiveActivityState?) {
    lock.withLock { latest = state }
  }

  deinit { worker?.cancel() }
}
