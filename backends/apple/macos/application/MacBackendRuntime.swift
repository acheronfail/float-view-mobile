import FloatViewBackend
import Foundation

func trace(_ message: String) {
  #if DEBUG
    FileHandle.standardError.write(Data("\(message)\n".utf8))
  #endif
}

final class MacBackendRuntime: @unchecked Sendable {
  private let bluetooth: any BluetoothPort
  private let service: SharedService
  private let html: Data

  init(html: Data) {
    let relay = BluetoothEventRelay()
    let bluetooth: any BluetoothPort
    #if FLOAT_VIEW_CONFORMANCE && DEBUG
      bluetooth = FixtureBluetoothAdapter(eventHandler: { @Sendable event in relay.emit(event) })
    #else
      bluetooth = CoreBluetoothAdapter(
        eventHandler: { @Sendable event in relay.emit(event) },
        diagnosticHandler: { diagnostic in
          trace("[FloatViewState] adapter \(diagnostic)")
        }
      )
    #endif
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
    let service = try! SharedService(root: logRoot, bluetooth: bluetooth, location: CoreLocationAdapter())
    relay.setHandler { [weak service] event in service?.receive(event) }
    self.bluetooth = bluetooth
    self.service = service
    self.html = html
  }

  func makeServer(
    exportPresenter: (@Sendable (NativeFilePresentation) -> Bool)? = nil,
    theme: (@Sendable (ThemePreference) -> Void)? = nil
  ) -> LoopbackServer {
    let port = ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_PORT"]
      .flatMap(UInt16.init)
      ?? PlatformLoopbackPort.macOS
    try! service.start(presenter: exportPresenter, theme: theme)
    return LoopbackServer(html: html, port: port, service: service)
  }

  func shutdown() {
    service.shutdown()
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
