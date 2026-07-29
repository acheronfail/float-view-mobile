import Foundation

public enum RefloatTelemetryFamily: Sendable, Equatable {
  case legacyFixed
  case dynamic
}

public struct RefloatCompatibilityEntry: Sendable, Equatable {
  public let sourceVersion: String
  public let wireVersion: String
  public let family: RefloatTelemetryFamily
}

public enum RefloatCompatibility {
  public static let entries: [RefloatCompatibilityEntry] = [
    .init(sourceVersion: "1.0.0", wireVersion: "1.0", family: .legacyFixed),
    .init(sourceVersion: "1.1.0", wireVersion: "1.1.0", family: .dynamic),
    .init(sourceVersion: "1.2.3", wireVersion: "1.2.3", family: .dynamic),
  ]

  public static func entry(wireVersion: String) -> RefloatCompatibilityEntry? {
    entries.first { $0.wireVersion == wireVersion }
  }
}
