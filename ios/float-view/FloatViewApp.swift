import SwiftUI

func print(_ items: Any..., separator: String = " ", terminator: String = "\n") {
  let output = items.map { "\($0)" }.joined(separator: separator)
  NSLog("%@", output)
}

@main
struct FloatViewApp: App {
  init() {
    // Start server synchronously right now, before any views are created
    LocalServer.shared.start()
  }

  var body: some Scene {
    WindowGroup {
      ContentView(server: LocalServer.shared).preferredColorScheme(.dark)
    }
  }
}
