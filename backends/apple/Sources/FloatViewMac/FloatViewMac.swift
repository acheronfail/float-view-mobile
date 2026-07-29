import AppKit
import FloatViewBackend
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
    let htmlURL = environment["FLOAT_VIEW_FRONTEND_HTML"].map(URL.init(fileURLWithPath:))
      ?? Bundle.module.url(forResource: "index", withExtension: "html", subdirectory: "Resources")
    guard let htmlURL else { throw CocoaError(.fileNoSuchFile) }
    let html = try Data(contentsOf: htmlURL)
    if CommandLine.arguments.contains("--browser-host") {
      let application = NSApplication.shared
      application.setActivationPolicy(.accessory)
      let runtime = MacBackendRuntime(html: html)
      let server = runtime.makeServer(exportPresenter: MacExportPresenter.schedule)
      try server.start()
      try printReadiness(server: server, runtime: runtime)
      application.run()
      server.stop()
      runtime.shutdown()
      return
    }
    if CommandLine.arguments.contains("--test-host") {
      let runtime = MacBackendRuntime(html: html)
      let server = runtime.makeServer()
      try server.start()
      try printReadiness(server: server, runtime: runtime)
      RunLoop.current.run()
      return
    }

    let application = NSApplication.shared
    let delegate = MacAppDelegate(html: html)
    application.delegate = delegate
    application.setActivationPolicy(.regular)
    installMainMenu(on: application)
    application.run()
  }

  private static func printReadiness(
    server: LoopbackServer,
    runtime: MacBackendRuntime
  ) throws {
    let readiness: [String: Any] = [
      "webSocketUrl": "ws://127.0.0.1:\(server.port)/api/ws",
      "sessionToken": server.token,
      "origin": server.origin,
      "savedDeviceId": runtime.testSavedDeviceID ?? NSNull(),
    ]
    let data = try JSONSerialization.data(withJSONObject: readiness)
    print(String(decoding: data, as: UTF8.self))
    fflush(stdout)
  }
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
    withTitle: "Hide \(applicationName)",
    action: #selector(NSApplication.hide(_:)),
    keyEquivalent: "h"
  )
  let hideOthersItem = applicationMenu.addItem(
    withTitle: "Hide Others",
    action: #selector(NSApplication.hideOtherApplications(_:)),
    keyEquivalent: "h"
  )
  hideOthersItem.keyEquivalentModifierMask = [.command, .option]
  applicationMenu.addItem(
    withTitle: "Show All",
    action: #selector(NSApplication.unhideAllApplications(_:)),
    keyEquivalent: ""
  )

  applicationMenu.addItem(.separator())
  applicationMenu.addItem(
    withTitle: "Quit \(applicationName)",
    action: #selector(NSApplication.terminate(_:)),
    keyEquivalent: "q"
  )

  applicationMenuItem.submenu = applicationMenu
  mainMenu.addItem(applicationMenuItem)
  application.mainMenu = mainMenu
}

private final class MacAppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate,
  WKNavigationDelegate
{
  private let runtime: MacBackendRuntime
  private var server: LoopbackServer?
  private var generation = 0
  private var window: NSWindow?
  private var webView: WKWebView?

  init(html: Data) {
    let runtime = MacBackendRuntime(html: html)
    self.runtime = runtime
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
    server?.stop()
    runtime.shutdown()
  }

  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    false
  }

  func windowWillClose(_ notification: Notification) {
    webView?.navigationDelegate = nil
    webView?.stopLoading()
    server?.stop()
    server = nil
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
      let server = runtime.makeServer(exportPresenter: MacExportPresenter.schedule)
      try server.start()
      self.server = server
      generation += 1
      let webView = WKWebView(frame: .zero)
      webView.navigationDelegate = self
      webView.load(URLRequest(url: URL(string: server.origin)!))
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
      trace("[FloatViewState] frontend state=ready generation=\(generation)")
    } catch {
      let message = loopbackStartupFailure(
        port: PlatformLoopbackPort.macOS,
        error: error
      )
      trace("[FloatViewState] frontend state=failed \(message)")
    }
  }

  private func closeFrontendWindow() {
    window?.delegate = nil
    window?.orderOut(nil)
    webView?.navigationDelegate = nil
    webView?.stopLoading()
    webView = nil
    window = nil
    server?.stop()
    server = nil
  }
}

private enum MacExportPresenter {
  static func schedule(_ value: SessionExportRecord) -> Bool {
    DispatchQueue.main.async {
      let panel = NSSavePanel()
      panel.nameFieldStringValue = value.fileName
      panel.canCreateDirectories = true
      panel.begin { response in
        guard response == .OK, let destination = panel.url else { return }
        do {
          if FileManager.default.fileExists(atPath: destination.path) {
            try FileManager.default.removeItem(at: destination)
          }
          try FileManager.default.copyItem(at: value.fileURL, to: destination)
        } catch {
          trace("[FloatViewState] export presentation failed")
        }
      }
    }
    return true
  }
}

private final class MacBackendRuntime: @unchecked Sendable {
  private let bluetooth: any BluetoothPort
  private let applicationService: ApplicationService
  private let html: Data
  private let events: EventHub
  private let eventSubscription: EventSubscription
  let testSavedDeviceID: String?

  init(html: Data) {
    let relay = BluetoothEventRelay()
    let bluetooth: any BluetoothPort =
      ProcessInfo.processInfo.environment["FLOAT_VIEW_SOFTWARE_BACKEND"] == "1"
      ? SoftwareBluetoothAdapter(eventHandler: { @Sendable event in relay.emit(event) })
      : CoreBluetoothAdapter(
        eventHandler: { @Sendable event in relay.emit(event) },
        diagnosticHandler: { diagnostic in
          trace("[FloatViewState] adapter \(diagnostic)")
        }
      )
    let logRoot: URL
    let testLogRoot = ProcessInfo.processInfo.environment["FLOAT_VIEW_LOG_ROOT"]
    if let override = testLogRoot {
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
    let bundledSchemaDirectory = Bundle.module.resourceURL!.appending(path: "sql")
    let schemaDirectory = ProcessInfo.processInfo.environment["FLOAT_VIEW_SQL_ROOT"]
      .map(URL.init(fileURLWithPath:))
      ?? bundledSchemaDirectory
    let logStore = try! SQLiteLogStore(
      root: logRoot,
      schemaDirectory: schemaDirectory,
      availableBytes: {
        if testLogRoot != nil { return Int64.max }
        return (try? logRoot.resourceValues(
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
    if ProcessInfo.processInfo.environment["FLOAT_VIEW_TEST_SEED_PROFILE"] == "1" {
      let seed = try! logStore.create(
        deviceID: DeviceID("conformance-profile-fixture"),
        at: Date(timeIntervalSince1970: 0)
      )
      var seedSamples: [TelemetrySample] = []
      for index in 0..<60 {
        seedSamples.append(
          TelemetrySample(
            sequence: Int64(index + 1),
            observedAt: Date(timeIntervalSince1970: Double(index)),
            elapsedMs: Int64(index * 1_000),
            rideState: index < 5 ? .idle : .riding,
            packVoltageV: 60 - Double(index) / 20,
            dutyCycle: Double(index) / 100,
            controllerTempC: 30 + Double(index) / 10,
            motorTempC: 28 + Double(index) / 10,
            footpadState: index < 5 ? 0 : 3,
            extensionFields: [
              "motorCurrentA": Double(index),
              "batteryCurrentA": Double(index) / 2,
              "erpm": Double(index * 100),
              "speedKph": Double(index) / 3,
            ]
          )
        )
      }
      try! logStore.append(jobID: seed.jobID, samples: seedSamples)
      _ = try! logStore.close(
        jobID: seed.jobID,
        at: Date(timeIntervalSince1970: 60),
        reason: "test_seed"
      )
      testSavedDeviceID = try! logStore.savedDevices().first!.savedDeviceID
    } else {
      testSavedDeviceID = nil
    }
    let settings = try! logStore.settings()
    let events = EventHub()
    let debugEvents = DebugEventSink()
    let applicationService = ApplicationService(
      bluetooth: bluetooth,
      clock: SystemClock(),
      logStore: logStore,
      productStore: logStore,
      sessionStore: logStore,
      protocolCodec: VescProtocolService(),
      scheduler: SystemScheduler(),
      events: events,
      samplingSettings: SamplingSettings(
        idleProbeIntervalMs: settings.idleProbeIntervalMs,
        idlePersistIntervalMs: settings.idlePersistIntervalMs,
        ridingPersistIntervalMs: settings.ridingPersistIntervalMs,
        postRideTailMs: settings.postRideTailMs,
        reconnectDurationMs: settings.reconnectDurationMs
      )
    )
    relay.setHandler { [weak applicationService] event in
      try? applicationService?.receive(event)
    }
    self.bluetooth = bluetooth
    self.applicationService = applicationService
    self.html = html
    self.events = events
    eventSubscription = events.subscribe { debugEvents.publish($0) }
  }

  func makeServer(
    exportPresenter: (@Sendable (SessionExportRecord) -> Bool)? = nil
  ) -> LoopbackServer {
    LoopbackServer(
      html: html,
      port: PlatformLoopbackPort.macOS,
      applicationService: applicationService,
      exportPresenter: exportPresenter,
      events: events
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
