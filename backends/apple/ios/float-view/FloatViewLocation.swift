// Capability status: incomplete — direct authorization-state delivery remains.
// Concrete OS owner: CoreLocationAdapter.

enum FloatViewLocation {
  static func make() -> any LocationProviderPort {
    CoreLocationAdapter()
  }
}
