// Capability status: implemented — direct authorization/background state and focused observation evidence pass.
// Concrete OS owner: CoreLocationAdapter.

enum FloatViewLocation {
  static func make() -> any LocationProviderPort {
    CoreLocationAdapter()
  }
}
