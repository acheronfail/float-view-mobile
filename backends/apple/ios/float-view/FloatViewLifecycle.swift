// Capability status: incomplete — reconciled lifecycle evidence remains.
// Concrete owner: RustAppBackend, retained across disposable SwiftUI views.

import SwiftUI

@MainActor
final class RustAppBackend: ObservableObject {
  @Published private(set) var frontend: FrontendEndpoint?
  private var owner: FloatViewRuntimeBridge?
  private var generation = 0

  init() {
    owner = try? FloatViewRuntimeBridge { [weak self] endpoint, token in
      Task { @MainActor [weak self] in
        guard let self, let endpoint = authenticatedLaunchURL(endpoint: endpoint, token: token) else {
          return
        }
        generation += 1
        frontend = FrontendEndpoint(origin: endpoint, generation: generation)
      }
    }
    owner?.updateLifecycle(foreground: true)
  }

  func update(_ phase: ScenePhase) {
    owner?.updateLifecycle(foreground: phase != .background)
    if phase == .background { frontend = nil }
  }

  func recoverFrontend() {
    owner?.updateLifecycle(foreground: false)
    owner?.updateLifecycle(foreground: true)
  }

  deinit { owner?.shutdown() }
}
