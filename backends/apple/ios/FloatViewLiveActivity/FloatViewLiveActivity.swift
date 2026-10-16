import ActivityKit
import SwiftUI
import WidgetKit

@main
struct FloatViewLiveActivity: Widget {
  var body: some WidgetConfiguration {
    ActivityConfiguration(for: RideActivityAttributes.self) { context in
      RideSummary(state: context.state, stale: context.isStale)
        .activityBackgroundTint(Color(red: 15 / 255, green: 23 / 255, blue: 42 / 255))
        .activitySystemActionForegroundColor(.white)
        .environment(\.colorScheme, .dark)
    } dynamicIsland: { context in
      DynamicIsland {
        DynamicIslandExpandedRegion(.center) {
          RideSummary(state: context.state, stale: context.isStale)
        }
      } compactLeading: {
        if context.state.status == "Stopped" {
          HStack(spacing: 3) {
            Image(systemName: "battery.100").font(.caption2)
            Text(context.isStale ? "—" : context.state.batteryText).monospacedDigit()
          }
          .foregroundStyle(.white)
          .accessibilityElement(children: .ignore)
          .accessibilityLabel("Battery \(context.isStale ? "unavailable" : context.state.batteryText)")
        } else {
          FootpadIndicator(state: context.state, stale: context.isStale)
            .frame(width: 24, height: 14)
        }
      } compactTrailing: {
        if context.state.status == "Stopped" {
          Text(context.isStale ? "—" : context.state.distanceText)
            .monospacedDigit().foregroundStyle(.white)
            .lineLimit(1).minimumScaleFactor(0.7)
            .accessibilityLabel("Ride distance \(context.isStale ? "unavailable" : context.state.distanceText)")
        } else {
          Text(context.isStale ? "—" : (context.state.status ?? context.state.loadText))
            .monospacedDigit().foregroundStyle(context.state.status == nil ? context.state.loadColor : .white)
        }
      } minimal: {
        Text(context.isStale ? "—" : (context.state.status ?? context.state.loadText))
          .font(.caption2.bold()).monospacedDigit()
          .lineLimit(1).minimumScaleFactor(0.6)
          .foregroundStyle(context.state.status == nil ? context.state.loadColor : .white)
          .accessibilityLabel(
            "\(context.state.status == nil ? context.state.loadLabel : "Board") \(context.isStale ? "unavailable" : (context.state.status ?? context.state.loadText))"
          )
      }
    }
  }
}

private struct RideSummary: View {
  let state: RideActivityAttributes.ContentState
  let stale: Bool
  @Environment(\.dynamicTypeSize) private var typeSize

  var body: some View {
    Group {
      if typeSize >= .xxLarge {
        grid
      } else {
        ViewThatFits(in: .horizontal) {
          fourColumns
          grid
        }
      }
    }
    .foregroundStyle(.white)
    .padding(typeSize >= .xxLarge ? 10 : 12)
  }

  private var fourColumns: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(spacing: 6) {
        if stale {
          Text("Waiting for telemetry").font(.caption2).foregroundStyle(.secondary)
        } else {
          Label("Float View", systemImage: "bolt.circle.fill").font(.caption2.bold())
        }
        footpads
        Spacer(minLength: 0)
      }
      HStack(alignment: .top, spacing: 4) {
        load()
        Spacer(minLength: 0)
        distance
        Spacer(minLength: 0)
        battery
        Spacer(minLength: 0)
        temperature
      }
    }
  }

  private var grid: some View {
    Grid(alignment: .topLeading, horizontalSpacing: 12, verticalSpacing: 4) {
      GridRow {
        load(showFootpads: true).frame(maxWidth: .infinity, alignment: .leading)
        distance.frame(maxWidth: .infinity, alignment: .leading)
      }
      GridRow {
        battery
        temperature
      }
    }
  }

  private var footpads: some View {
    FootpadIndicator(state: state, stale: stale).frame(width: 24, height: 14)
  }

  private func load(showFootpads: Bool = false) -> some View {
    VStack(alignment: .leading, spacing: 2) {
      HStack(spacing: 6) {
        title(state.status == nil ? "Load" : "Board")
        if showFootpads {
          footpads
        }
      }
      value(stale ? "—" : (state.status ?? state.loadText), color: state.status == nil ? state.loadColor : .white)
      if stale {
        detail("No telemetry")
      } else if state.status == nil {
        detail(state.loadLabel)
      }
    }
    .fixedSize(horizontal: true, vertical: false)
    .accessibilityElement(children: .combine)
  }

  private var distance: some View {
    metric(
      "Distance",
      value: stale ? "—" : state.distanceMeters.map {
        String(format: "%.2f", $0 / (state.imperial ? 1609.344 : 1000))
      } ?? "—",
      detail: state.imperial ? "mi" : "km"
    )
    .accessibilityLabel("Ride distance \(stale ? "unavailable" : state.distanceText)")
  }

  private var battery: some View {
    metric("Battery", value: stale ? "—" : state.batteryText, detail: stale ? "—" : state.voltageText)
      .accessibilityLabel(
        "Battery \(stale ? "unavailable" : state.batteryText), voltage \(stale ? "unavailable" : state.voltageText)"
      )
  }

  private var temperature: some View {
    VStack(alignment: .leading, spacing: 2) {
      HStack(spacing: 4) {
        Image(systemName: "thermometer.medium")
          .foregroundStyle(stale ? .secondary : state.temperatureColor)
          .accessibilityHidden(true)
        title("Temp")
      }
      .font(.caption2)
      .dynamicTypeSize(...DynamicTypeSize.xxLarge)
      value(stale ? "—" : state.temperatureText)
      detail(stale ? "—" : (state.temperatureSource ?? "—"))
    }
    .fixedSize(horizontal: true, vertical: false)
    .accessibilityElement(children: .ignore)
    .accessibilityLabel(
      "Temperature, \(stale ? "sensor" : state.temperatureSource ?? "sensor"), \(stale || state.temperatureText == "—" ? "unavailable" : state.temperatureText)"
    )
  }

  private func title(_ text: String) -> some View {
    // Bound secondary text so the three-line grid fits ActivityKit's 160-point height.
    Text(text).font(.caption2).foregroundStyle(.white.opacity(0.85))
      .dynamicTypeSize(...DynamicTypeSize.xxLarge)
  }

  private func value(_ text: String, color: Color = .white) -> some View {
    Text(text).font(.headline).monospacedDigit().foregroundStyle(color)
      .dynamicTypeSize(...DynamicTypeSize.xxxLarge)
  }

  private func detail(_ text: String) -> some View {
    Text(text).font(.caption2).monospacedDigit().foregroundStyle(.secondary)
      .dynamicTypeSize(...DynamicTypeSize.xLarge)
  }

  private func metric(_ title: String, value: String, detail: String) -> some View {
    VStack(alignment: .leading, spacing: 2) {
      self.title(title)
      self.value(value)
      self.detail(detail)
    }
    .fixedSize(horizontal: true, vertical: false)
    .accessibilityElement(children: .combine)
  }
}

private extension RideActivityAttributes.ContentState {
  var temperatureColor: Color {
    guard let rgb = temperatureRGB else { return .secondary }
    return Color(red: Double(rgb[0]) / 255, green: Double(rgb[1]) / 255, blue: Double(rgb[2]) / 255)
  }

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
    .accessibilityLabel(stale ? "Footpads unavailable" :
      "Footpads: left \(label(state.footpadLeft)), right \(label(state.footpadRight))")
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

#if DEBUG
  private extension RideActivityAttributes {
    static let preview = RideActivityAttributes(generation: 1)

    static func previewState(status: String? = nil, imperial: Bool = false,
                             distanceMeters: Double = 12340) -> ContentState
    {
      ContentState(
        source: "duty", percentage: 42, distanceMeters: distanceMeters,
        batteryPercent: 67, batteryVoltage: 75.6,
        highestTemperatureC: 54, temperatureSource: "Motor",
        status: status, footpadLeft: "active", footpadRight: "active",
        imperial: imperial, observedAt: .now
      )
    }
  }

  #Preview("Lock Screen", as: .content, using: RideActivityAttributes.preview) {
    FloatViewLiveActivity()
  } contentStates: {
    RideActivityAttributes.previewState()
    RideActivityAttributes.previewState(status: "Stopped")
    RideActivityAttributes.previewState(imperial: true)
    RideActivityAttributes.ContentState(
      footpadLeft: "neutral", footpadRight: "neutral", imperial: false
    )
  }

  #Preview("Expanded Island", as: .dynamicIsland(.expanded), using: RideActivityAttributes.preview) {
    FloatViewLiveActivity()
  } contentStates: {
    RideActivityAttributes.previewState()
  }

  #Preview("Stale readings") {
    RideSummary(state: RideActivityAttributes.previewState(), stale: true)
      .background(Color(red: 15 / 255, green: 23 / 255, blue: 42 / 255))
      .environment(\.colorScheme, .dark)
  }

  #Preview("Narrow · four columns") {
    RideSummary(state: RideActivityAttributes.previewState(imperial: true, distanceMeters: 1986666), stale: false)
      .frame(width: 343)
      .background(Color(red: 15 / 255, green: 23 / 255, blue: 42 / 255))
      .environment(\.colorScheme, .dark)
  }

  #Preview("Narrow · XXXL") {
    RideSummary(state: RideActivityAttributes.previewState(imperial: true, distanceMeters: 1986666), stale: false)
      .frame(width: 343)
      .dynamicTypeSize(.xxxLarge)
      .background(Color(red: 15 / 255, green: 23 / 255, blue: 42 / 255))
      .environment(\.colorScheme, .dark)
  }
#endif
