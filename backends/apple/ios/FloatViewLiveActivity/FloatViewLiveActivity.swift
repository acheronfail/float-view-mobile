import ActivityKit
import SwiftUI
import WidgetKit

@main
struct FloatViewLiveActivity: Widget {
  var body: some WidgetConfiguration {
    ActivityConfiguration(for: RideActivityAttributes.self) { context in
      LockScreenSummary(state: context.state, stale: context.isStale)
    } dynamicIsland: { context in
      DynamicIsland {
        DynamicIslandExpandedRegion(.center) {
          RideSummary(state: context.state, stale: context.isStale)
            .environment(\.colorScheme, .dark)
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
            .environment(\.colorScheme, .dark)
        }
      } compactTrailing: {
        if context.state.isReconnecting {
          Image(systemName: "arrow.triangle.2.circlepath")
            .foregroundStyle(.white).accessibilityLabel("Reconnecting")
        } else if context.state.status == "Stopped" {
          Text(context.isStale ? "—" : context.state.distanceText)
            .monospacedDigit().foregroundStyle(.white)
            .lineLimit(1).minimumScaleFactor(0.7)
            .accessibilityLabel("Ride distance \(context.isStale ? "unavailable" : context.state.distanceText)")
        } else {
          Text(context.state.boardText(stale: context.isStale))
            .monospacedDigit().foregroundStyle(context.state.status == nil ? context.state.loadColor : .white)
        }
      } minimal: {
        if context.state.isReconnecting {
          Image(systemName: "arrow.triangle.2.circlepath")
            .foregroundStyle(.white).accessibilityLabel("Reconnecting")
        } else {
          Text(context.state.boardText(stale: context.isStale))
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
}

/// The Island always has a black surface; only the Lock Screen follows appearance.
private struct LockScreenSummary: View {
  let state: RideActivityAttributes.ContentState
  let stale: Bool
  @Environment(\.colorScheme) private var colorScheme

  private var appearance: ColorScheme {
    state.isDark.map { $0 ? .dark : .light } ?? colorScheme
  }

  var body: some View {
    RideSummary(state: state, stale: stale)
      .environment(\.colorScheme, appearance)
      .activityBackgroundTint(RideActivityPalette.background(for: appearance))
      .activitySystemActionForegroundColor(appearance == .dark ? .white : .black)
  }
}

private enum RideActivityPalette {
  static func background(for scheme: ColorScheme) -> Color {
    scheme == .dark ? Color(red: 15 / 255, green: 23 / 255, blue: 42 / 255) : .white
  }

  static func active(for scheme: ColorScheme) -> Color {
    scheme == .dark ? Color(red: 110 / 255, green: 231 / 255, blue: 183 / 255)
      : Color(red: 4 / 255, green: 120 / 255, blue: 87 / 255)
  }

  static func warning(for scheme: ColorScheme) -> Color {
    scheme == .dark ? Color(red: 253 / 255, green: 164 / 255, blue: 175 / 255)
      : Color(red: 190 / 255, green: 18 / 255, blue: 60 / 255)
  }
}

private struct RideSummary: View {
  let state: RideActivityAttributes.ContentState
  let stale: Bool
  @Environment(\.colorScheme) private var colorScheme
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
    .foregroundStyle(.primary)
    .padding(typeSize >= .xxLarge ? 10 : 12)
  }

  private var fourColumns: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(spacing: 6) {
        if stale {
          Text(state.isReconnecting ? "Reconnecting" : "Waiting for telemetry").font(.caption2)
            .foregroundStyle(.secondary)
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
      value(
        state.boardText(stale: stale),
        color: state.status == nil ? state.loadColor(for: colorScheme) : .primary
      )
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
    Text(text).font(.caption2).foregroundStyle(.primary.opacity(0.85))
      .dynamicTypeSize(...DynamicTypeSize.xxLarge)
  }

  private func value(_ text: String, color: Color = .primary) -> some View {
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

  /// Compact Island text always uses the dark palette.
  var loadColor: Color {
    loadColor(for: .dark)
  }

  /// Matches the frontend primary gauge's light and dark palettes.
  func loadColor(for scheme: ColorScheme) -> Color {
    switch source {
    case "duty": scheme == .dark ? Color(red: 252 / 255, green: 211 / 255, blue: 77 / 255)
      : Color(red: 148 / 255, green: 108 / 255, blue: 0)
    case "motor": RideActivityPalette.warning(for: scheme)
    case "battery": RideActivityPalette.active(for: scheme)
    default: .gray
    }
  }
}

private struct FootpadIndicator: View {
  @Environment(\.colorScheme) private var colorScheme
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
    case "active": RideActivityPalette.active(for: colorScheme)
    case "warning": RideActivityPalette.warning(for: colorScheme)
    default: colorScheme == .dark ? RideActivityPalette.background(for: .dark) : Color(.systemGray5)
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

  #Preview("Light · riding, stopped, stale and large text") {
    VStack(spacing: 16) {
      RideSummary(state: RideActivityAttributes.previewState(), stale: false)
      RideSummary(state: RideActivityAttributes.previewState(status: "Stopped"), stale: false)
      RideSummary(state: RideActivityAttributes.previewState(), stale: true)
      RideSummary(state: RideActivityAttributes.previewState(imperial: true), stale: false)
        .dynamicTypeSize(.xxxLarge)
    }
    .frame(width: 343)
    .background(RideActivityPalette.background(for: .light))
    .environment(\.colorScheme, .light)
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
