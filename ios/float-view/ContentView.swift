import SwiftUI

final class AppBackend: ObservableObject {
  let server: LoopbackServer
  @Published var url: URL?

  init() {
    let webRoot = Bundle.main.resourceURL!.appendingPathComponent("build", isDirectory: true)
    server = LoopbackServer(webRoot: webRoot, platform: "ios")
    do {
      try server.start()
      url = URL(string: server.origin)
    } catch {
      url = nil
    }
  }

  deinit {
    server.stop()
  }
}

struct ContentView: View {
  @StateObject private var backend = AppBackend()

  var body: some View {
    if let url = backend.url {
      WebView(url: url).ignoresSafeArea()
    } else {
      ContentUnavailableView("Backend unavailable", systemImage: "exclamationmark.triangle")
    }
  }
}
