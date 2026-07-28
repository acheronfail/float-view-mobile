import AppKit
import FloatViewBluetooth
import FloatViewCore
import FloatViewHost
import FloatViewLogging
import FloatViewProtocol
import Foundation
import WebKit

private func trace(_ message: String) {
  #if DEBUG
  FileHandle.standardError.write(Data("\(message)\n".utf8))
  #endif
}

@main
enum FloatViewMac {
  static func main() throws {
    let environment = ProcessInfo.processInfo.environment
    let bundledWebRoot = Bundle.main.resourceURL?.appending(path: "web")
    let root = environment["FLOAT_VIEW_WEB_ROOT"].map(URL.init(fileURLWithPath:))
      ?? bundledWebRoot.flatMap {
        FileManager.default.fileExists(atPath: $0.path) ? $0 : nil
      }
      ?? URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
      .deletingLastPathComponent().appending(path: "web/build")
    if CommandLine.arguments.contains("--test-host") {
      let runtime = MacBackendRuntime(webRoot: root)
      let server = runtime.makeServer()
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

private final class MacAppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate,
  WKNavigationDelegate
{
  private let runtime: MacBackendRuntime
  private let frontendLifecycle: FrontendLifecycleCoordinator
  private var window: NSWindow?
  private var webView: WKWebView?

  init(webRoot: URL) {
    let runtime = MacBackendRuntime(webRoot: webRoot)
    self.runtime = runtime
    frontendLifecycle = FrontendLifecycleCoordinator {
      runtime.makeServer()
    }
  }

  func applicationDidFinishLaunching(_ notification: Notification) {
    showFrontend()
  }

  func applicationShouldHandleReopen(
    _ sender: NSApplication,
    hasVisibleWindows flag: Bool
  ) -> Bool {
    if !flag {
      showFrontend()
    }
    return true
  }

  func applicationWillTerminate(_ notification: Notification) {
    frontendLifecycle.deactivate()
    runtime.shutdown()
  }

  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    false
  }

  func windowWillClose(_ notification: Notification) {
    webView?.navigationDelegate = nil
    webView?.stopLoading()
    frontendLifecycle.deactivate()
    trace("[FloatViewState] frontend state=stopped reason=window_closed")
    DispatchQueue.main.async { [weak self] in
      self?.webView = nil
      self?.window = nil
    }
  }

  func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
    guard window != nil else { return }
    closeFrontendWindow()
    showFrontend()
  }

  private func showFrontend() {
    guard window == nil else {
      window?.makeKeyAndOrderFront(nil)
      return
    }
    do {
      let frontend = try frontendLifecycle.activate()
      let webView = WKWebView(frame: .zero)
      webView.navigationDelegate = self
      webView.load(URLRequest(url: frontend.origin))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 430, height: 820),
        styleMask: [.titled, .closable, .miniaturizable, .resizable],
        backing: .buffered,
        defer: false
      )
      window.title = "Float View"
      window.isReleasedWhenClosed = false
      window.contentView = webView
      window.delegate = self
      window.center()
      window.makeKeyAndOrderFront(nil)
      NSApplication.shared.activate(ignoringOtherApps: true)
      self.webView = webView
      self.window = window
      trace("[FloatViewState] frontend state=ready generation=\(frontend.generation)")
    } catch {
      trace("[FloatViewState] frontend state=failed")
    }
  }

  private func closeFrontendWindow() {
    window?.delegate = nil
    window?.orderOut(nil)
    webView?.navigationDelegate = nil
    webView?.stopLoading()
    webView = nil
    window = nil
    frontendLifecycle.deactivate()
  }
}

private final class MacBackendRuntime: @unchecked Sendable {
  private let bluetooth: CoreBluetoothAdapter
  private let applicationService: ApplicationService
  private let webRoot: URL

  init(webRoot: URL) {
    let relay = BluetoothEventRelay()
    let bluetooth = CoreBluetoothAdapter(
      eventHandler: { @Sendable event in relay.emit(event) },
      diagnosticHandler: { diagnostic in
        trace("[FloatViewState] adapter \(diagnostic)")
      }
    )
    let logRoot: URL
    if let override = ProcessInfo.processInfo.environment["FLOAT_VIEW_LOG_ROOT"] {
      logRoot = URL(fileURLWithPath: override, isDirectory: true)
    } else {
      let applicationSupport = try! FileManager.default.url(
        for: .applicationSupportDirectory,
        in: .userDomainMask,
        appropriateFor: nil,
        create: true
      )
      logRoot = applicationSupport
        .appending(path: "FloatView")
        .appending(path: "Sessions")
    }
    let bundledSchemaDirectory = Bundle.main.resourceURL?.appending(path: "sql")
    let schemaDirectory = bundledSchemaDirectory.flatMap {
      FileManager.default.fileExists(atPath: $0.path) ? $0 : nil
    } ?? webRoot
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
    #if DEBUG
    let recoveredSessions = try! logStore.recoverInterrupted()
    trace("[FloatViewState] recovery interruptedSessions=\(recoveredSessions)")
    #else
    try! logStore.recoverInterrupted()
    #endif
    let applicationService = ApplicationService(
      bluetooth: bluetooth,
      clock: SystemClock(),
      logStore: logStore,
      protocolCodec: VescProtocolService(),
      scheduler: SystemScheduler(),
      events: DebugEventSink()
    )
    relay.setHandler { [weak applicationService] event in
      try? applicationService?.receive(event)
    }
    self.bluetooth = bluetooth
    self.applicationService = applicationService
    self.webRoot = webRoot
  }

  func makeServer() -> LoopbackServer {
    LoopbackServer(
      webRoot: webRoot,
      platform: "macos",
      applicationService: applicationService
    )
  }

  func shutdown() {
    try? applicationService.disconnect()
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

private struct DebugEventSink: EventSink {
  func publish(_ event: any Sendable) {
    #if DEBUG
    switch event {
    case let state as BluetoothState:
      trace(
        "[FloatViewState] bluetooth radio=\(state.radio.rawValue) " +
          "scanPermission=\(state.scanPermission.rawValue) " +
          "connectPermission=\(state.connectPermission.rawValue)"
      )
    case let state as ConnectionState:
      trace("[FloatViewState] connection state=\(state.rawValue)")
    case let session as SessionRecord:
      trace("[FloatViewState] session active=\(session.active)")
    default:
      break
    }
    #endif
  }
}
