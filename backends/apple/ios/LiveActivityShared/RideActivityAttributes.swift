import ActivityKit
import Foundation

struct RideActivityAttributes: ActivityAttributes {
  struct ContentState: Codable, Hashable {
    var source: String?
    var percentage: Double?
    var distanceMeters: Double?
    var batteryPercent: Double?
    var imperial: Bool
    var observedAt: Date?

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

    var distanceText: String {
      distanceMeters.map { String(format: "%.2f %@", $0 / (imperial ? 1609.344 : 1000), imperial ? "mi" : "km") } ?? "—"
    }
  }

  let generation: UInt64
}
