import ActivityKit
import Foundation

struct RideActivityAttributes: ActivityAttributes {
  struct ContentState: Codable, Hashable {
    var source: String?
    var percentage: Double?
    var distanceMeters: Double?
    var batteryPercent: Double?
    var batteryVoltage: Double?
    var highestTemperatureC: Double?
    var temperatureSource: String?
    var status: String?
    var footpadLeft: String
    var footpadRight: String
    var imperial: Bool
    var observedAt: Date?
    /// iOS 26 may report the wallpaper's appearance to widgets instead of the
    /// system setting. The native host supplies its resolved app appearance.
    var isDark: Bool? = nil

    var isReconnecting: Bool {
      status == "Reconnecting"
    }

    func boardText(stale: Bool) -> String {
      isReconnecting ? "Reconnecting" : (stale ? "—" : (status ?? loadText))
    }

    var loadLabel: String {
      switch source {
      case "duty": "Duty"
      case "motor": "Motor"
      case "battery": "Battery"
      default: "Drive load"
      }
    }

    var loadText: String {
      percentage.map { String(format: "%.0f%%", $0) } ?? "—"
    }

    var batteryText: String {
      batteryPercent.map { String(format: "%.0f%%", $0) } ?? "—"
    }

    /// Matches gauge-colors.ts: green at 30°C, orange halfway to red.
    /// Cell temperatures reach red at 45°C; other sensors at 60°C.
    var temperatureRGB: [Int]? {
      guard let value = highestTemperatureC, value.isFinite else { return nil }
      let green = [22, 163, 74]
      let orange = [234, 88, 12]
      let red = [220, 38, 38]
      let redAt = temperatureSource == "Battery" ? 45.0 : 60.0
      if value <= 30 {
        return green
      }
      if value >= redAt {
        return red
      }
      let orangeAt = (30 + redAt) / 2
      let start = value >= orangeAt ? orange : green
      let end = value >= orangeAt ? red : orange
      let amount = value >= orangeAt
        ? (value - orangeAt) / (redAt - orangeAt)
        : (value - 30) / (orangeAt - 30)
      return zip(start, end).map { Int((Double($0) + Double($1 - $0) * amount).rounded()) }
    }

    var temperatureText: String {
      guard let value = highestTemperatureC, value.isFinite else { return "—" }
      return String(format: "%.0f°%@", imperial ? value * 9 / 5 + 32 : value, imperial ? "F" : "C")
    }

    var voltageText: String {
      batteryVoltage.map { String(format: "%.1f V", $0) } ?? "—"
    }

    var distanceText: String {
      distanceMeters.map { String(format: "%.2f %@", $0 / (imperial ? 1609.344 : 1000), imperial ? "mi" : "km") } ?? "—"
    }
  }

  let generation: UInt64
}
