import AppKit
import FloatViewCore
import FloatViewHost
import Foundation
import WebKit

@main
enum FloatViewMac {
  static func main() throws {
    let environment = ProcessInfo.processInfo.environment
    let root = environment["FLOAT_VIEW_WEB_ROOT"].map(URL.init(fileURLWithPath:))
      ?? URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        .deletingLastPathComponent().appending(path: "web/build")
    if CommandLine.arguments.contains("--test-host") {
      let server = LoopbackServer(webRoot: root)
      try server.start()
      let readiness: [String: Any] = [
        "webSocketUrl": "ws://127.0.0.1:\(server.port)/api/v1/ws",
        "sessionToken": server.token,
        "origin": server.origin,
      ]
      let data = try JSONSerialization.data(withJSONObject: readiness)
      print(String(decoding: data, as: UTF8.self))
      fflush(stdout)
      RunLoop.current.run()
      return
    }

    let application = NSApplication.shared
    let delegate = MacAppDelegate(webRoot: root)
    application.delegate = delegate
    application.setActivationPolicy(.regular)
    application.run()
  }
}

private final class MacAppDelegate: NSObject, NSApplicationDelegate {
  private let server: LoopbackServer
  private var window: NSWindow?

  init(webRoot: URL) {
    server = LoopbackServer(webRoot: webRoot)
  }

  func applicationDidFinishLaunching(_ notification: Notification) {
    do {
      try server.start()
      let webView = WKWebView(frame: .zero)
      webView.load(URLRequest(url: URL(string: server.origin)!))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 430, height: 820),
        styleMask: [.titled, .closable, .miniaturizable, .resizable],
        backing: .buffered,
        defer: false
      )
      window.title = "Float View"
      window.contentView = webView
      window.center()
      window.makeKeyAndOrderFront(nil)
      NSApplication.shared.activate(ignoringOtherApps: true)
      self.window = window
    } catch {
      NSApplication.shared.terminate(nil)
    }
  }

  func applicationWillTerminate(_ notification: Notification) {
    server.stop()
  }

  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    true
  }
}
