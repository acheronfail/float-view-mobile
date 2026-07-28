import AppKit
import FloatViewBluetooth
import FloatViewCore
import FloatViewHost
import FloatViewLogging
import FloatViewProtocol
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
      let runtime = MacBackendRuntime(webRoot: root)
      let server = runtime.server
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
  private let runtime: MacBackendRuntime
  private let server: LoopbackServer
  private var window: NSWindow?

  init(webRoot: URL) {
    let runtime = MacBackendRuntime(webRoot: webRoot)
    self.runtime = runtime
    server = runtime.server
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

private final class MacBackendRuntime {
  let server: LoopbackServer
  private let bluetooth: CoreBluetoothAdapter
  private let applicationService: ApplicationService

  init(webRoot: URL) {
    let relay = BluetoothEventRelay()
    let bluetooth = CoreBluetoothAdapter(eventHandler: { @Sendable event in relay.emit(event) })
    let applicationSupport = try! FileManager.default.url(
      for: .applicationSupportDirectory,
      in: .userDomainMask,
      appropriateFor: nil,
      create: true
    )
    let logRoot = applicationSupport
      .appending(path: "FloatView")
      .appending(path: "Sessions")
    let schemaDirectory = webRoot
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .appending(path: "logging/sql")
    let logStore = try! SQLiteLogStore(
      root: logRoot,
      schemaDirectory: schemaDirectory,
      availableBytes: {
        (try? logRoot.resourceValues(
          forKeys: [.volumeAvailableCapacityForImportantUsageKey]
        ).volumeAvailableCapacityForImportantUsage).map { Int64($0) } ?? Int64.max
      }
    )
    let applicationService = ApplicationService(
      bluetooth: bluetooth,
      clock: SystemClock(),
      logStore: logStore,
      protocolCodec: VescProtocolService(),
      scheduler: SystemScheduler(),
      events: IgnoreEvents()
    )
    relay.setHandler { [weak applicationService] event in
      try? applicationService?.receive(event)
    }
    self.bluetooth = bluetooth
    self.applicationService = applicationService
    server = LoopbackServer(
      webRoot: webRoot,
      platform: "macos",
      applicationService: applicationService
    )
  }
}

private final class BluetoothEventRelay: @unchecked Sendable {
  private let lock = NSLock()
  private var handler: (@Sendable (BluetoothEvent) -> Void)?

  func setHandler(_ value: @escaping @Sendable (BluetoothEvent) -> Void) {
    lock.lock()
    handler = value
    lock.unlock()
  }

  func emit(_ event: BluetoothEvent) {
    lock.lock()
    let current = handler
    lock.unlock()
    current?(event)
  }
}

private struct SystemClock: ClockPort {
  func monotonicMilliseconds() -> Int64 {
    Int64(ProcessInfo.processInfo.systemUptime * 1_000)
  }

  func now() -> Date { Date() }
}

private struct IgnoreEvents: EventSink {
  func publish(_: any Sendable) {}
}
