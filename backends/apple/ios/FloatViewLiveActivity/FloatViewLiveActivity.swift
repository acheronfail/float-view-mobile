import ActivityKit
import SwiftUI
import WidgetKit

@main
struct FloatViewLiveActivity: Widget {
  var body: some WidgetConfiguration {
    ActivityConfiguration(for: RideActivityAttributes.self) { context in
      RideSummary(state: context.state, stale: context.isStale)
        .padding(16)
        .activityBackgroundTint(Color(red: 15 / 255, green: 23 / 255, blue: 42 / 255))
        .activitySystemActionForegroundColor(.white)
        .environment(\.colorScheme, .dark)
    } dynamicIsland: { context in
      DynamicIsland {
        DynamicIslandExpandedRegion(.center) {
          RideSummary(state: context.state, stale: context.isStale)
        }
      } compactLeading: {
        FootpadIndicator(state: context.state, stale: context.isStale)
          .frame(width: 24, height: 14)
      } compactTrailing: {
        Text(context.isStale ? "—" : (context.state.status ?? context.state.loadText))
          .monospacedDigit().foregroundStyle(context.state.status == nil ? context.state.loadColor : .white)
      } minimal: {
        Text(context.isStale ? "—" : (context.state.status ?? context.state.loadText))
          .font(.caption2.bold()).monospacedDigit()
          .lineLimit(1).minimumScaleFactor(0.6)
          .foregroundStyle(context.state.status == nil ? context.state.loadColor : .white)
          .accessibilityLabel("\(context.state.status == nil ? context.state.loadLabel : "Board") \(context.isStale ? "unavailable" : (context.state.status ?? context.state.loadText))")
      }
    }
  }
}

private struct RideSummary: View {
  let state: RideActivityAttributes.ContentState
  let stale: Bool

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack {
        Label("Float View", systemImage: "bolt.circle.fill").font(.caption.bold())
        FootpadIndicator(state: state, stale: stale).frame(width: 28, height: 16)
        Spacer()
        Text(stale ? "Waiting for telemetry" : "Connected").font(.caption).foregroundStyle(.secondary)
      }
      HStack(alignment: .firstTextBaseline, spacing: 16) {
        metric(state.status == nil ? state.loadLabel : "Board", value: stale ? "—" : (state.status ?? state.loadText), color: state.status == nil ? state.loadColor : .white)
        Spacer(minLength: 0)
        metric("Distance", value: stale ? "—" : state.distanceText)
        Spacer(minLength: 0)
        metric("Battery charge", value: stale ? "—" : state.batteryText)
      }
    }
    .foregroundStyle(.white)
  }

  private func metric(_ title: String, value: String, color: Color = .white) -> some View {
    VStack(alignment: .leading, spacing: 3) {
      Text(title).font(.caption2).foregroundStyle(color.opacity(0.85))
      Text(value).font(.title3.bold()).monospacedDigit().foregroundStyle(color)
        .lineLimit(1).minimumScaleFactor(0.7)
    }
    .accessibilityElement(children: .combine)
  }
}

private extension RideActivityAttributes.ContentState {
  /// Primary gauge's dark palette (app.css), suited to the Island's black surface.
  var loadColor: Color {
    switch source {
    case "duty": Color(red: 252 / 255, green: 211 / 255, blue: 77 / 255)
    case "motor": Color(red: 253 / 255, green: 164 / 255, blue: 175 / 255)
    case "battery": Color(red: 110 / 255, green: 231 / 255, blue: 183 / 255)
    default: .gray
    }
  }
}

private struct FootpadIndicator: View {
  let state: RideActivityAttributes.ContentState
  let stale: Bool

  var body: some View {
    HStack(spacing: 2) {
      Rectangle().fill(color(state.footpadLeft))
      Rectangle().fill(color(state.footpadRight))
    }
    .clipShape(Capsule())
    .overlay(Capsule().stroke(.gray.opacity(0.6), lineWidth: 1))
    .accessibilityElement(children: .ignore)
    .accessibilityLabel(stale ? "Footpads unavailable" : "Footpads: left \(label(state.footpadLeft)), right \(label(state.footpadRight))")
  }

  private func label(_ value: String) -> String {
    switch value {
    case "active": "Enabled"
    case "warning": "Disabled while moving"
    default: "Disabled"
    }
  }

  private func color(_ value: String) -> Color {
    switch stale ? "neutral" : value {
    case "active": Color(red: 110 / 255, green: 231 / 255, blue: 183 / 255)
    case "warning": Color(red: 253 / 255, green: 164 / 255, blue: 175 / 255)
    default: Color(red: 15 / 255, green: 23 / 255, blue: 42 / 255)
    }
  }
}
