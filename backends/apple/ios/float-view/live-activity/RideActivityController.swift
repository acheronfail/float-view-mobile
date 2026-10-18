import ActivityKit
import Foundation
import OSLog
import UIKit

/// One latest-value mailbox and one serial ActivityKit worker; telemetry never
/// creates an unbounded queue of tasks. The OS still controls rendering cadence.
final class RideActivityController: @unchecked Sendable {
  private static let logger = Logger(subsystem: "fail.acheron.floatview", category: "LiveActivity")
  private let lock = NSLock()
  private var latest: LiveActivityState?
  private var worker: Task<Void, Never>?
  private var timer: Task<Void, Never>?
  private var activityObserver: Task<Void, Never>?
  private let wake: AsyncStream<Void>.Continuation
  @MainActor private var activity: Activity<RideActivityAttributes>?
  @MainActor private var lifecycle = LiveActivityLifecyclePolicy()
  @MainActor private var policy = LiveActivityUpdatePolicy()
  @MainActor private var lastContent: RideActivityAttributes.ContentState?
  @MainActor private weak static var current: RideActivityController?
  @MainActor private static var showRequested = false
  @MainActor private var baseline = false
  @MainActor private var explicitlyRequested = false

  /// The explicit Shortcuts action may run before SwiftUI creates the app owner.
  @MainActor
  static func requestPresentation() {
    if let current {
      current.explicitlyRequested = true
      current.wake.yield(())
    } else {
      showRequested = true
    }
  }

  func foregroundChanged() {
    wake.yield(())
  }

  @MainActor private var themePreference: ThemePreference = .system

  @MainActor
  func setThemePreference(_ preference: ThemePreference) {
    themePreference = preference
    wake.yield(())
  }

  @MainActor
  private var isDark: Bool {
    switch themePreference {
    case .dark: true
    case .light: false
    case .system:
      UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        .first?.traitCollection.userInterfaceStyle == .dark
    }
  }

  @MainActor
  init() {
    let (events, continuation) = AsyncStream<Void>.makeStream(bufferingPolicy: .bufferingNewest(1))
    wake = continuation
    Self.current = self
    if Self.showRequested {
      explicitlyRequested = true
      Self.showRequested = false
    }
    #if DEBUG
      baseline = ProcessInfo.processInfo.environment["FLOAT_VIEW_LIVE_ACTIVITY_BASELINE"] == "1"
    #endif
    let baselineMode = baseline
    Self.logger.info("worker_started baseline=\(baselineMode, privacy: .public)")
    worker = Task { [weak self] in
      // A new process has a new connection owner. Remove orphaned activities.
      for activity in Activity<RideActivityAttributes>.activities {
        await activity.end(nil, dismissalPolicy: .immediate)
      }
      for await _ in events {
        guard let self, !Task.isCancelled else { return }
        await reconcile()
      }
    }
    // Fallback for freshness deadlines, bounded retries and system appearance.
    timer = Task {
      while !Task.isCancelled {
        try? await Task.sleep(for: .seconds(1))
        if !Task.isCancelled {
          continuation.yield(())
        }
      }
    }
    wake.yield(())
  }

  @MainActor
  private func reconcile() async {
    let state = snapshot()
    lifecycle.receive(generation: state?.generation)
    if let current = activity, current.attributes.generation != state?.generation {
      activityObserver?.cancel()
      activityObserver = nil
      await current.end(nil, dismissalPolicy: .immediate)
      activity = nil
      policy = LiveActivityUpdatePolicy()
      lastContent = nil
      Self.logger.info("activity_ended reason=connection_finished")
    }
    if let current = activity,
       current.activityState == .dismissed || current.activityState == .ended
    {
      let dismissed = current.activityState == .dismissed
      lifecycle.activityFinished(dismissed ? .dismissed : .ended)
      activityObserver?.cancel()
      activityObserver = nil
      activity = nil
      policy = LiveActivityUpdatePolicy()
      lastContent = nil
      Self.logger.info("activity_finished dismissed=\(dismissed, privacy: .public)")
    }
    if explicitlyRequested {
      lifecycle.show()
      explicitlyRequested = false
    }
    if let state {
      let content = RideActivityAttributes.ContentState(
        source: state.source, percentage: state.percentage,
        distanceMeters: state.distanceMeters, batteryPercent: state.batteryPercent,
        batteryVoltage: state.batteryVoltage,
        highestTemperatureC: state.highestTemperatureC, temperatureSource: state.temperatureSource,
        status: state.status, footpadLeft: state.footpadLeft, footpadRight: state.footpadRight,
        imperial: state.imperial,
        observedAt: state.observedAt.map { Date(timeIntervalSince1970: Double($0) / 1000) },
        isDark: isDark
      )
      let payload = ActivityContent(
        state: content,
        staleDate: (content.observedAt ?? .distantPast).addingTimeInterval(LiveActivityUpdatePolicy.staleAfter)
      )
      let presentation = LiveActivityUpdatePolicy.Presentation(
        source: content.status == nil ? content.source : nil,
        load: content.status == nil ? content.loadText : "—", distance: content.distanceText,
        battery: content.batteryText, voltage: content.voltageText,
        temperature: content.temperatureText, temperatureSource: content.temperatureSource,
        temperatureRGB: content.temperatureRGB,
        status: content.status, footpadLeft: content.footpadLeft, footpadRight: content.footpadRight,
        imperial: content.imperial
      )
      let now = Date()
      let uptime = ProcessInfo.processInfo.systemUptime
      if let current = activity {
        let appearanceChanged = content.isDark != lastContent?.isDark
        let reason = appearanceChanged ? LiveActivityUpdatePolicy.Reason.transition : baseline
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
      } else if lifecycle.canStart(
        foreground: UIApplication.shared.applicationState == .active,
        enabled: ActivityAuthorizationInfo().areActivitiesEnabled, uptime: uptime
      ) {
        do {
          activity = try Activity.request(
            attributes: RideActivityAttributes(generation: state.generation),
            content: payload, pushType: nil
          )
          if let activity {
            activityObserver = Task { [weak self] in
              for await _ in activity.activityStateUpdates {
                guard !Task.isCancelled else { return }
                self?.wake.yield(())
              }
            }
          }
          policy.submitted(presentation, observedAt: content.observedAt, now: now, uptime: uptime)
          lastContent = content
          Self.logger.info("activity_started")
        } catch {
          // Authorization or system capacity may change. Bound retries.
          lifecycle.startFailed(uptime: ProcessInfo.processInfo.systemUptime)
          Self.logger.info("activity_start_failed retry_seconds=30")
        }
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
    wake.yield(())
  }

  deinit {
    wake.finish()
    worker?.cancel()
    timer?.cancel()
    activityObserver?.cancel()
  }
}
