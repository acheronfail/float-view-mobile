import ActivityKit
import Foundation
import OSLog
import UIKit

/// One latest-value mailbox and one serial ActivityKit worker; telemetry never
/// creates an unbounded queue of tasks. The OS still controls rendering cadence.
final class RideActivityController: @unchecked Sendable {
  private static let logger = Logger(subsystem: "fail.acheron.float-view", category: "LiveActivity")
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
      // Debug launch environment only; compare against the original one-second
      // submission behavior on the same phone without shipping a product setting.
      var baseline = false
      #if DEBUG
        baseline = ProcessInfo.processInfo.environment["FLOAT_VIEW_LIVE_ACTIVITY_BASELINE"] == "1"
      #endif
      Self.logger.info("worker_started baseline=\(baseline, privacy: .public)")
      var policy = LiveActivityUpdatePolicy()
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
          policy = LiveActivityUpdatePolicy()
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
            staleDate: (content.observedAt ?? .distantPast).addingTimeInterval(LiveActivityUpdatePolicy.staleAfter)
          )
          let presentation = LiveActivityUpdatePolicy.Presentation(
            source: content.status == nil ? content.source : nil,
            load: content.status == nil ? content.loadText : "—", distance: content.distanceText,
            battery: content.batteryText, voltage: content.voltageText,
            status: content.status, footpadLeft: content.footpadLeft, footpadRight: content.footpadRight,
            imperial: content.imperial
          )
          let now = Date()
          let uptime = ProcessInfo.processInfo.systemUptime
          if let current = activity {
            let reason = baseline
              ? (content != lastContent ? LiveActivityUpdatePolicy.Reason.baseline : nil)
              : policy.reason(for: presentation, observedAt: content.observedAt, now: now, uptime: uptime)
            if let reason {
              // Unified logging supplies wall-clock start/completion timestamps.
              // No addresses, activity IDs, or telemetry values are logged.
              let age = Self.sourceAgeMilliseconds(content.observedAt)
              Self.logger
                .info("update_begin reason=\(reason.rawValue, privacy: .public) source_age_ms=\(age, privacy: .public)")
              await current.update(payload)
              let duration = Int((ProcessInfo.processInfo.systemUptime - uptime) * 1000)
              let completedAge = Self.sourceAgeMilliseconds(content.observedAt)
              Self.logger
                .info(
                  "update_end duration_ms=\(duration, privacy: .public) source_age_ms=\(completedAge, privacy: .public)"
                )
              policy.submitted(presentation, observedAt: content.observedAt, now: now, uptime: uptime)
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
              policy.submitted(presentation, observedAt: content.observedAt, now: now, uptime: uptime)
              lastContent = content
              Self.logger.info("activity_started")
            } catch {
              // Authorization or system capacity may change. Bound retries.
              retryAfter = Date().addingTimeInterval(30)
              Self.logger.info("activity_start_failed retry_seconds=30")
            }
          }
        }
        try? await Task.sleep(for: .seconds(1))
      }
    }
  }

  private static func sourceAgeMilliseconds(_ observedAt: Date?) -> Int {
    guard let observedAt else { return -1 }
    return Int(min(86_400_000, max(0, Date().timeIntervalSince(observedAt) * 1000)))
  }

  private func snapshot() -> LiveActivityState? {
    lock.withLock { latest }
  }

  func receive(_ state: LiveActivityState?) {
    lock.withLock { latest = state }
  }

  deinit { worker?.cancel() }
}
