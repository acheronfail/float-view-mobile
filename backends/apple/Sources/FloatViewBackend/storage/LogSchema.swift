public enum LogSchema {
  public static let storageVersion = "4"
}

public enum StorageLevel: String, Sendable {
  case normal
  case warning
  case critical
  case stop
}

public enum StoragePolicy {
  public static let warningBytes: Int64 = 1_073_741_824
  public static let criticalBytes: Int64 = 524_288_000
  public static let stopBytes: Int64 = 262_144_000

  public static func level(availableBytes: Int64) -> StorageLevel {
    if availableBytes < stopBytes { return .stop }
    if availableBytes < criticalBytes { return .critical }
    if availableBytes < warningBytes { return .warning }
    return .normal
  }
}
