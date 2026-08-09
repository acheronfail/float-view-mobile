import SwiftUI
import UIKit

struct ContentView: View {
  @StateObject private var backend = AppBackend()
  @Environment(\.scenePhase) private var scenePhase

  var body: some View {
    Group {
      if let frontend = backend.frontend {
        WebView(url: frontend.origin, onContentProcessTerminated: backend.recoverFrontend)
          .id(frontend.generation)
          .ignoresSafeArea()
      } else if scenePhase == .background {
        Color(uiColor: .systemBackground).ignoresSafeArea()
      } else {
        ContentUnavailableView("Just a sec mate...", systemImage: "gauge.with.dots.needle.33percent")
      }
    }
    .preferredColorScheme(backend.themePreference.preferredColorScheme)
    .onChange(of: scenePhase, initial: true) { _, phase in
      switch phase {
      case .active:
        backend.enterForeground()
      case .background:
        backend.enterBackground()
      case .inactive:
        break
      @unknown default:
        break
      }
    }
  }
}
