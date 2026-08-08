// Capability status: incomplete — automated edge cases and physical open/save evidence remain.
// Concrete OS owner: shared AppleSystemPresentation's macOS panels.

import FloatViewBackend

enum FloatViewDocuments {
  static func makePresenter() -> AppleSystemPresentation {
    AppleSystemPresentation()
  }
}
