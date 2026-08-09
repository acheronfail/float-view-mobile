// Capability status: implemented — iOS physical background evidence records
// BLE telemetry reaching the process-owned Rust runtime after WebView teardown.
// Concrete OS owner: the iOS background-mode declarations and process runtime.

enum FloatViewBackgroundExecution {
  /// Rust owns logging intent; iOS reports whether the process runtime may keep
  /// observing with the declared Bluetooth and location background modes.
  static let effectiveEligibility = true

  /// iOS has no foreground-service toggle. The declaration-backed capability
  /// is already active; this completion acknowledges Rust's logging intent.
  static func setLoggingActive(_: Bool) -> Bool {
    effectiveEligibility
  }
}
