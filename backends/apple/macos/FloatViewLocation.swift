// Capability status: incomplete — authorization/provider evidence remains.
// Concrete OS owner: shared CoreLocationAdapter.

import FloatViewBackend

enum FloatViewLocation {
  static func make() -> any LocationProviderPort {
    CoreLocationAdapter()
  }
}
