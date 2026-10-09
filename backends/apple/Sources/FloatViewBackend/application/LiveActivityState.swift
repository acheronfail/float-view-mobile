import Foundation

/// Bounded presentation supplied by the process owner, independent of WebViews.
public struct LiveActivityState: Codable, Hashable, Sendable {
  public let generation: UInt64
  public let observedAt: Int64?
  public let source: String?
  public let percentage: Double?
  public let distanceMeters: Double?
  public let batteryPercent: Double?
  public let imperial: Bool
}
