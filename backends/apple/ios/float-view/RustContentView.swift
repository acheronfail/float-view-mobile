import SwiftUI

struct FrontendEndpoint {
  let origin: URL
  let generation: Int
}

struct RustContentView: View {
  @StateObject private var backend = RustAppBackend()
  @Environment(\.scenePhase) private var scenePhase

  var body: some View {
    Group {
      if let frontend = backend.frontend {
        FloatViewWebView(url: frontend.origin, onContentProcessTerminated: backend.recoverFrontend)
          .id(frontend.generation)
          .ignoresSafeArea()
      } else {
        ContentUnavailableView("Just a sec mate...", systemImage: "gauge.with.dots.needle.33percent")
      }
    }
    .onChange(of: scenePhase, initial: true) { _, phase in backend.update(phase) }
  }
}
