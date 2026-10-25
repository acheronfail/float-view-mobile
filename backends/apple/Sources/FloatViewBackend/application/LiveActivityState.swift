import Foundation

/// Bounded presentation supplied by the process owner, independent of WebViews.
public struct LiveActivityState: Codable, Hashable, Sendable {
  public let generation: UInt64
  public let observedAt: Int64?
  public let source: String?
  public let percentage: Double?
  /// Shared current-ride total, including saved distance restored on resume.
  public let distanceMeters: Double?
  public let batteryPercent: Double?
  public let batteryVoltage: Double?
  public let highestTemperatureC: Double?
  public let temperatureSource: String?
  public let status: String?
  public let footpadLeft: String
  public let footpadRight: String
  public let imperial: Bool
}
