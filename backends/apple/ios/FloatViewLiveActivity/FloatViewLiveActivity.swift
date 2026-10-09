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
        Text(context.isStale ? "—" : context.state.loadLabel)
          .font(.caption2.bold())
          .foregroundStyle(context.state.loadColor)
      } compactTrailing: {
        Text(context.isStale ? "—" : context.state.loadText)
          .monospacedDigit().foregroundStyle(context.state.loadColor)
      } minimal: {
        Text(context.isStale ? "—" : context.state.loadText)
          .font(.caption2.bold()).monospacedDigit()
          .foregroundStyle(context.state.loadColor)
          .accessibilityLabel("\(context.state.loadLabel) \(context.isStale ? "unavailable" : context.state.loadText)")
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
        Spacer()
        Text(stale ? "Waiting for telemetry" : "Connected").font(.caption).foregroundStyle(.secondary)
      }
      HStack(alignment: .firstTextBaseline, spacing: 16) {
        metric(state.loadLabel, value: stale ? "—" : state.loadText, color: state.loadColor)
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
