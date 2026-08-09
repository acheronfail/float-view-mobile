import AppKit
import FloatViewBackend
import FloatViewCoreBridge
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
    let htmlURL = ProcessInfo.processInfo.environment["FLOAT_VIEW_FRONTEND_HTML"]
      .map(URL.init(fileURLWithPath:))
      ?? Bundle.module.url(forResource: "index", withExtension: "html", subdirectory: "Resources")
    guard let htmlURL else { throw CocoaError(.fileNoSuchFile) }

    if CommandLine.arguments.contains("--rust-core-smoke") {
      let ready = RustReadiness()
      let runtime = try FloatViewRuntimeBridge(htmlURL: htmlURL, readyHandler: ready.publish)
      guard ready.wait(timeout: 10) != nil else { throw CocoaError(.coderReadCorrupt) }
      runtime.shutdown()
      return
    }

    if CommandLine.arguments.contains("--rust-backend")
      || CommandLine.arguments.contains("--browser-host")
      || CommandLine.arguments.contains("--test-host")
    {
      let ready = RustReadiness()
      let runtime = try FloatViewRuntimeBridge(htmlURL: htmlURL, readyHandler: ready.publish)
      guard let result = ready.wait(timeout: 10) else { throw CocoaError(.coderReadCorrupt) }
      try printReadiness(result, runtimeGeneration: runtime.generation)
      withExtendedLifetime(runtime) { RunLoop.current.run() }
      return
    }

    let application = NSApplication.shared
    installApplicationIcon(on: application)
    let delegate = MacAppDelegate(htmlURL: htmlURL)
    application.delegate = delegate
    application.setActivationPolicy(.regular)
    installMainMenu(on: application)
    application.run()
  }

  private static func printReadiness(
    _ ready: RustReadiness.Value,
    runtimeGeneration: UInt64
  ) throws {
    let result: [String: Any] = [
      "abiVersion": FloatViewCoreRuntime.abiVersion,
      "buildIdentity": "float-view-abi04",
      "runtimeGeneration": runtimeGeneration,
      "backendVariant": "rust",
      "origin": ready.endpoint.absoluteString,
      "webSocketUrl": ready.endpoint.absoluteString
        .replacingOccurrences(of: "http://", with: "ws://") + "/api/ws",
      "sessionToken": ready.token,
    ]
    let data = try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
    print(String(decoding: data, as: UTF8.self))
    fflush(stdout)
  }

  @MainActor
  private static func installApplicationIcon(on application: NSApplication) {
    guard let iconURL = Bundle.module.url(
      forResource: "FloatView",
      withExtension: "icns",
      subdirectory: "Resources"
    ), let icon = NSImage(contentsOf: iconURL) else { return }
    application.applicationIconImage = icon
  }
}

final class MacRustRuntime: @unchecked Sendable {
  private let runtime: FloatViewAppleRuntime

  var generation: UInt64 {
    runtime.generation
  }

  init(
    htmlURL: URL,
    readyHandler: @escaping FloatViewRuntimeBridgeAdapter.RuntimeReadyHandler
  ) throws {
    let softwareTransport = ProcessInfo.processInfo.environment["FLOAT_VIEW_SOFTWARE_BACKEND"] == "1"
    let presentation = FloatViewDocuments.makePresenter()
    let configuredStorageRoot = ProcessInfo.processInfo.environment["FLOAT_VIEW_STORAGE_ROOT"]
      .map { URL(fileURLWithPath: $0, isDirectory: true) }
    let host = try FloatViewRuntimeHost.standard(
      durableRoot: configuredStorageRoot,
      backgroundLogging: FloatViewBackgroundExecution.setMobileBackgroundService,
      presentation: presentation
    )
    let requestedPort = ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_PORT"]
      .flatMap(UInt16.init) ?? 0
    let storageRoot = try host.durableStorageRoot()
    runtime = try FloatViewAppleRuntime(
      requestedPort: requestedPort,
      softwareTransport: softwareTransport,
      storageRoot: storageRoot,
      frontendIndex: Data(contentsOf: htmlURL),
      bluetoothFactory: { handler in
        FloatViewBluetooth.make(
          softwareTransport: softwareTransport,
          eventHandler: handler,
          diagnosticHandler: { trace("[FloatViewCore] adapter \($0)") }
        )
      },
      location: FloatViewLocation.make(),
      platformHost: host,
      retainedPresentation: presentation,
      backgroundEligibility: { FloatViewBackgroundExecution.effectiveEligibility },
      diagnosticHandler: { stage, code in
        trace("[FloatViewCore] stage=\(stage) code=\(code)")
      },
      runtimeReadyHandler: readyHandler
    )
    runtime.updateLifecycle(foreground: true)
  }

  func updateLifecycle(foreground: Bool) {
    runtime.updateLifecycle(foreground: foreground)
  }

  func shutdown() {
    runtime.shutdown()
  }
}

final class MacAppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate,
  WKNavigationDelegate
{
  private let htmlURL: URL
  private var runtime: MacRustRuntime?
  private var endpoint: URL?
  private var window: NSWindow?
  private var webView: WKWebView?
  private var presentationGeneration = 0

  init(htmlURL: URL) {
    self.htmlURL = htmlURL
  }

  func applicationDidFinishLaunching(_: Notification) {
    do {
      runtime = try FloatViewRuntimeBridge(htmlURL: htmlURL) { [weak self] endpoint, token in
        guard let launchURL = authenticatedLaunchURL(endpoint: endpoint, token: token) else { return }
        DispatchQueue.main.async {
          self?.endpoint = launchURL
          self?.showFrontend()
        }
      }
    } catch {
      trace("[FloatViewCore] startup_failed stage=runtime code=unavailable")
    }
  }

  func applicationShouldHandleReopen(_: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
    if !flag { showFrontend() }
    return true
  }

  func applicationWillTerminate(_: Notification) {
    runtime?.shutdown()
  }

  func applicationShouldTerminateAfterLastWindowClosed(_: NSApplication) -> Bool {
    false
  }

  func applicationDidBecomeActive(_: Notification) {
    runtime?.updateLifecycle(foreground: true)
  }

  func applicationDidResignActive(_: Notification) {
    runtime?.updateLifecycle(foreground: false)
  }

  func windowWillClose(_: Notification) {
    discardFrontend()
  }

  func webViewWebContentProcessDidTerminate(_: WKWebView) {
    discardFrontend()
    showFrontend()
  }

  private func showFrontend() {
    guard let endpoint else { return }
    guard window == nil else {
      window?.makeKeyAndOrderFront(nil)
      return
    }
    presentationGeneration += 1
    let webView = WKWebView(frame: .zero)
    webView.navigationDelegate = self
    webView.allowsMagnification = false
    webView.load(URLRequest(url: endpoint))
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
    trace("[FloatViewState] frontend state=ready generation=\(presentationGeneration)")
  }

  private func discardFrontend() {
    let discardedWindow = window
    window = nil
    discardedWindow?.delegate = nil
    webView?.navigationDelegate = nil
    webView?.stopLoading()
    webView = nil
    discardedWindow?.close()
    trace("[FloatViewState] frontend state=stopped reason=window_closed")
  }
}

private final class RustReadiness: @unchecked Sendable {
  typealias Value = (endpoint: URL, token: String)
  private let lock = NSLock()
  private let available = DispatchSemaphore(value: 0)
  private var value: Value?

  func publish(_ endpoint: URL, _ token: String) {
    lock.withLock { value = (endpoint, token) }
    available.signal()
  }

  func wait(timeout: TimeInterval) -> Value? {
    guard available.wait(timeout: .now() + timeout) == .success else { return nil }
    return lock.withLock { value }
  }
}

private func authenticatedLaunchURL(endpoint: URL, token: String) -> URL? {
  guard var components = URLComponents(url: endpoint, resolvingAgainstBaseURL: false) else {
    return nil
  }
  var items = components.queryItems ?? []
  items.removeAll { $0.name == "token" }
  items.append(URLQueryItem(name: "token", value: token))
  components.queryItems = items
  return components.url
}

@MainActor
private func installMainMenu(on application: NSApplication) {
  let applicationName = "Float View"
  let mainMenu = NSMenu()
  let applicationMenuItem = NSMenuItem(title: applicationName, action: nil, keyEquivalent: "")
  let applicationMenu = NSMenu(title: applicationName)
  applicationMenu.addItem(
    withTitle: "About \(applicationName)",
    action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)),
    keyEquivalent: ""
  )
  applicationMenu.addItem(.separator())
  let servicesMenu = NSMenu(title: "Services")
  let servicesMenuItem = NSMenuItem(title: "Services", action: nil, keyEquivalent: "")
  servicesMenuItem.submenu = servicesMenu
  applicationMenu.addItem(servicesMenuItem)
  application.servicesMenu = servicesMenu
  applicationMenu.addItem(.separator())
  applicationMenu.addItem(
    withTitle: "Hide \(applicationName)", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h"
  )
  let hideOthers = applicationMenu.addItem(
    withTitle: "Hide Others", action: #selector(NSApplication.hideOtherApplications(_:)), keyEquivalent: "h"
  )
  hideOthers.keyEquivalentModifierMask = [.command, .option]
  applicationMenu.addItem(
    withTitle: "Show All", action: #selector(NSApplication.unhideAllApplications(_:)), keyEquivalent: ""
  )
  applicationMenu.addItem(.separator())
  applicationMenu.addItem(
    withTitle: "Quit \(applicationName)", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"
  )
  applicationMenuItem.submenu = applicationMenu
  mainMenu.addItem(applicationMenuItem)
  application.mainMenu = mainMenu
}
