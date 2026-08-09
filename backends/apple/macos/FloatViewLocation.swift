// Capability status: implemented — embedded usage metadata and physical authorization evidence pass.
// Concrete OS owner: shared CoreLocationAdapter.

import FloatViewBackend

enum FloatViewLocation {
  static func make() -> any LocationProviderPort {
    CoreLocationAdapter()
  }
}
