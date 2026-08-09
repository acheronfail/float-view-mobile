// Capability status: implemented — bounded presenter tests and physical open/save roundtrip pass.
// Concrete OS owner: shared AppleSystemPresentation's macOS panels.

import FloatViewBackend

enum FloatViewDocuments {
  static func makePresenter() -> AppleSystemPresentation {
    AppleSystemPresentation()
  }
}
