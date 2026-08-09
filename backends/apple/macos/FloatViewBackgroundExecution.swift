// Capability status: implemented — normal process lifecycle is the mechanism.
// Concrete OS owner: the ordinary macOS application process lifecycle.

enum FloatViewBackgroundExecution {
  /// Closing or inactivating a window does not suspend the macOS process, so
  /// the process-owned Rust runtime remains eligible without a mobile service.
  static let effectiveEligibility = true

  /// macOS has no mobile background-service surface. Return the stable
  /// unavailable result while lifecycle eligibility continues to report true.
  static func setMobileBackgroundService(_: Bool) -> Bool {
    false
  }
}
